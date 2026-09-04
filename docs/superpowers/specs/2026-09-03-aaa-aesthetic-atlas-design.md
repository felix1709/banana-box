# AAA-Aesthetic-Atlas 与 Banana Box 集成设计

## 1. 目标

建立一个本地审美知识库 `AAA-Aesthetic-Atlas`，让 Banana Box 负责收集、识图、分类和展示，让 `AAA-prompt-felix` 通过 MCP 调用知识库参与提示词构建。三者可以独立运行，也可以联动运行。

## 2. 已确认决策

- 知识库根目录默认：`C:\Users\<用户名>\Documents\AAA-Aesthetic-Atlas`
- 数据格式：`Markdown + YAML frontmatter` + `assets/` 图片
- MCP 服务：独立 Python stdio 服务 `aaa-aesthetic-atlas-mcp`
- Banana Box 本地服务：只负责入库、识图、Chrome 插件接入和界面支撑，可以手动开关
- Chrome 插件：第一版包含右键图片入库和当前网页图片抓取
- Obsidian：第一版不依赖，不接 OB；`.md` 以后可直接被 Obsidian 打开
- 图片过大：自动压缩，不要求用户手动操作
- 识图模型：按配置顺序自动切换，一个失败换下一个
- MCP 注册：自动注册到 Codex 和 Claude Code（若检测到）

## 3. 总体架构

```text
Chrome 插件
   ↓ HTTP localhost
Banana Box 本地入库服务
   ↓ 写文件
AAA-Aesthetic-Atlas 知识库文件夹
   ↑ 读文件
aaa-aesthetic-atlas-mcp
   ↑ MCP 调用
AAA-prompt-felix
```

- Banana Box 是收集和展示工具。
- MCP 是独立查询入口。
- 知识库文件夹是唯一数据源。
- 所有文件写入采用“临时文件 + 原子替换”。

## 4. 路径发现与联动

- Banana Box 首次创建知识库后，写配置：

```text
C:\Users\<用户名>\.banana-box\atlas-config.json
```

配置文件最小结构：

```json
{
  "version": 1,
  "rootPath": "C:\\Users\\<用户名>\\Documents\\AAA-Aesthetic-Atlas",
  "createdAt": "2026-09-03"
}
```

- `AAA-prompt-felix` 启动或需要查资料时：
  - 先读 `~/.banana-box/atlas-config.json`。
  - 有配置：使用 `rootPath`。
  - 无配置：询问用户，必要时创建知识库并写回配置。
- Banana Box 不运行时，MCP 仍可读知识库。

## 5. 知识库目录结构

```text
AAA-Aesthetic-Atlas/
  README.md
  entries/
    01-scene-concept/
    02-style/
    03-character-pose/
    04-composition/
    05-lighting-atmosphere/
    06-fx-effects/
    07-color-texture/
    08-emotion-mood/
    09-camera-lens/
    10-model-constraints/
  assets/
    01-scene-concept/
    02-style/
    03-character-pose/
    04-composition/
    05-lighting-atmosphere/
    06-fx-effects/
    07-color-texture/
    08-emotion-mood/
    09-camera-lens/
    10-model-constraints/
  index/
    index.json
```

## 6. 条目文件规范

图片和知识卡片同名配对：

```text
assets/05-lighting-atmosphere/neon-backlight-01.jpg
entries/05-lighting-atmosphere/neon-backlight-01.md
```

`.md` 示例：

```markdown
---
id: neon-backlight-01
title: 赛博朋克霓虹逆光
dimension: lighting-atmosphere
tags: [霓虹, 逆光, 冷暖对比]
aliases: [霓虹逆光, cyberpunk backlight]
positive_keywords: [霓虹光, 轮廓光, 冷暖对比]
avoid: [过曝, 塑料皮肤]
models: [MJ, FLUX]
score: 0.92
status: confirmed
source_url: ""
created_at: 2026-09-03
updated_at: 2026-09-03
---

## 定义

以霓虹光作为主要色彩来源，主体使用逆光轮廓，冷色为主、暖色点缀。

## 分析

（识图模型输出）

## 提示词片段

（可直接复用的提示词内容）

## 关联标签

（同维度或其他维度关联词条）
```

`status` 取值：

- `confirmed`：已人工确认。
- `pending_review`：低置信度或识图失败，待人工确认。

## 7. 索引规范

`index/index.json` 只用于快速浏览和搜索，不是数据源头：

```json
{
  "version": 1,
  "generated_at": "2026-09-03T12:00:00Z",
  "entries": [
    {
      "id": "neon-backlight-01",
      "file": "entries/05-lighting-atmosphere/neon-backlight-01.md",
      "image": "assets/05-lighting-atmosphere/neon-backlight-01.jpg",
      "dimension": "lighting-atmosphere",
      "tags": ["霓虹", "逆光", "冷暖对比"],
      "score": 0.92,
      "status": "confirmed"
    }
  ]
}
```

索引可随时根据 `.md` 文件重建。

## 8. Banana Box 参考库模块

新增侧边栏工具：`参考库`。

页面结构：

- 顶部：服务开关、打开知识库文件夹、手动入库。
- 左侧：10 个分类、标签筛选、搜索框。
- 中间：图片网格预览。
- 右侧：大图预览 + 各维度提示词分析。

拖图入库流程：

1. 用户把图片拖到 Banana Box 悬浮窗。
2. 选择“存入审美参考库”。
3. Banana Box 本地服务复制图片到 `assets/`。
4. 自动压缩图片到适合识图和入库的规格。
5. 按配置顺序调用识图模型。
6. 生成结构化分析结果。
7. 写入 `.md + YAML`。
8. 更新 `index/index.json`。
9. 低置信度条目标记为 `pending_review`。

## 9. 本地入库服务

- 由 Banana Box 管理，可手动开启/关闭。
- 只监听 `127.0.0.1`。
- 使用本地 token，防止普通网页乱调用。
- 端口冲突时自动换可用端口并写回配置。

核心接口：

```text
POST /v1/ingest
GET  /v1/health
GET  /v1/categories
GET  /v1/entries
```

`POST /v1/ingest` 请求最小结构：

```json
{
  "sourceUrl": "",
  "localPath": "",
  "title": "",
  "dimension": "",
  "tags": []
}
```

## 10. Chrome 插件

- 右键图片出现“分析入库”。
- 点击插件按钮抓取当前网页图片。
- 显示分析摘要，用户可修改标签。
- 确认后调用 Banana Box 本地服务入库。

## 11. MCP 服务

服务名：`aaa-aesthetic-atlas-mcp`

第一版工具：

- `list_tags`：按维度或关键词列出标签。
- `match_tags`：根据 QuerySpec 返回候选参考卡片和置信度。
- `get_entry`：读取完整知识条目。
- `feedback`：记录参考卡片使用结果。

MCP 第一版只读知识条目；`feedback` 只追加使用反馈，不修改条目正文。条目写入主要由 Banana Box 完成，未来可开放受限写入工具。

## 12. 识图模型自动切换

- 读取 Banana Box 已配置的识图 provider/model 列表。
- 按顺序尝试。
- 失败、超时、限流时自动切换下一个。
- 记录成功模型。
- 全部失败则保存图片，条目标记为 `pending_review`。

## 13. 文件安全

- 所有写文件操作使用临时文件，成功后再原子替换。
- 删除或更新索引时，不让其他进程读到半成品。
- 索引与条目不一致时，以条目 `.md` 为准重建索引。
- 重复图片通过文件指纹检测。

## 14. 测试范围

- YAML 读写、索引重建、原子写入。
- MCP 工具：`list_tags`、`match_tags`、`get_entry`、`feedback`。
- Banana Box 页面：分类、搜索、图片预览、服务开关。
- 拖图入库：自动压缩、模型切换、状态标记。
- Chrome 插件：发送图片、接收摘要、确认入库。
- 端到端：建库 → 拖图 → 入库 → 检索 → MCP 查询。

## 15. 发布范围

- Banana Box 按 `fabu.MD` 升级版本、构建并发布安装包。
- 自动注册 MCP 到 Codex 配置。
- 检测并注册 MCP 到 Claude Code 配置。
- Chrome 插件单独打包并说明安装方式。
- 不提交私钥、密码或 API Key。

## 16. 第一版成功标准

- 只安装 Banana Box：能建库、拖图、识图、分类、浏览和检索。
- 只安装 `AAA-prompt-felix`：能通过 MCP 查询已有知识库。
- 两者都安装：Banana Box 入库展示，`AAA-prompt-felix` 查询并用于提示词构建。
- Chrome 插件能把图片送入 Banana Box 本地服务并完成入库。
- 两个进程并发读写时不会产生损坏文件。
