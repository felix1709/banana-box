# Banana Box 提交发布流程

本文档是 Banana Box 的固定发布手册。以后用户说“提交发布”、“提交&发布”、“发布”或要求发布新版本时，先读本文件，再按这里执行。

## 0. 先理解“当前目录不是 Git 仓库”

Git 仓库就是带有 `.git` 隐藏目录的项目文件夹。`.git` 里保存提交历史、分支、远程仓库地址等信息。

如果执行下面命令失败：

```powershell
git rev-parse --show-toplevel
```

并出现：

```text
fatal: not a git repository (or any of the parent directories): .git
```

说明当前目录没有 `.git`，不能执行真正的提交、推送、打 tag、创建 GitHub Release。此时必须停止 Git 发布步骤，告诉用户需要提供带 `.git` 的项目目录，或先恢复/放回 `.git`。

不要自动运行 `git init`，也不要自动改写历史。

## 1. 发布目标

发布包含两件事：

1. 提交代码到 Git 仓库。
2. 构建安装包并发布到 GitHub Release，让用户可以安装或更新到新版本。

## 2. 固定项目信息

- 项目目录：`C:\Users\admin\banana-box-main`
- GitHub 仓库：`https://github.com/felix1709/banana-box.git`
- Git 用户名：`felix1709`
- Git 邮箱：`331631382@qq.com`
- Updater 私钥路径：`C:\Users\admin\.tauri\banana-box-updater.key`
- Updater 私钥密码路径：`C:\Users\admin\.tauri\banana-box-updater.password`

注意：绝对不要打印、提交或上传 updater 私钥和密码内容。

## 3. 发布前检查

在项目根目录执行：

```powershell
git rev-parse --show-toplevel
git status --short
git remote -v
gh auth status
```

如果不是 Git 仓库，停止提交和 GitHub Release 发布。

配置或确认 Git 身份：

```powershell
git config user.name "felix1709"
git config user.email "331631382@qq.com"
git config user.name
git config user.email
```

## 4. 版本号规则

发布前必须升级版本号，不能用旧版本重复发布。

常用规则：

- 只修复问题或优化 UI：补丁版本加 1，例如 `0.1.2 -> 0.1.3`
- 新增明显功能：次版本加 1，例如 `0.1.3 -> 0.2.0`
- 有破坏性变化：主版本加 1，例如 `0.2.0 -> 1.0.0`

以下文件里的版本号必须保持一致：

1. `package.json`
2. `src-tauri/tauri.conf.json`
3. `src-tauri/Cargo.toml`
4. `src-tauri/Cargo.lock`

检查命令：

```powershell
Select-String -Path package.json -Pattern '"version"'
Select-String -Path src-tauri\tauri.conf.json -Pattern '"version"'
Select-String -Path src-tauri\Cargo.toml -Pattern '^version'
Select-String -Path src-tauri\Cargo.lock -Pattern 'name = "banana-box"' -Context 0,3
```

如果本地版本、Cargo 版本、Tauri 配置版本、GitHub Release tag 不一致，先停下说明差异，不要继续发布。

查看 GitHub 最新 Release：

```powershell
gh release list --limit 5
```

## 5. 发布前验证

每次发布都必须重新运行，不能使用旧结果：

```powershell
pnpm check
cargo test --manifest-path src-tauri\Cargo.toml
```

任何一步失败都停止发布，先修复问题。

## 6. 设置 Tauri updater 签名环境变量

构建前设置签名环境变量：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw "C:\Users\admin\.tauri\banana-box-updater.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -Raw "C:\Users\admin\.tauri\banana-box-updater.password"
```

只设置环境变量，不要 `echo` 或打印变量内容。

## 7. 构建安装包和更新清单

执行：

```powershell
pnpm tauri build
pnpm release:manifest
```

构建完成后检查产物：

```powershell
Get-ChildItem src-tauri\target\release\bundle -Recurse
```

通常需要关注：

- Windows 安装包，例如 `.exe` 或 `.msi`
- updater 签名文件，例如 `.sig`
- updater 清单，例如 `latest.json`

发布资产文件名尽量使用英文、数字、横线、下划线，不要使用中文文件名。

## 8. latest.json 要求

如果本次发布包含自动更新清单，必须确认：

1. `version` 等于本次发布版本。
2. `url` 指向本次 GitHub Release 中的安装包资产。
3. `signature` 来自对应 `.sig` 文件。
4. `pub_date` 使用 UTC 时间。
5. Windows 平台键使用 Tauri 期望的平台名。
6. `latest.json` 必须是 UTF-8 无 BOM。

不要用可能写入 BOM 的方式生成 `latest.json`。如需手动写入，使用无 BOM UTF-8：

```powershell
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($latestPath, $json, $utf8NoBom)
```

检查远程或本地 `latest.json` 是否有 BOM：

```powershell
$bytes = [System.IO.File]::ReadAllBytes("latest.json")
$hasUtf8Bom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
$hasUtf8Bom
Get-Content -Raw latest.json | ConvertFrom-Json
```

如果 `$hasUtf8Bom` 是 `True`，不能说发布完成，必须重新生成无 BOM 文件并重新上传。

## 9. 提交代码

先查看改动：

```powershell
git status --short
```

只提交和本次发布相关的文件。常见文件：

```powershell
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock fabu.MD AGENTS.md
```

如果本次还有功能代码改动，也要按文件名明确添加，不要盲目 `git add .`。

提交：

```powershell
git commit -m "chore: release v版本号"
git push
```

禁止使用：

```powershell
git push --force
```

除非用户明确要求强推。

## 10. 创建 GitHub Release

tag 格式固定为 `v版本号`，例如 `v0.1.3`。

创建 Release 示例：

```powershell
gh release create v版本号 `
  --title "Banana Box v版本号" `
  --notes "本版本包含最近的 Banana Box 功能更新、交互优化和问题修复。" `
  路径\安装包.exe `
  路径\latest.json
```

如果 Release 已存在，不要直接覆盖。除非用户明确要求覆盖，否则优先建议升级新版本号。

如用户明确要求覆盖指定资产，才使用：

```powershell
gh release upload v版本号 路径\资产文件 --clobber
```

## 11. 发布后验证

发布后必须检查：

```powershell
gh release view v版本号
```

确认 Release 中至少有：

- 安装包资产
- updater 需要的清单文件，例如 `latest.json`
- 如有签名文件，也确认已上传

下载远程 `latest.json` 后验证：

```powershell
gh release download v版本号 --pattern latest.json --dir .release-check --clobber
$bytes = [System.IO.File]::ReadAllBytes(".release-check\latest.json")
$hasUtf8Bom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
$hasUtf8Bom
$json = Get-Content -Raw ".release-check\latest.json" | ConvertFrom-Json
$json.version
$json
```

必须确认：

- `$hasUtf8Bom` 是 `False`
- `ConvertFrom-Json` 能正常解析
- `version` 是本次发布版本
- `url` 指向本次 Release 资产
- `signature` 存在且长度正常

如果 GitHub CDN 短时间缓存旧文件，需要分别验证：

- `gh release download` 下载结果
- `releases/download/v版本号/latest.json`
- `releases/latest/download/latest.json`

等待缓存刷新后再确认。

## 12. 最终回复用户

发布完成后，用中文告诉用户：

1. 发布的版本号。
2. GitHub Release 地址。
3. 安装包资产名。
4. `latest.json` 是否无 BOM。
5. 更新器是否可以正常解析。
6. 如果需要用户手动测试，告诉用户打开软件后点击哪里检查更新。

## 13. 当前目录不是 Git 仓库时怎么办

如果当前目录不是 Git 仓库：

1. 明确告诉用户 `.git` 缺失。
2. 不要提交。
3. 不要推送。
4. 不要创建 GitHub Release。
5. 可以继续整理发布文档或做本地构建检查。
6. 等用户提供正确 Git 仓库目录，或恢复 `.git` 后，再继续完整发布流程。
