# banana.md — Banana Box 项目规范

> 本文件是 Banana Box 项目的协作规范，给人看也给 AI 协作者看。任何改动都应遵守这里的约定。
> 设计依据见 [PLAN.md](./PLAN.md)，执行清单见 [todo.md](./todo.md)。

## 1. 项目简介

**Banana Box** 是一个轻量级桌面提示词库：常驻系统托盘，随时唤出悬浮面板，点击提示词即复制到剪贴板。支持分类整理、关键词检索、参考图预览，本地存储 + 一键导出/导入。

- 类型：Tauri 2 桌面应用
- 平台：Windows 优先（后续可扩 macOS/Linux）
- 目标：轻量（安装包 ~5MB、内存 <80MB）、即点即用、换电脑可迁移

## 2. 技术栈

| 层 | 技术 |
|----|------|
| 桌面壳 | Tauri 2（Rust） |
| 前端 | Vue 3 + TypeScript + Vite |
| 状态 | Pinia |
| UI | Naive UI（按需引入） |
| 测试 | Vitest（前端）+ Rust `#[test]`（后端） |
| 包管理 | pnpm |

## 3. 常用命令

```bash
# 安装依赖
pnpm install

# 前端开发（纯 Web，可调试界面，无 Tauri 能力）
pnpm dev

# Tauri 桌面开发（完整能力：托盘、快捷键、文件、剪贴板）
pnpm tauri dev

# 构建生产安装包
pnpm tauri build

# 检查（提交前必跑）
pnpm typecheck      # tsc 类型检查
pnpm lint           # ESLint
pnpm test           # Vitest 单元测试
pnpm check          # = typecheck + lint + test（一行全跑）

# Rust 端测试
cd src-tauri && cargo test
```

**提交前必须跑 `pnpm check` 全绿。**（对齐全局 CLAUDE.md 的"证据说话"规则）

## 4. 目录约定

```
src-tauri/src/
  main.rs          # 入口、托盘、快捷键、窗口管理
  commands.rs      # 所有 IPC 命令（前端可调用的 Rust 函数）
  library.rs       # 数据读写、版本迁移、导出/导入逻辑

src/
  components/      # Vue 组件，每个文件一个组件，单一职责
  stores/          # Pinia store：library（数据）/ ui（面板状态）
  types/           # TypeScript 类型定义（与 Rust 结构对齐）
  lib/             # 工具：ipc.ts 封装 invoke、id 生成等
  styles/          # 全局样式
```

**原则：**
- 每个文件单一职责，文件变大（>300 行）就考虑拆分
- 组件只管渲染和交互，不直接调 Rust；通过 `lib/ipc.ts` 封装层调用
- 业务逻辑放 Pinia store，便于测试

## 5. 前后端通信约定（IPC）

- 前端**不直接**调 `@tauri-apps/api` 的 invoke，统一走 `src/lib/ipc.ts` 封装的函数
- Rust 端命令集中在 `commands.rs`，每个命令用 `#[tauri::command]` 标注
- 命令命名：`snake_case`（Rust 侧）/ `camelCase`（TS 侧，Tauri 自动转换）
- 数据结构：Rust struct 与 TS interface 在 `types/index.ts` 对齐，字段名一致

约定命令清单（随实现更新）：
- `load_library()` → 读取并返回 `Library`
- `save_library(library)` → 写入 `library.json`
- `copy_to_clipboard(text)` → 写剪贴板
- `save_image(bytes, ext)` → 存图，返回相对路径
- `delete_image(path)` → 删图
- `export_library()` → 打包 zip，返回保存路径
- `import_library(zip_path)` → 解压并返回 `Library`

## 6. 数据存储约定

- 数据根目录：`%APPDATA%/banana-box/`（用 Tauri `app_data_dir` 获取，不硬编码）
- `library.json`：全部提示词、分类、设置
- `images/`：参考图文件，文件名 `{uuid}.{ext}`
- 提示词只存图片**相对路径**，不嵌 base64
- 删除提示词时同步删除其图片文件
- `library.json` 写入用"先写临时文件再原子替换"，避免写坏
- 保留 `version` 字段，未来结构变更走迁移函数

## 7. 界面与交互约定

- 悬浮面板：失焦自动隐藏；弹出位置跟随鼠标；记忆上次尺寸/位置
- 复制反馈：点击卡片后 toast "已复制 ✓"（1.5s）
- 搜索：实时过滤，匹配 title/content/tags，不区分大小写
- 危险操作（删除分类、覆盖导入）必须二次确认
- 主题默认跟随系统（auto）

## 8. 代码风格

- TypeScript：严格模式（`strict: true`），禁用 `any`，用 `unknown` + 类型守卫
- Vue：`<script setup lang="ts">` 单文件组件，Composition API
- Rust：`clippy` 无警告，格式化用 `rustfmt`
- 命名：组件 PascalCase、变量/函数 camelCase（TS）/ snake_case（Rust）、常量 UPPER_SNAKE
- 注释：写"为什么"，不写"是什么"；中文注释 OK，但代码标识符用英文

## 9. 给 AI 协作者的提示

- 改代码前先读 [PLAN.md](./PLAN.md) 了解整体设计
- 执行任务时对照 [todo.md](./todo.md)，做完一项打勾
- 任何"我做好了"之前必须跑 `pnpm check` 拿证据，对齐全局 CLAUDE.md 的"证据说话"
- 危险操作（删文件、覆盖数据）先解释后果再等用户确认，对齐"初学者保护规则"
- 不在本期范围的功能（云同步、AI 生成、富文本等）不要擅自加，见 PLAN.md 第 9 节
- 用中文与项目所有者交流，代码和命令保持英文（对齐全局 CLAUDE.md）
