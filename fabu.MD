# Banana Box 发布流程

本文件是 Banana Box 的固定发布流程。以后每次用户要求“发布”“发版”“发布到 GitHub Release”时，都按这里执行。

## 1. 发布前确认

1. 确认当前目录是项目根目录：`C:\Users\admin\banana-box-main`。
2. 确认项目是 Git 仓库，并且已经配置 GitHub 远程仓库：
   ```powershell
   git rev-parse --show-toplevel
   git remote -v
   ```
3. 确认 GitHub CLI 已登录：
   ```powershell
   gh auth status
   ```
4. 确认 pnpm 允许构建 Tauri/Vite 需要的 `esbuild`：
   ```powershell
   Get-Content pnpm-workspace.yaml
   ```
   文件中应包含：
   ```yaml
   allowBuilds:
     esbuild: true
   ```
5. 确认工作区状态，不能覆盖用户未确认的修改：
   ```powershell
   git status --short
   ```

如果当前目录不是 Git 仓库，或者没有 GitHub 远程仓库，先停止发布，只能做本地打包，不能创建 GitHub Release。

## 2. 确定新版本号

Banana Box 使用语义化版本号：`主版本.次版本.补丁版本`，例如 `0.1.1`。

- 只修复问题或优化 UI：补丁版本加 1，例如 `0.1.0 -> 0.1.1`。
- 新增明显功能：次版本加 1，例如 `0.1.1 -> 0.2.0`。
- 有破坏性变化：主版本加 1，例如 `0.2.0 -> 1.0.0`。

发布前必须同步修改这些文件里的版本号：

1. `package.json`
2. `src-tauri/tauri.conf.json`
3. `src-tauri/Cargo.toml`
4. `src-tauri/Cargo.lock` 中 `name = "banana-box"` 对应的版本

## 3. 发布前验证

每次发布前都必须重新运行以下命令，不能使用旧结果：

```powershell
.\node_modules\.bin\vue-tsc.CMD --noEmit
.\node_modules\.bin\vitest.CMD run
.\node_modules\.bin\eslint.CMD src
cargo check
```

其中 `cargo check` 需要在 `src-tauri` 目录运行：

```powershell
Set-Location src-tauri
cargo check
Set-Location ..
```

任何一步失败都停止发布，先修复问题。

## 4. 构建安装包

在项目根目录运行：

```powershell
.\node_modules\.bin\tauri.CMD build
```

构建完成后，检查安装包目录：

```powershell
Get-ChildItem src-tauri\target\release\bundle -Recurse
```

Windows 发布优先上传 `.msi` 或 `.exe` 安装包。如果生成了签名文件或 updater 文件，也一起保留并按需要上传。

## 5. 提交发布修改

如果项目是 Git 仓库，按以下顺序提交：

```powershell
git status --short
git add package.json pnpm-workspace.yaml src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock fabu.MD
git commit -m "chore: release v版本号"
git push
```

不要使用 `git push --force`。

## 6. 创建 GitHub Release

使用版本号创建 tag 和 GitHub Release。tag 格式固定为 `v版本号`，例如 `v0.1.1`。

```powershell
gh release create v版本号 `
  --title "Banana Box v版本号" `
  --notes "本版本包含最近的 Banana Box 功能更新、交互优化和修复。" `
  路径\到\安装包.msi
```

如果有多个安装文件，可以追加到同一条命令后面：

```powershell
gh release create v版本号 `
  --title "Banana Box v版本号" `
  --notes "本版本包含最近的 Banana Box 功能更新、交互优化和修复。" `
  文件1.msi 文件2.exe
```

## 7. 发布后检查

1. 打开 GitHub Release 页面，确认版本号、标题、说明、安装包都存在。
2. 下载刚发布的安装包。
3. 在 Windows 上安装，确认可以覆盖旧版本或作为更新版本安装。
4. 启动 Banana Box，确认主要功能可用：
   - 悬浮按钮可打开主面板
   - 单击提示词展开，再次单击收起
   - 删除、编辑、分类、复制按钮可用
   - 分类按钮直接显示分类菜单
   - 图片上传、拖拽、粘贴、预览可用

## 8. 当前发布阻塞说明

如果执行发布时发现当前目录不是 Git 仓库，或者没有远程仓库，则不能真正发布到 GitHub Release。此时应完成：

1. 更新版本号。
2. 运行验证。
3. 构建安装包。
4. 告诉用户需要提供或初始化 GitHub 仓库后，才能执行 `gh release create`。
