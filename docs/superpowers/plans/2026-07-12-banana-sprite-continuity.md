# 香蕉悬浮图标连续动画实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** 重制未剥开到完全剥开的 12 帧香蕉动画，以用户参考图的角度、比例和画风为准，并由测试防止视觉跳变。

**Architecture:** Vue 播放器、64px 点击范围、帧号、360ms 时序与第 6 帧揭示协议不变。资源以 4 列 x 3 行草图生成，去除色键背景后裁切为 12 个 256px 画布，再无损拼接为横向 WebP。Rust 测试锁定端点、透明边缘、中心和相邻帧的尺寸变化。

**Tech Stack:** Vue 3、Vitest、Rust image crate、FFmpeg、内置 ImageGen、Pillow 色键移除脚本、Tauri 2。

---

## 文件边界

- src/assets/banana/banana-peel-sprite.webp：最终 3072 x 256px 无损横向精灵图。
- src/assets/banana/banana-closed-mirrored-approved.png：第 0 帧审核端点。
- src/assets/banana/banana-open-mirrored-approved.png：第 11 帧审核端点。
- docs/design/banana-closed-mirrored-approved.sha256：第 0 帧审核哈希。
- docs/design/banana-open-mirrored-approved.sha256：第 11 帧审核哈希。
- src-tauri/tests/banana_assets.rs：资源完整性测试。
- tests/components/AnimatedBananaButton.test.ts：组件起始帧与显示区域测试。

### Task 1：先写会失败的资源契约

**Files:**
- Create: docs/design/banana-closed-mirrored-approved.sha256
- Modify: src-tauri/tests/banana_assets.rs
- Modify: tests/components/AnimatedBananaButton.test.ts

- [ ] **Step 1: 增加合拢端点与哈希断言**

在 banana_assets.rs 中新增合拢端点与记录文件，并让两端都校验哈希和像素：

~~~rust
let closed_path = root.join("src/assets/banana/banana-closed-mirrored-approved.png");
let closed_hash = root.join("docs/design/banana-closed-mirrored-approved.sha256");
let approved_closed = image::open(&closed_path)
    .expect("approved closed endpoint must exist")
    .to_rgba8();

assert_hash_matches(&closed_path, &closed_hash);
assert_hash_matches(&open_path, &hash_path);
assert_visible_pixels_equal(&frames[0], &approved_closed, "frame 0 must equal approved closed endpoint");
assert_visible_pixels_equal(&frames[11], &approved_open, "frame 11 must equal approved open endpoint");
~~~

新增 bbox_extent 辅助函数。相邻帧的宽和高差不超过 32px：

~~~rust
let (left_width, left_height) = bbox_extent(alpha_bbox(&frames[index]).unwrap());
let (right_width, right_height) = bbox_extent(alpha_bbox(&frames[index + 1]).unwrap());
assert!(
    (left_width as i32 - right_width as i32).unsigned_abs() <= 32
        && (left_height as i32 - right_height as i32).unsigned_abs() <= 32,
    "adjacent frame {index} must not change banana scale abruptly"
);
~~~

assert_hash_matches 必须计算 SHA-256，并要求记录文件以实际十六进制哈希开头。先创建只含 pending 的合拢端点记录。

在 AnimatedBananaButton.test.ts 追加：

~~~ts
it('keeps the approved closed artwork as the first rendered sprite frame', () => {
  const wrapper = mount(AnimatedBananaButton, { props: { open: false } })

  expect(wrapper.attributes('data-frame')).toBe('0')
  expect(wrapper.find('.banana-sprite').attributes('aria-hidden')).toBe('true')
})
~~~

- [ ] **Step 2: 运行 RED**

~~~powershell
Set-Location 'C:\Users\Felix\Downloads\banana-box-workspace\.worktrees\codex-v1-execution'
cargo test --manifest-path src-tauri/Cargo.toml --test banana_assets
pnpm test -- tests/components/AnimatedBananaButton.test.ts
~~~

Expected: Rust 测试仅因合拢端点记录为 pending 而失败；组件测试通过。

- [ ] **Step 3: 提交测试契约**

~~~powershell
git add src-tauri/tests/banana_assets.rs tests/components/AnimatedBananaButton.test.ts docs/design/banana-closed-mirrored-approved.sha256
git commit -m "test: lock banana sprite continuity contract"
~~~

### Task 2：生成与审核 12 帧草图

**Files:**
- Create: tmp/banana-v2/reference.png
- Create: tmp/banana-v2/sheet-chroma.png
- Create: tmp/banana-v2/sheet-alpha.png
- Create: tmp/banana-v2/sequence-preview.png

- [ ] **Step 1: 保存用户参考图供本地处理**

~~~powershell
Set-Location 'C:\Users\Felix\Downloads\banana-box-workspace\.worktrees\codex-v1-execution'
New-Item -ItemType Directory -Force tmp\banana-v2 | Out-Null
Copy-Item 'C:\Users\Felix\AppData\Local\Temp\codex-clipboard-83118587-44cb-4d2e-83f8-bb2acdc6ad7d.png' tmp\banana-v2\reference.png
~~~

用 view_image 确认它仅作视觉参考，不把截图蓝色背景带入最终资源。

- [ ] **Step 2: 用内置 ImageGen 生成 4 列 x 3 行草图**

把 reference.png 作为参考图，使用以下提示词：

~~~text
Use case: stylized-concept.
Create one square 1024 by 1024 reference sheet containing exactly 12 animation frames
in a strict invisible 4 columns by 3 rows grid, sequence left-to-right then top-to-bottom.
Match the supplied reference banana's compact proportion, warm yellow peel, dark hand-drawn
outline, playful polished cartoon style, and horizontally mirrored direction. Frame 0 is one
complete unpeeled banana. Frames 1 through 10 gradually and coherently peel open that same
banana. Frame 11 is completely peeled. Keep body center, angle, scale, line weight, and
lighting identical in every cell; only the peel opening changes. Flat #00ff00 chroma-key
background; no grid lines, no shadows, no text, no face, no hands, no stickers, no extra fruit,
no UI, and no cropped banana.
~~~

复制生成结果为 tmp/banana-v2/sheet-chroma.png，不覆盖 src/assets 中的文件。

- [ ] **Step 3: 去色键并生成横向审核预览**

~~~powershell
python 'C:\Users\Felix\.codex\skills\.system\imagegen\scripts\remove_chroma_key.py' --input tmp\banana-v2\sheet-chroma.png --out tmp\banana-v2\sheet-alpha.png --key-color '#00ff00' --soft-matte --transparent-threshold 12 --opaque-threshold 220 --despill
~~~

用 FFmpeg 从 sheet-alpha.png 依次裁切 0,0、256,0、512,0、768,0、0,256、256,256、512,256、768,256、0,512、256,512、512,512、768,512 的 12 个 256 x 256 区域，并使用 hstack=inputs=12 写入 tmp/banana-v2/sequence-preview.png。

用 view_image 检查：12 格完整、首格未剥开、末格全剥开、没有背景、没有裁切、每格方向/大小/画风一致。把预览给用户确认；未确认不得导出正式资源。

### Task 3：导出、验收并启动调试

**Files:**
- Modify: src/assets/banana/banana-peel-sprite.webp
- Modify: src/assets/banana/banana-closed-mirrored-approved.png
- Modify: src/assets/banana/banana-open-mirrored-approved.png
- Modify: docs/design/banana-closed-mirrored-approved.sha256
- Modify: docs/design/banana-open-mirrored-approved.sha256

- [ ] **Step 1: 由审核草图导出端点和无损精灵图**

用 FFmpeg 将第 0 格导出为 banana-closed-mirrored-approved.png，将第 11 格导出为 banana-open-mirrored-approved.png。按 Task 2 的同一裁切顺序 hstack 为 3072 x 256 RGBA 图，并使用以下编码参数覆盖 banana-peel-sprite.webp：

~~~powershell
-c:v libwebp -lossless 1 -q:v 100 -pix_fmt yuva420p
~~~

为两个 PNG 重新生成 SHA-256 记录。每个记录的第一行必须是实际文件哈希，第二行必须是：

~~~text
2026-07-12 用户提供截图为比例和角度参考；12 帧整组重制；全序列水平镜像。
~~~

- [ ] **Step 2: 运行 GREEN 和静态检查**

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test banana_assets
pnpm test -- tests/lib/bananaAnimation.test.ts tests/components/AnimatedBananaButton.test.ts tests/components/FloatButton.test.ts
pnpm typecheck
pnpm lint
~~~

Expected: Rust 资源测试 1 项通过；列出的 Vitest 全部通过；类型检查与 lint 退出码均为 0。

- [ ] **Step 3: 启动桌面调试并做真实逐帧验收**

~~~powershell
pnpm tauri dev --config src-tauri/tauri.dev-1423.conf.json
~~~

在 100% 与 200% Windows 缩放下各确认一次：

1. 收起时第 0 帧是未剥开的香蕉。
2. 打开时前向播放，关闭时从当前帧反向播放。
3. 起始、中段、第 6 帧和终态无突然缩放、位置漂移或旧素材跳回。
4. 香蕉始终位于 52px 显示框中心区域，64px 点击热区不变。

- [ ] **Step 4: 只提交香蕉资源重制相关文件**

~~~powershell
git add src/assets/banana/banana-peel-sprite.webp src/assets/banana/banana-closed-mirrored-approved.png src/assets/banana/banana-open-mirrored-approved.png docs/design/banana-closed-mirrored-approved.sha256 docs/design/banana-open-mirrored-approved.sha256 src-tauri/tests/banana_assets.rs tests/components/AnimatedBananaButton.test.ts
git commit -m "fix: align banana sprite opening sequence"
~~~

Expected: 不提交 tmp 文件，也不混入项目管理、当日任务、故事板或设置页改动。
