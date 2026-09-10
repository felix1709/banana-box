# PI-Web 外部依赖化（安装包瘦身）实现计划

- 日期：2026-09-10
- 状态：**阶段 1 已实施**（纯新增能力：不改打包、不删除任何文件）；**阶段 2**（摘掉内置包、改打包配置）待确认
- 影响范围：仅 PI-Web 模块、打包配置与准备脚本；不动其他业务模块

## 目标

把 PI-Web 从「随安装包内置」改为「安装包只带运行环境，PI-Web 本体按需从 npm 下载」，安装包从约 303MB 降到约 40-50MB，同时保留一键配置、一键更新、一键清理。

## 实测数据（本轮已核实）

| 项目 | 实测值 |
| --- | --- |
| 当前 NSIS 安装包 `banana-box_0.11.0_x64-setup.exe` | 302.8 MB |
| 其中 `resources/pi-web-runtime.zip` | 314.3 MB |
| 该 zip 解压后 | 1053.7 MB / 113787 个文件 |
| 内置 `node.exe`（保留） | 87.2 MB |
| 内置 npm（保留，用于下载） | 约 11 MB |
| 预计改造后安装包 | 约 40-50 MB（待实测） |

- 腾讯镜像 `https://mirrors.cloud.tencent.com/npm/` 实测可用；`@agegr/pi-web` 最新版是 **0.9.0**，当前内置的是 **0.7.16**，所以「更新」功能一上线就有真实目标。
- `@agegr/pi-web@0.9.0` 要求 `node >= 22.19.0`；内置 node 是 **v24.14.1**，满足。
- 依赖里的原生模块（`node-pty`、`sharp`、`@next/swc` 等）**全部自带预编译二进制**：实测 `node-pty@1.1.0` 2 秒装完，**不需要 Visual Studio / node-gyp 编译**。

## 已确认决策（来自你的回复）

1. 保留 Node：安装包继续内置 `node.exe` 与 npm，用户不需要自己配环境。
2. 下载源用 npm，默认走腾讯镜像；**不修改用户全局 npm 配置**，每次安装用 `--registry` 参数传入。
3. 自动配置：复用你已在「设置 → API设置」填好的 Key（`反推图片` 优先，取不到再用 `故事板`），配默认地址 `https://ai.leihuo.netease.com/v1` 与默认模型 `deepseek-v4-flash`，自动写入 `~\.pi\agent\`，用户不再手填。
4. 只保留一份、始终最新；检测到已有可用 PI-Web 就直接启动它，日常负责更新。
5. 进度用「分阶段进度 + 实时日志 + 已下载体积近似值」，不追求精确百分比（npm 不提供）。

## 与原始需求文档不一致的 3 处（请最终确认）

1. 「旧版内置遗留」的更新策略：文档要求禁用更新并提示「请手动迁移」；你的回复是「内置版本必须可以更新」。本计划按**可以更新**实现，状态保留但不再屏蔽更新。文档自查清单里「对旧内置遗留实例屏蔽更新」一条不再适用。
2. 「副本直接删除」：文档要求「禁止删除、禁止覆盖」；你的回复是「副本直接删除」。本计划按删除实现，删除清单见下，且**只在新版本部署并校验成功之后才删**。
3. 进度条精确百分比：文档要求显示百分比；受 npm 限制，改为分阶段进度 + 实时日志。

## 目录布局

- 运行环境（随安装包，只读）：`<安装目录>\resources\pi-web\` 下的 `node\node.exe` 与 `npm\`
- PI-Web 本体（唯一一份，可更新）：`<数据目录>\pi-web-runtime\current\` 下的 `package.json` 与 `node_modules\@agegr\pi-web\`
- 用户配置（永不删除）：`~\.pi\agent\` 下的 `settings.json`、`models.json`、`auth.json`
- 下载与解压临时区：`<数据目录>\pi-web-runtime\.staging\`，成功后整体替换 `current`

其中数据目录 = `%APPDATA%\com.bananabox.app`

## 删除清单（精确）

会自动删除，前提是新版「下载 + 校验 + 切换」全部成功：

1. `<数据目录>\pi-web-runtime\` 下除 `current` 以外的所有目录（旧版本目录、`.staging`、`*.tmp`）
2. `<安装目录>\resources\pi-web-runtime.zip`（旧版内置包）。你机器上对应 `C:\Users\admin\AppData\Local\banana-box\resources\pi-web-runtime.zip`，314.3MB

永不删除：

- `~\.pi\agent\` 下任何文件
- 用户手动指定的外部 PI-Web 目录
- 系统 npm 缓存
- 任何一步失败时，什么都不删

## 后端改动（主要在 `src-tauri/src/pi_web.rs`）

1. 运行环境解析：优先内置 `resources\pi-web\node\node.exe` 与内置 npm，其次退回系统 `node` / `npm`；都没有则报「缺少运行环境」并给下载链接（复用现有 `missing_dependency`）。
2. 来源检测（异步，不阻塞主程序启动）：`current` 就绪则「已安装」；存在旧的内置解压目录则标记「内置遗留，可更新」；都没有则「未安装」。
3. 一键下载部署：用内置 node 执行 npm 安装 `@agegr/pi-web` 到临时区，分阶段发进度事件（准备 / 下载 / 解压 / 校验）并回传 npm 日志尾部；开始前做磁盘空间预检（约需 1.2GB）；失败写本地日志。
4. 校验：安装后确认 `bin\pi-web.js` 存在，并用内置 node 起服务探测端口，通过后才切换 `current`。
5. 启动 / 停止 / 报错检查：沿用现有 `start_pi_web`、`stop_pi_web`、`get_pi_web_chat_health`，只把「用哪个 node、哪个脚本」改为解析结果。
6. 自动写配置：调用现有 `repair_pi_agent_config_at`，Key 取自 provider（`reverse-image` 优先，兜底 `storyboard`）。
7. 更新检查：查询 npm 上的最新版本，与 `current` 内的版本比对，返回「已是最新」或「可更新到 X」。
8. 清理：按上面的删除清单执行，只在明确动作（更新成功或用户点清理）时触发。

## 前端改动

1. `src/lib/piWebIpc.ts`：新增状态、下载、更新、清理、选择目录、日志相关 IPC。
2. `src/components/piweb/PiWebPage.vue`：展示 6 种状态（未安装 / 下载中 / 部署中 / 已就绪 / 可更新 / 内置遗留可更新）与当前版本号，提供一键下载、更新、清理、手动指定目录。
3. 进度弹窗复用现有 `src/components/MediaToolProgressDialog.vue`（自带进度、日志、重试）。
4. 布局适配 100% / 125% / 150% 缩放，控件不重叠不溢出。

## 打包改动

1. `scripts/prepare-pi-web-runtime.mjs`：不再生成 `pi-web-runtime.zip`，改为把当前 `node.exe` 与 npm 复制到 `src-tauri\resources\pi-web\`。
2. `src-tauri/tauri.conf.json`：`resources` 用 `resources/pi-web` 替换 `resources/pi-web-runtime.zip`。
3. `.gitignore` 已有 `src-tauri/resources/pi-web/*` 规则，无需改动。

## 验证步骤

1. `pnpm check` 全绿；`cargo test --manifest-path src-tauri\Cargo.toml` 全绿。
2. 全新环境（先手动删掉 `<数据目录>\pi-web-runtime`）：进入「Pi智能体」显示「未安装」→ 点一键下载 → 有进度与日志 → 完成后自动写入配置并可正常对话。
3. 更新：把 `current` 换回 0.7.16 再点更新，应升到 0.9.0 且磁盘上只留一份。
4. 老用户升级：先装 v0.11.0（内置包），再装新版，确认 `resources\pi-web-runtime.zip` 只在首次成功部署后才消失，且 `~\.pi\agent\` 内容完全没变。
5. 异常场景：断网、磁盘不足、包损坏，都要有提示与重试，不崩溃，不删任何东西。
6. `pnpm tauri build` 实测安装包体积并记录。

## 风险与取舍

- 首次使用需要联网下载约 1GB、占盘约 1.1GB，比现在「装完即用」多一次等待。
- 依赖镜像可用性；不可用时按「腾讯 → npmmirror → 官方」顺序回退。
- 内置 node 本身约 87MB，安装包无法回到「几 MB」级别，这是保留 Node 的代价。

## 阶段 1 实施记录（已完成）

范围：只新增能力，**没有**改 `tauri.conf.json`、**没有**改 `scripts/prepare-pi-web-runtime.mjs`、**没有**删除任何现存文件或目录。

新增文件：

- `src-tauri/src/pi_web_runtime.rs`：运行环境解析、版本检测、一键下载部署、更新、清理、手动指定目录、进度事件、安装日志。

最小改动：

- `src-tauri/src/lib.rs`：注册 `pi_web_runtime` 模块、托管 `PiWebRuntimeService`、注册 4 个命令。
- `src-tauri/src/pi_web.rs`：启动时优先使用外部安装副本；新增 `auto_repair_pi_agent_config`（只在配置不完整时才写入）。
- `src-tauri/Cargo.toml`：新增 `flate2`、`tar`（都已在 `Cargo.lock` 里，只用于解压 npm 官方包）。
- `src/lib/piWebIpc.ts`、`src/components/piweb/PiWebPage.vue`、`tests/components/PiWebPage.test.ts`：新增「PI-WEB 运行环境」卡片、进度弹窗与测试。

运行环境来源（按优先级）：

1. 打包资源 `<安装目录>\resources\pi-web\node\node.exe`（阶段 2 的布局，本仓库已存在）
2. 从内置 `pi-web-runtime.zip` 里**只解出一个** `node/node.exe`（不用解压整个 300MB）
3. 用户从安装包内置版本解压出来的 `node.exe`
4. 系统 PATH 里的 `node`

npm 来源（按优先级）：打包资源里的 npm → 系统 Node 自带的 npm → 从镜像下载 npm 官方压缩包（约 3MB）解压到数据目录。

实测（本机，2026-09-10）：

| 项目 | 实测值 |
| --- | --- |
| `npm install @agegr/pi-web@0.9.0`（腾讯镜像） | **49 秒**，362 个包，exit 0 |
| 安装后占用 | 38208 个文件 / 898.3 MB |
| 用内置 `node.exe` 启动 0.9.0 | 2.1 秒 Ready，`http://127.0.0.1:30199` 返回 **200** |
| 校验点 `bin/pi-web.js`、`.next`、`next/dist/bin/next` | 全部存在 |

验证结果：`cargo test` 237 passed / 0 failed；`pnpm check` 全绿（lint 0 error，前端 383 passed）。

阶段 2 待办（未做）：把 `resources/pi-web-runtime.zip` 换成 `resources/pi-web`（node + npm），删除内置包，实测安装包体积。
