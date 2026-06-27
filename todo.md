# Banana Box 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: 用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务执行。步骤用 `- [ ]` 复选框跟踪。

**Goal:** 从零搭建一个 Tauri 2 + Vue 3 + TS 的桌面提示词库，常驻托盘、点击复制、分类检索、参考图预览、导出导入。

**Architecture:** Rust 后端负责系统操作（文件/剪贴板/托盘/快捷键），Vue 3 前端负责界面与状态，通过 Tauri IPC 命令通信。数据存本地 `%APPDATA%/banana-box/`（library.json + images/）。

**Tech Stack:** Tauri 2、Vue 3 + TypeScript + Vite、Pinia、Naive UI、Vitest、Rust `#[test]`。

**环境前提:** Windows 10、Node v24、npm 11.9 已装。需补装 Rust 工具链、pnpm、MSVC Build Tools。

**全局约定:** 每完成一个 Task 跑一次 `pnpm check`（typecheck+lint+test），全绿再进下一个。提交信息用 `feat:`/`fix:`/`chore:` 前缀。

---

## Task 0: 环境准备（一次性）

**说明:** Tauri 在 Windows 需要 Rust + MSVC 构建工具。装一次即可。

- [ ] **Step 0.1: 安装 pnpm**

```bash
npm install -g pnpm
pnpm --version
```
Expected: 版本号（如 9.x）

- [ ] **Step 0.2: 安装 MSVC Build Tools**（Tauri 编译 Rust 需要）

```bash
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```
说明：这步下载较大（~2GB），是一次性投入。若已装过 VS，可跳过。
Expected: 安装完成（重启终端）

- [ ] **Step 0.3: 安装 Rust 工具链**

去 https://rustup.rs 下载 `rustup-init.exe` 运行，选默认安装。或：
```bash
winget install Rustlang.Rustup
```
装完重启终端，验证：
```bash
rustc --version
cargo --version
```
Expected: 两个版本号

- [ ] **Step 0.4: 确认 WebView2 已装**（Win10/11 一般预装）

```bash
winget list Microsoft.EdgeWebView2Runtime
```
若未装：`winget install Microsoft.EdgeWebView2Runtime`

- [ ] **Step 0.5: 初始化 git 仓库并提交设计文档**

```bash
cd c:/Users/Felix/banana-box
git init
git add PLAN.md banana.md todo.md
git commit -m "docs: add design spec, conventions and implementation plan"
```

---

## Task 1: 脚手架初始化

**Files:**
- Create: 整个项目骨架（由 create-tauri-app 生成）

- [ ] **Step 1.1: 用官方脚手架生成项目**

在 `c:/Users/Felix/` 下运行（项目名 banana-box，已存在目录则先生成到临时名再搬入；或直接在当前目录生成）：
```bash
cd c:/Users/Felix/banana-box
pnpm create tauri-app .
```
交互选项选择：
- Project name: `banana-box`
- Identifier: `com.bananabox.app`
- Frontend language: `TypeScript`
- UI template: `Vue`
- UI package manager: `pnpm`

- [ ] **Step 1.2: 安装依赖**

```bash
pnpm install
```
Expected: 依赖安装完成，无报错

- [ ] **Step 1.3: 跑通前端 dev**

```bash
pnpm dev
```
Expected: 浏览器/Vite 打开看到 Vue 欢迎页，无错误。Ctrl+C 退出。

- [ ] **Step 1.4: 跑通 Tauri 桌面 dev**（首次会编译 Rust，耗时较长）

```bash
pnpm tauri dev
```
Expected: 弹出桌面窗口显示 Vue 欢迎页。关闭窗口退出。

- [ ] **Step 1.5: 清理脚手架默认内容**

删除 `src/components/` 下脚手架自带示例组件（如 `HelloWorld.vue`、`TheHeader.vue` 等），把 `src/App.vue` 改成空白骨架：
```vue
<script setup lang="ts">
</script>

<template>
  <main class="app">
    <p> Banana Box </p>
  </main>
</template>

<style scoped>
.app { padding: 16px; font-family: system-ui, sans-serif; }
</style>
```

- [ ] **Step 1.6: 提交**

```bash
git add -A
git commit -m "chore: scaffold tauri + vue3 + ts project"
```

---

## Task 2: 安装项目依赖

**Files:**
- Modify: `package.json`

- [ ] **Step 2.1: 装运行时依赖**

```bash
pnpm add @tauri-apps/api pinia naive-ui uuid
pnpm add -D @types/uuid
```

- [ ] **Step 2.2: 装开发依赖（测试/lint）**

```bash
pnpm add -D vitest @vue/test-utils jsdom @vitest/coverage-v8
pnpm add -D eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin eslint-plugin-vue
```

- [ ] **Step 2.3: 装 Tauri 插件（Rust 端）**

```bash
pnpm tauri add clipboard-manager
pnpm tauri add dialog
pnpm tauri add fs
pnpm tauri add global-shortcut
```
说明：这些命令会自动改 `src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的 permissions。每条稍等编译。

- [ ] **Step 2.4: 配置 package.json 脚本**

编辑 `package.json` 的 `scripts`，确保含：
```json
"scripts": {
  "dev": "vite",
  "build": "vue-tsc --noEmit && vite build",
  "tauri": "tauri",
  "typecheck": "vue-tsc --noEmit",
  "test": "vitest run",
  "test:watch": "vitest",
  "lint": "eslint src --ext .ts,.vue",
  "check": "pnpm typecheck && pnpm lint && pnpm test"
}
```

- [ ] **Step 2.5: 配置 vitest**

创建 `vitest.config.ts`：
```ts
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    globals: true,
    passWithNoTests: true,
  },
})
```

- [ ] **Step 2.6: 配置 eslint（最简）**

创建 `.eslintrc.cjs`：
```js
module.exports = {
  root: true,
  parser: 'vue-eslint-parser',
  parserOptions: { parser: '@typescript-eslint/parser' },
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:vue/vue3-recommended',
  ],
  rules: { 'vue/multi-word-component-names': 'off' },
}
```

- [ ] **Step 2.7: 验证 check 跑通**

```bash
pnpm check
```
Expected: 全绿（暂无测试，typecheck/lint 通过）

- [ ] **Step 2.8: 提交**

```bash
git add -A
git commit -m "chore: add deps and tooling config"
```

---

## Task 3: TypeScript 类型定义

**Files:**
- Create: `src/types/index.ts`

- [ ] **Step 3.1: 写类型定义**

```ts
// src/types/index.ts

export interface Category {
  id: string
  name: string
  color: string
  order: number
}

export interface Prompt {
  id: string
  title: string
  content: string
  categoryId: string | null
  tags: string[]
  image: string | null  // 相对路径，如 images/abc.png
  createdAt: number
  updatedAt: number
}

export interface Settings {
  hotkey: string
  theme: 'auto' | 'light' | 'dark'
}

export interface Library {
  version: number
  categories: Category[]
  prompts: Prompt[]
  settings: Settings
}

// 用于 Tauri invoke 返回值（null 在 JSON 里安全）
export type LibraryDto = Library
```

- [ ] **Step 3.2: 类型检查**

```bash
pnpm typecheck
```
Expected: 通过

- [ ] **Step 3.3: 提交**

```bash
git add src/types/index.ts
git commit -m "feat: add core types"
```

---

## Task 4: 前端 IPC 封装层

**Files:**
- Create: `src/lib/ipc.ts`

说明：所有 Rust 调用集中在此，组件/store 只调这里的函数，便于替换和测试。

- [ ] **Step 4.1: 写 ipc 封装**

```ts
// src/lib/ipc.ts
import { invoke } from '@tauri-apps/api/core'
import type { Library } from '@/types'

export async function loadLibrary(): Promise<Library> {
  return await invoke<Library>('load_library')
}

export async function saveLibrary(library: Library): Promise<void> {
  await invoke('save_library', { library })
}

export async function copyToClipboard(text: string): Promise<void> {
  await invoke('copy_to_clipboard', { text })
}

export async function saveImage(bytes: number[], ext: string): Promise<string> {
  return await invoke<string>('save_image', { bytes, ext })
}

export async function deleteImage(path: string): Promise<void> {
  await invoke('delete_image', { path })
}

export async function exportLibrary(): Promise<void> {
  await invoke('export_library')
}

export async function importLibrary(): Promise<Library | null> {
  return await invoke<Library | null>('import_library')
}
```

- [ ] **Step 4.2: 配置 @ 路径别名**（若脚手架未配）

检查 `tsconfig.json` 与 `vite.config.ts` 是否有 `@` -> `src` 别名。若没有，在 `vite.config.ts` 加：
```ts
import path from 'path'
// 在 defineConfig 里：
resolve: { alias: { '@': path.resolve(__dirname, './src') } }
```
并在 `tsconfig.json` 的 `compilerOptions` 加：
```json
"baseUrl": ".",
"paths": { "@/*": ["src/*"] }
```

- [ ] **Step 4.3: 类型检查**

```bash
pnpm typecheck
```
Expected: 通过（Rust 命令此时还没实现，但 TS 端只声明类型，能过）

- [ ] **Step 4.4: 提交**

```bash
git add -A
git commit -m "feat: add ipc wrapper"
```

---

## Task 5: Rust 后端 — 数据读写逻辑（TDD）

**Files:**
- Create: `src-tauri/src/library.rs`
- Test: `src-tauri/src/library.rs`（同文件 `#[cfg(test)]`）

说明：纯逻辑，不依赖 Tauri runtime，便于单测。load/save 接收目录路径参数。

- [ ] **Step 5.1: 写失败测试**

在 `src-tauri/src/library.rs`：
```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub version: i32,
    pub categories: Vec<Category>,
    pub prompts: Vec<Prompt>,
    pub settings: Settings,
}

impl Default for Library {
    fn default() -> Self {
        Library {
            version: 1,
            categories: vec![],
            prompts: vec![],
            settings: Settings {
                hotkey: "Ctrl+Shift+B".to_string(),
                theme: "auto".to_string(),
            },
        }
    }
}

pub fn library_path(dir: &Path) -> PathBuf {
    dir.join("library.json")
}

pub fn load_library(dir: &Path) -> Library {
    let path = library_path(dir);
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Library::default()),
        Err(_) => Library::default(),
    }
}

pub fn save_library(dir: &Path, lib: &Library) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = library_path(dir);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(lib).expect("serialize library");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_dir() -> PathBuf {
        let mut d = env::temp_dir();
        // 用线程 id 避免并发冲突
        d.push(format!("banana-box-test-{}", std::process::id()));
        d
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = temp_dir();
        let _ = fs::remove_dir_all(&dir);
        let lib = load_library(&dir);
        assert_eq!(lib.version, 1);
        assert!(lib.prompts.is_empty());
        assert_eq!(lib.settings.hotkey, "Ctrl+Shift+B");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir();
        let _ = fs::remove_dir_all(&dir);
        let mut lib = Library::default();
        lib.categories.push(Category {
            id: "c1".into(), name: "写作".into(), color: "#f59e0b".into(), order: 0,
        });
        lib.prompts.push(Prompt {
            id: "p1".into(), title: "总结".into(), content: "总结三点".into(),
            category_id: Some("c1".into()), tags: vec!["中文".into()],
            image: None, created_at: 1, updated_at: 1,
        });
        save_library(&dir, &lib).unwrap();
        let loaded = load_library(&dir);
        assert_eq!(loaded, lib);
    }
}
```

- [ ] **Step 5.2: 跑测试验证失败/通过**

```bash
cd src-tauri && cargo test -- --nocapture; cd ..
```
Expected: 2 个测试 PASS（save/load 已实现，应一次通过；若先写测试再实现，先看到编译失败再补实现）

- [ ] **Step 5.3: 加入模块树**

在 `src-tauri/src/main.rs` 顶部加 `mod library;`（若 main.rs 用了 lib.rs 结构则加到 lib.rs）。先只加模块声明，命令在 Task 6 实现。

- [ ] **Step 5.4: 提交**

```bash
git add src-tauri/src/library.rs src-tauri/src/main.rs
git commit -m "feat(rust): library load/save with tests"
```

---

## Task 6: Rust 后端 — IPC 命令

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/Cargo.toml`（加 uuid、zip 依赖）

- [ ] **Step 6.1: 加 Rust 依赖**

编辑 `src-tauri/Cargo.toml` 的 `[dependencies]`，加：
```toml
uuid = { version = "1", features = ["v4"] }
zip = "2"
```

- [ ] **Step 6.2: 写 commands.rs**

```rust
// src-tauri/src/commands.rs
use crate::library::{self, Library};
use std::path::PathBuf;
use tauri::Manager;

fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("no app data dir")
}

#[tauri::command]
pub fn load_library(app: tauri::AppHandle) -> Library {
    library::load_library(&data_dir(&app))
}

#[tauri::command]
pub fn save_library(app: tauri::AppHandle, library: Library) -> std::io::Result<()> {
    library::save_library(&data_dir(&app), &library)
}

#[tauri::command]
pub fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_image(app: tauri::AppHandle, bytes: Vec<u8>, ext: String) -> Result<String, String> {
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let name = format!("{}.{}", id, ext);
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(format!("images/{}", name))
}

#[tauri::command]
pub fn delete_image(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let full = data_dir(&app).join(&path);
    if full.exists() {
        std::fs::remove_file(&full).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 6.3: 注册命令到 main.rs**

编辑 `src-tauri/src/main.rs`，在 `tauri::Builder` 链上加 `.invoke_handler(...)` 和插件：
```rust
mod library;
mod commands;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::load_library,
            commands::save_library,
            commands::copy_to_clipboard,
            commands::save_image,
            commands::delete_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running banana box");
}
```

- [ ] **Step 6.4: 编译验证**

```bash
cd src-tauri && cargo check; cd ..
```
Expected: 编译通过（首次会拉取 uuid/zip 依赖）

- [ ] **Step 6.5: 权限配置**

检查 `src-tauri/capabilities/default.json`，确保包含 `clipboard-manager`、`dialog`、`fs`、`global-shortcut` 的 permissions。脚手架 `pnpm tauri add` 应已自动加；若有缺失按提示补。

- [ ] **Step 6.6: 提交**

```bash
git add -A
git commit -m "feat(rust): ipc commands for library/clipboard/image"
```

---

## Task 7: 导出/导入命令（TDD）

**Files:**
- Modify: `src-tauri/src/library.rs`（加 export/import 逻辑 + 测试）
- Modify: `src-tauri/src/commands.rs`（加命令）

- [ ] **Step 7.1: 写失败测试 — 导出/导入往返**

在 `library.rs` 的 `#[cfg(test)]` 模块加：
```rust
    #[test]
    fn export_import_roundtrip() {
        let dir = temp_dir();
        let _ = fs::remove_dir_all(&dir);
        let mut lib = Library::default();
        lib.prompts.push(Prompt {
            id: "p1".into(), title: "t".into(), content: "c".into(),
            category_id: None, tags: vec![], image: Some("images/a.png".into()),
            created_at: 1, updated_at: 1,
        });
        // 模拟图片文件
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::write(dir.join("images/a.png"), b"fakepng").unwrap();
        save_library(&dir, &lib).unwrap();

        let zip_path = dir.join("export.zip");
        export_library(&dir, &zip_path).unwrap();

        // 读回 zip
        let imported = import_library(&zip_path).unwrap();
        assert_eq!(imported.prompts.len(), 1);
        assert_eq!(imported.prompts[0].image, Some("images/a.png".into()));
    }
```

- [ ] **Step 7.2: 跑测试确认失败**

```bash
cd src-tauri && cargo test export_import; cd ..
```
Expected: 编译失败（export_library/import_library 未定义）

- [ ] **Step 7.3: 实现 export/import**

在 `library.rs` 顶部加 `use std::io::{Read, Write};` 和 `use zip;`，然后加：
```rust
pub fn export_library(data_dir: &Path, zip_path: &Path) -> std::io::Result<()> {
    let file = fs::File::create(zip_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();

    // library.json
    let json = serde_json::to_string_pretty(&load_library(data_dir))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writer.start_file("library.json", opts)?;
    writer.write_all(json.as_bytes())?;

    // images/
    let images_dir = data_dir.join("images");
    if images_dir.exists() {
        for entry in fs::read_dir(&images_dir)? {
            let entry = entry?;
            let rel = format!("images/{}", entry.file_name().to_string_lossy());
            writer.start_file(&rel, opts)?;
            let mut f = fs::File::open(entry.path())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            writer.write_all(&buf)?;
        }
    }
    writer.finish()?;
    Ok(())
}

pub fn import_library(zip_path: &Path) -> std::io::Result<Library> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut json_str: Option<String> = None;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let name = file.name().to_string();
        if name == "library.json" {
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            json_str = Some(s);
        }
    }
    let json = json_str.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no library.json in zip"))?;
    let lib: Library = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(lib)
}
```
注意：`import_library` 这里只解析 library.json 返回数据；图片解包在命令层按需做（合并/覆盖策略在前端处理）。

- [ ] **Step 7.4: 跑测试通过**

```bash
cd src-tauri && cargo test; cd ..
```
Expected: 全部 PASS

- [ ] **Step 7.5: 加导出/导入命令**

在 `commands.rs` 加：
```rust
#[tauri::command]
pub fn export_library(app: tauri::AppHandle, dest: String) -> Result<(), String> {
    let zip_path = std::path::PathBuf::from(&dest);
    library::export_library(&data_dir(&app), &zip_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_library(zip_path: String) -> Result<Library, String> {
    library::import_library(std::path::Path::new(&zip_path)).map_err(|e| e.to_string())
}
```
说明：Tauri 2 的 dialog 是异步的，前端用 `@tauri-apps/plugin-dialog` 的 `save()`/`open()` 拿路径，再调上面的命令。把 `export_library` 和 `import_library` 注册进 `main.rs` 的 `generate_handler!`。

更新 `src/lib/ipc.ts` 的 `exportLibrary`/`importLibrary`（安装前端 dialog 插件：`pnpm add @tauri-apps/plugin-dialog`）：
```ts
import { save, open } from '@tauri-apps/plugin-dialog'

export async function exportLibrary(): Promise<void> {
  const dest = await save({
    defaultPath: `banana-box-export-${new Date().toISOString().slice(0,10).replace(/-/g,'')}.zip`,
    filters: [{ name: 'zip', extensions: ['zip'] }],
  })
  if (!dest) return
  await invoke('export_library', { dest })
}

export async function importLibrary(): Promise<Library | null> {
  const picked = await open({
    filters: [{ name: 'zip', extensions: ['zip'] }],
    multiple: false,
  })
  if (!picked || Array.isArray(picked)) return null
  return await invoke<Library>('import_library', { zipPath: picked })
}
```

- [ ] **Step 7.6: 编译验证**

```bash
cd src-tauri && cargo check; cd ..
pnpm typecheck
```
Expected: 双侧通过

- [ ] **Step 7.7: 提交**

```bash
git add -A
git commit -m "feat(rust): export/import library zip"
```

---

## Task 8: Pinia library store（TDD）

**Files:**
- Create: `src/stores/library.ts`
- Test: `tests/stores/library.test.ts`

说明：纯逻辑 store，IPC 调用抽到 ipc.ts，store 内可测的方法不依赖 Tauri。

- [ ] **Step 8.1: 写失败测试 — 增删改 + 搜索**

```ts
// tests/stores/library.test.ts
import { setActivePinia, createPinia } from 'pinia'
import { describe, it, expect, beforeEach } from 'vitest'
import { useLibraryStore } from '@/stores/library'
import type { Library } from '@/types'

function seed(): Library {
  return {
    version: 1,
    categories: [
      { id: 'c1', name: '写作', color: '#f59e0b', order: 0 },
      { id: 'c2', name: '翻译', color: '#3b82f6', order: 1 },
    ],
    prompts: [
      { id: 'p1', title: '总结助手', content: '请总结三点', categoryId: 'c1', tags: ['中文'], image: null, createdAt: 1, updatedAt: 1 },
      { id: 'p2', title: '润色', content: 'polish this', categoryId: 'c2', tags: ['en'], image: null, createdAt: 2, updatedAt: 2 },
    ],
    settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
  }
}

describe('library store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const s = useLibraryStore()
    s.hydrate(seed())
  })

  it('filters by search term across title/content/tags', () => {
    const s = useLibraryStore()
    s.search = '总结'
    expect(s.filteredPrompts.map(p => p.id)).toEqual(['p1'])
    s.search = 'en'
    expect(s.filteredPrompts.map(p => p.id)).toEqual(['p2'])
    s.search = ''
    expect(s.filteredPrompts.length).toBe(2)
  })

  it('filters by current category', () => {
    const s = useLibraryStore()
    s.currentCategoryId = 'c1'
    expect(s.filteredPrompts.map(p => p.id)).toEqual(['p1'])
  })

  it('addPrompt appends with uuid and timestamps', () => {
    const s = useLibraryStore()
    const before = s.library.prompts.length
    s.addPrompt({ title: '新', content: 'x', categoryId: null, tags: [], image: null })
    expect(s.library.prompts.length).toBe(before + 1)
    expect(s.library.prompts[before].id.length).toBeGreaterThan(8)
    expect(s.library.prompts[before].createdAt).toBeGreaterThan(0)
  })

  it('deletePrompt removes prompt', () => {
    const s = useLibraryStore()
    s.deletePrompt('p1')
    expect(s.library.prompts.find(p => p.id === 'p1')).toBeUndefined()
  })

  it('updatePrompt mutates and bumps updatedAt', () => {
    const s = useLibraryStore()
    s.updatePrompt('p1', { title: '改后' })
    expect(s.library.prompts[0].title).toBe('改后')
    expect(s.library.prompts[0].updatedAt).toBeGreaterThanOrEqual(1)
  })

  it('deleteCategory nulls categoryId of its prompts', () => {
    const s = useLibraryStore()
    s.deleteCategory('c1')
    expect(s.library.categories.find(c => c.id === 'c1')).toBeUndefined()
    expect(s.library.prompts[0].categoryId).toBeNull()
  })
})
```

- [ ] **Step 8.2: 跑测试确认失败**

```bash
pnpm test
```
Expected: FAIL（store 未定义）

- [ ] **Step 8.3: 实现 store**

```ts
// src/stores/library.ts
import { defineStore } from 'pinia'
import { v4 as uuid } from 'uuid'
import type { Library, Prompt, Category } from '@/types'
import * as ipc from '@/lib/ipc'

const emptyLibrary = (): Library => ({
  version: 1,
  categories: [],
  prompts: [],
  settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
})

export const useLibraryStore = defineStore('library', {
  state: () => ({
    library: emptyLibrary() as Library,
    search: '' as string,
    currentCategoryId: null as string | null, // null = 全部
    loaded: false as boolean,
  }),
  getters: {
    filteredPrompts(state): Prompt[] {
      const kw = state.search.trim().toLowerCase()
      return state.library.prompts.filter(p => {
        if (state.currentCategoryId && p.categoryId !== state.currentCategoryId) return false
        if (!kw) return true
        return (
          p.title.toLowerCase().includes(kw) ||
          p.content.toLowerCase().includes(kw) ||
          p.tags.some(t => t.toLowerCase().includes(kw))
        )
      })
    },
    categories(state): Category[] {
      return [...state.library.categories].sort((a, b) => a.order - b.order)
    },
  },
  actions: {
    hydrate(lib: Library) {
      this.library = lib
      this.loaded = true
    },
    async load() {
      this.library = await ipc.loadLibrary()
      this.loaded = true
    },
    async persist() {
      await ipc.saveLibrary(this.library)
    },
    addPrompt(input: Omit<Prompt, 'id' | 'createdAt' | 'updatedAt'>) {
      const now = Math.floor(Date.now() / 1000)
      const p: Prompt = { ...input, id: uuid(), createdAt: now, updatedAt: now }
      this.library.prompts.push(p)
      this.persist()
    },
    updatePrompt(id: string, patch: Partial<Prompt>) {
      const p = this.library.prompts.find(x => x.id === id)
      if (!p) return
      Object.assign(p, patch, { updatedAt: Math.floor(Date.now() / 1000) })
      this.persist()
    },
    async deletePrompt(id: string) {
      const p = this.library.prompts.find(x => x.id === id)
      if (p?.image) await ipc.deleteImage(p.image)
      this.library.prompts = this.library.prompts.filter(x => x.id !== id)
      this.persist()
    },
    addCategory(name: string, color = '#888') {
      const order = this.library.categories.length
      this.library.categories.push({ id: uuid(), name, color, order })
      this.persist()
    },
    deleteCategory(id: string) {
      this.library.categories = this.library.categories.filter(c => c.id !== id)
      for (const p of this.library.prompts) {
        if (p.categoryId === id) p.categoryId = null
      }
      this.persist()
    },
    async copyPrompt(id: string) {
      const p = this.library.prompts.find(x => x.id === id)
      if (!p) return
      await ipc.copyToClipboard(p.content)
    },
  },
})
```

- [ ] **Step 8.4: 跑测试通过**

```bash
pnpm test
```
Expected: 全部 PASS

- [ ] **Step 8.5: 提交**

```bash
git add -A
git commit -m "feat(store): library store with search/filter/crud + tests"
```

---

## Task 9: Pinia UI store

**Files:**
- Create: `src/stores/ui.ts`

- [ ] **Step 9.1: 写 ui store**

```ts
// src/stores/ui.ts
import { defineStore } from 'pinia'

export const useUiStore = defineStore('ui', {
  state: () => ({
    panelVisible: false as boolean,
    editorOpen: false as boolean,
    editingPromptId: null as string | null,
    settingsOpen: false as boolean,
    previewImage: null as string | null, // 大图预览
    toast: '' as string,
  }),
  actions: {
    togglePanel() { this.panelVisible = !this.panelVisible },
    showPanel() { this.panelVisible = true },
    hidePanel() { this.panelVisible = false },
    openEditor(id: string | null) { this.editingPromptId = id; this.editorOpen = true },
    closeEditor() { this.editorOpen = false; this.editingPromptId = null },
    openSettings() { this.settingsOpen = true },
    closeSettings() { this.settingsOpen = false },
    preview(img: string | null) { this.previewImage = img },
    showToast(msg: string) {
      this.toast = msg
      setTimeout(() => { this.toast = '' }, 1500)
    },
  },
})
```

- [ ] **Step 9.2: typecheck**

```bash
pnpm typecheck
```
Expected: 通过

- [ ] **Step 9.3: 提交**

```bash
git add src/stores/ui.ts
git commit -m "feat(store): ui store"
```

---

## Task 10: App.vue 主布局

**Files:**
- Modify: `src/main.ts`
- Modify: `src/App.vue`

- [ ] **Step 10.1: 注册 Pinia + Naive UI**

```ts
// src/main.ts
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './styles/main.css'

createApp(App).use(createPinia()).mount('#app')
```

- [ ] **Step 10.2: 写 App.vue 骨架**

```vue
<!-- src/App.vue -->
<script setup lang="ts">
import { onMounted } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import SearchBar from '@/components/SearchBar.vue'
import CategoryTree from '@/components/CategoryTree.vue'
import PromptCard from '@/components/PromptCard.vue'
import PromptEditor from '@/components/PromptEditor.vue'
import SettingsModal from '@/components/SettingsModal.vue'

const lib = useLibraryStore()
const ui = useUiStore()

onMounted(async () => {
  await lib.load()
  ui.showPanel()
})
</script>

<template>
  <div class="app" v-show="ui.panelVisible">
    <header class="topbar">
      <SearchBar />
      <button class="btn" @click="ui.openSettings()">⚙️</button>
      <button class="btn primary" @click="ui.openEditor(null)">＋</button>
    </header>
    <div class="body">
      <aside class="sidebar"><CategoryTree /></aside>
      <main class="content">
        <PromptCard v-for="p in lib.filteredPrompts" :key="p.id" :prompt="p" />
        <p v-if="lib.filteredPrompts.length === 0" class="empty">未找到匹配的提示词</p>
      </main>
    </div>
    <PromptEditor v-if="ui.editorOpen" />
    <SettingsModal v-if="ui.settingsOpen" />
    <div v-if="ui.toast" class="toast">{{ ui.toast }}</div>
  </div>
</template>

<style scoped>
.app { width: 720px; height: 520px; display: flex; flex-direction: column; font-family: system-ui, sans-serif; }
.topbar { display: flex; gap: 8px; padding: 8px; border-bottom: 1px solid #eee; }
.body { flex: 1; display: flex; overflow: hidden; }
.sidebar { width: 160px; border-right: 1px solid #eee; overflow-y: auto; }
.content { flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
.empty { color: #999; text-align: center; margin-top: 32px; }
.btn { cursor: pointer; }
.btn.primary { font-weight: bold; }
.toast { position: fixed; bottom: 16px; left: 50%; transform: translateX(-50%); background: #333; color: #fff; padding: 6px 12px; border-radius: 6px; }
</style>
```

- [ ] **Step 10.3: 写全局样式**

```css
/* src/styles/main.css */
* { box-sizing: border-box; }
body { margin: 0; }
```

- [ ] **Step 10.4: typecheck**

```bash
pnpm typecheck
```
Expected: 通过（组件尚未实现会有 import 报错 → 先做 Task 11-15，或临时建空组件占位）

说明：若此时 typecheck 因组件未实现报错，可先建空占位组件文件，下面 Tasks 逐个填充。

- [ ] **Step 10.5: 提交**

```bash
git add -A
git commit -m "feat(ui): app shell layout"
```

---

## Task 11: SearchBar 组件

**Files:**
- Create: `src/components/SearchBar.vue`

- [ ] **Step 11.1: 写组件**

```vue
<!-- src/components/SearchBar.vue -->
<script setup lang="ts">
import { useLibraryStore } from '@/stores/library'
const lib = useLibraryStore()
</script>

<template>
  <input
    class="search"
    type="text"
    placeholder="🔍 搜索提示词、标签..."
    v-model="lib.search"
  />
</template>

<style scoped>
.search { flex: 1; padding: 6px 10px; border: 1px solid #ddd; border-radius: 6px; font-size: 14px; }
</style>
```

- [ ] **Step 11.2: typecheck + lint**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 11.3: 提交**

```bash
git add src/components/SearchBar.vue
git commit -m "feat(ui): search bar"
```

---

## Task 12: CategoryTree 组件

**Files:**
- Create: `src/components/CategoryTree.vue`

- [ ] **Step 12.1: 写组件**

```vue
<!-- src/components/CategoryTree.vue -->
<script setup lang="ts">
import { useLibraryStore } from '@/stores/library'
const lib = useLibraryStore()
function onAdd() {
  const name = window.prompt('分类名称')
  if (name) lib.addCategory(name)
}
</script>

<template>
  <div class="tree">
    <div
      class="item"
      :class="{ active: lib.currentCategoryId === null }"
      @click="lib.currentCategoryId = null"
    >全部</div>
    <div
      v-for="c in lib.categories"
      :key="c.id"
      class="item"
      :class="{ active: lib.currentCategoryId === c.id }"
      @click="lib.currentCategoryId = c.id"
    >
      <span class="dot" :style="{ background: c.color }"></span>
      {{ c.name }}
      <button class="del" @click.stop="lib.deleteCategory(c.id)">×</button>
    </div>
    <button class="add" @click="onAdd">＋ 新建分类</button>
  </div>
</template>

<style scoped>
.tree { padding: 8px; }
.item { padding: 6px 8px; cursor: pointer; border-radius: 6px; display: flex; align-items: center; gap: 6px; }
.item:hover, .item.active { background: #f0f0f0; }
.dot { width: 8px; height: 8px; border-radius: 50%; }
.del { margin-left: auto; border: none; background: none; cursor: pointer; color: #c00; }
.add { width: 100%; margin-top: 8px; padding: 6px; cursor: pointer; }
</style>
```

- [ ] **Step 12.2: typecheck + lint**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 12.3: 提交**

```bash
git add src/components/CategoryTree.vue
git commit -m "feat(ui): category tree"
```

---

## Task 13: PromptCard 组件

**Files:**
- Create: `src/components/PromptCard.vue`

- [ ] **Step 13.1: 加 read_image_bytes 命令**

在 `src-tauri/src/commands.rs` 加：
```rust
#[tauri::command]
pub fn read_image_bytes(app: tauri::AppHandle, path: String) -> Result<Vec<u8>, String> {
    let full = data_dir(&app).join(&path);
    std::fs::read(&full).map_err(|e| e.to_string())
}
```
并加到 `main.rs` 的 `generate_handler!`（在 Task 17 一并注册）。

- [ ] **Step 13.2: ipc.ts 加 readImageBytes**

```ts
export async function readImageBytes(path: string): Promise<string> {
  const bytes = await invoke<number[]>('read_image_bytes', { path })
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
}
```

- [ ] **Step 13.3: 写组件**

```vue
<!-- src/components/PromptCard.vue -->
<script setup lang="ts">
import { ref, watchEffect } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes } from '@/lib/ipc'
import type { Prompt } from '@/types'

const props = defineProps<{ prompt: Prompt }>()
const lib = useLibraryStore()
const ui = useUiStore()
const url = ref('')

watchEffect(async () => {
  if (props.prompt.image) url.value = await readImageBytes(props.prompt.image)
})

async function onCopy() {
  await lib.copyPrompt(props.prompt.id)
  ui.showToast('已复制 ✓')
}
function onPreview() { if (props.prompt.image) ui.preview(props.prompt.image) }
</script>

<template>
  <div class="card" @click="onCopy">
    <div class="row">
      <div class="title">📌 {{ prompt.title }}</div>
      <img v-if="url" class="thumb" :src="url" @click.stop="onPreview" alt="ref" />
    </div>
    <div class="content">{{ prompt.content }}</div>
    <div class="tags">
      <span v-for="t in prompt.tags" :key="t" class="tag">{{ t }}</span>
    </div>
  </div>
</template>

<style scoped>
.card { border: 1px solid #eee; border-radius: 8px; padding: 10px; cursor: pointer; }
.card:hover { background: #fafafa; }
.row { display: flex; justify-content: space-between; align-items: center; }
.title { font-weight: 600; }
.content { color: #555; font-size: 13px; margin: 4px 0; }
.tags { display: flex; gap: 4px; }
.tag { background: #eee; border-radius: 4px; padding: 1px 6px; font-size: 11px; }
.thumb { width: 48px; height: 48px; object-fit: cover; border-radius: 6px; }
</style>
```

- [ ] **Step 13.4: typecheck + lint**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 13.5: 提交**

```bash
git add -A
git commit -m "feat(ui): prompt card with copy + image"
```

- [ ] **Step 13.2: typecheck + lint**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 13.3: 提交**

```bash
git add -A
git commit -m "feat(ui): prompt card with copy + image"
```

---

## Task 14: PromptEditor 组件

**Files:**
- Create: `src/components/PromptEditor.vue`

- [ ] **Step 14.1: 写编辑弹窗**

```vue
<!-- src/components/PromptEditor.vue -->
<script setup lang="ts">
import { ref, computed } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { saveImage } from '@/lib/ipc'
import type { Prompt } from '@/types'

const lib = useLibraryStore()
const ui = useUiStore()

const editing = computed(() => lib.library.prompts.find(p => p.id === ui.editingPromptId) || null)

const form = ref({
  title: editing.value?.title ?? '',
  content: editing.value?.content ?? '',
  categoryId: editing.value?.categoryId ?? null as string | null,
  tags: editing.value?.tags.join(', ') ?? '',
  image: editing.value?.image ?? null as string | null,
})

async function onPickImage(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const ext = file.name.split('.').pop()?.toLowerCase() || 'png'
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  form.value.image = await saveImage(bytes, ext)
}

function onSave() {
  const tags = form.value.tags.split(',').map(t => t.trim()).filter(Boolean)
  const payload = {
    title: form.value.title,
    content: form.value.content,
    categoryId: form.value.categoryId,
    tags,
    image: form.value.image,
  }
  if (editing.value) {
    lib.updatePrompt(editing.value.id, payload)
  } else {
    lib.addPrompt(payload as Omit<Prompt, 'id' | 'createdAt' | 'updatedAt'>)
  }
  ui.closeEditor()
}
</script>

<template>
  <div class="mask" @click.self="ui.closeEditor()">
    <div class="dialog">
      <h3>{{ editing ? '编辑' : '新建' }}提示词</h3>
      <input v-model="form.title" placeholder="标题" />
      <textarea v-model="form.content" placeholder="提示词内容" rows="5"></textarea>
      <select v-model="form.categoryId">
        <option :value="null">未分类</option>
        <option v-for="c in lib.categories" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <input v-model="form.tags" placeholder="标签，逗号分隔" />
      <input type="file" accept="image/png,image/jpeg,image/webp" @change="onPickImage" />
      <div v-if="form.image">已附图：{{ form.image }}</div>
      <div class="actions">
        <button @click="ui.closeEditor()">取消</button>
        <button class="primary" @click="onSave">保存</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask { position: fixed; inset: 0; background: rgba(0,0,0,0.3); display: flex; align-items: center; justify-content: center; }
.dialog { background: #fff; padding: 16px; border-radius: 8px; width: 420px; display: flex; flex-direction: column; gap: 8px; }
input, textarea, select { padding: 6px; border: 1px solid #ddd; border-radius: 4px; font-size: 14px; }
.actions { display: flex; justify-content: flex-end; gap: 8px; }
.primary { font-weight: bold; }
</style>
```

- [ ] **Step 14.2: typecheck + lint**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 14.3: 提交**

```bash
git add src/components/PromptEditor.vue
git commit -m "feat(ui): prompt editor"
```

---

## Task 15: SettingsModal + 导出/导入

**Files:**
- Create: `src/components/SettingsModal.vue`

- [ ] **Step 15.1: 写设置弹窗**

```vue
<!-- src/components/SettingsModal.vue -->
<script setup lang="ts">
import { ref } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { exportLibrary, importLibrary } from '@/lib/ipc'

const lib = useLibraryStore()
const ui = useUiStore()
const hotkey = ref(lib.library.settings.hotkey)

function saveHotkey() {
  lib.library.settings.hotkey = hotkey.value
  lib.persist()
  ui.showToast('已保存')
}

async function onExport() {
  await exportLibrary()
  ui.showToast('已导出')
}

async function onImport() {
  const imported = await importLibrary()
  if (!imported) return
  const mode = window.confirm('确定覆盖当前库？（取消则合并去重）')
  if (mode) {
    lib.library = imported
  } else {
    const existIds = new Set(lib.library.prompts.map(p => p.id))
    for (const p of imported.prompts) if (!existIds.has(p.id)) lib.library.prompts.push(p)
    const existCats = new Set(lib.library.categories.map(c => c.id))
    for (const c of imported.categories) if (!existCats.has(c.id)) lib.library.categories.push(c)
  }
  lib.persist()
  ui.showToast('已导入')
}
</script>

<template>
  <div class="mask" @click.self="ui.closeSettings()">
    <div class="dialog">
      <h3>设置</h3>
      <label>全局快捷键 <input v-model="hotkey" /></label>
      <button @click="saveHotkey">保存快捷键</button>
      <hr />
      <button @click="onExport">导出 (.zip)</button>
      <button @click="onImport">导入 (.zip)</button>
      <div class="actions"><button @click="ui.closeSettings()">关闭</button></div>
    </div>
  </div>
</template>

<style scoped>
.mask { position: fixed; inset: 0; background: rgba(0,0,0,0.3); display: flex; align-items: center; justify-content: center; }
.dialog { background: #fff; padding: 16px; border-radius: 8px; width: 360px; display: flex; flex-direction: column; gap: 8px; }
input { padding: 6px; border: 1px solid #ddd; border-radius: 4px; }
.actions { display: flex; justify-content: flex-end; }
</style>
```

- [ ] **Step 15.2: typecheck + lint + test**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 15.3: 提交**

```bash
git add src/components/SettingsModal.vue
git commit -m "feat(ui): settings + export/import"
```

---

## Task 16: 参考图大图预览

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 16.1: 加预览浮层**

在 `App.vue` 模板末尾（toast 前）加：
```vue
    <div v-if="ui.previewImage" class="preview-mask" @click="ui.preview(null)">
      <img :src="previewUrl" class="preview-img" alt="preview" />
    </div>
```
并在 setup 加：
```ts
import { ref, watchEffect } from 'vue'
import { readImageBytes } from '@/lib/ipc'
const previewUrl = ref('')
watchEffect(async () => {
  if (ui.previewImage) previewUrl.value = await readImageBytes(ui.previewImage)
})
```
加样式：
```css
.preview-mask { position: fixed; inset: 0; background: rgba(0,0,0,0.7); display: flex; align-items: center; justify-content: center; }
.preview-img { max-width: 90%; max-height: 90%; }
```

- [ ] **Step 16.2: typecheck**

```bash
pnpm check
```
Expected: 通过

- [ ] **Step 16.3: 提交**

```bash
git add src/App.vue
git commit -m "feat(ui): image preview overlay"
```

---

## Task 17: 托盘 + 全局快捷键 + 窗口失焦隐藏

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 17.1: 配置窗口为无边框悬浮**

编辑 `src-tauri/tauri.conf.json` 的 `app.windows[0]`：
```json
{
  "title": "Banana Box",
  "width": 720,
  "height": 520,
  "decorations": false,
  "transparent": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": false,
  "resizable": true
}
```

- [ ] **Step 17.2: 写托盘 + 快捷键 + 失焦**

改 `src-tauri/src/main.rs`：
```rust
mod library;
mod commands;

use tauri::{Manager, Emitter, WindowEvent};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_shortcut(Shortcut::new(Some(Modifiers::CONTROL|Modifiers::SHIFT), Code::KeyB))
            .expect("shortcut")
            .on_shortcut(|app, _s, _e| {
                toggle_panel(app)
            })
            .build())
        .setup(|app| {
            // 托盘
            TrayIconBuilder::with_id("main")
                .tooltip("Banana Box")
                .icon(app.default_window_icon().expect("icon").clone())
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        toggle_panel(app);
                    }
                })
                .menu(
                    tauri::menu::MenuBuilder::new(app)
                        .item(&tauri::menu::MenuItemBuilder::with_id("show", "显示").build(app)?)
                        .item(&tauri::menu::MenuItemBuilder::with_id("quit", "退出").build(app)?)
                        .build()?
                )
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => toggle_panel(app),
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 失焦隐藏
            if let WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_library,
            commands::save_library,
            commands::copy_to_clipboard,
            commands::save_image,
            commands::delete_image,
            commands::read_image_bytes,
            commands::export_library,
            commands::import_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running banana box");
}

fn toggle_panel(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}
```

- [ ] **Step 17.3: 编译验证**

```bash
cd src-tauri && cargo check; cd ..
```
Expected: 通过（按报错补 use 或调整 API 名，Tauri 2 API 偶有差异，以 `cargo check` 为准）

- [ ] **Step 17.4: 提交**

```bash
git add -A
git commit -m "feat(rust): tray + global shortcut + blur-hide"
```

---

## Task 18: 集成验收

- [ ] **Step 18.1: 跑全套检查**

```bash
pnpm check
cd src-tauri && cargo test; cd ..
```
Expected: 前端 typecheck+lint+test 全绿，Rust 测试全绿

- [ ] **Step 18.2: 启动桌面应用验收**

```bash
pnpm tauri dev
```
逐条验证 6 个核心需求：
1. 托盘图标常驻，左键点 → 面板弹出；`Ctrl+Shift+B` 切换显隐；点外部自动隐藏 ✓
2. 点提示词卡片 → 剪贴板有内容 → toast "已复制 ✓" ✓
3. 设置页导出 .zip → 换目录导入 → 数据回 ✓
4. 编辑时上传图 → 卡片显示缩略图 → 点缩略图弹大图 ✓
5. 安装包/内存目测轻量 ✓
6. 分类点击筛选 + 搜索框实时过滤 ✓

- [ ] **Step 18.3: 用 gstack 截图存证**（可选，按全局工作流）

```
/qa 截图悬浮面板、编辑弹窗、设置页
```

- [ ] **Step 18.4: 最终提交 + 打 tag**

```bash
git add -A
git commit -m "chore: integration verified"
git tag v0.1.0
```

---

## 执行方式选择

计划已保存到 `todo.md`。两种执行方式：

1. **Subagent-Driven（推荐）** — 每个 Task 派一个子 agent 执行，任务间 review，快速迭代
2. **Inline Execution** — 在当前会话用 executing-plans 逐任务执行，带检查点

**选哪种？**
