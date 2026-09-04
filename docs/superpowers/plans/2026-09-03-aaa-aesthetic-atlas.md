# AAA-Aesthetic-Atlas 与 Banana Box 集成实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Banana Box 能创建并展示 `AAA-Aesthetic-Atlas` 知识库，让 `AAA-prompt-felix` 通过 MCP 查询同一个知识库，并通过 Chrome 插件把网页图片送入 Banana Box 入库。

**Architecture:** 知识库文件夹是唯一数据源，Banana Box 负责收集、识图、展示；Python MCP 服务负责查询；Banana Box 本地 HTTP 服务负责 Chrome 插件入库。

**Tech Stack:** Tauri 2 + Rust + Vue 3 + TypeScript + Pinia + Python 3 + MCP SDK + Chrome Extension Manifest V3。

## Global Constraints

- 知识库默认路径：`%USERPROFILE%\Documents\AAA-Aesthetic-Atlas`
- 配置路径：`%USERPROFILE%\.banana-box\atlas-config.json`
- 数据格式：`Markdown + YAML frontmatter`，图片放在 `assets/`，条目放在 `entries/`。
- 所有写入必须“临时文件 + 原子替换”。
- Banana Box 本地服务只监听 `127.0.0.1`，使用本地 token。
- MCP 服务只读知识条目；`feedback` 只追加反馈，不修改条目正文。
- 识图模型按配置顺序自动切换；图片过大自动压缩。
- 不提交 API Key、私钥、密码。

---

### Task 1: 初始化知识库目录和路径配置

**Files:**
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\init_atlas.py`
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_init_atlas.py`

**Interfaces:**
- Consumes: Python 3, `PyYAML`。
- Produces: `init_atlas.ensure_atlas(root: str | None = None) -> str`；`init_atlas.load_config() -> dict`；`init_atlas.save_config(root: str) -> None`。

- [ ] **Step 1: 写失败测试**

```python
import os
import tempfile
import unittest

import init_atlas


class InitAtlasTest(unittest.TestCase):
    def test_ensure_atlas_creates_expected_folders(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = init_atlas.ensure_atlas(os.path.join(tmp, "atlas"))
            for folder in ["entries", "assets", "index"]:
                self.assertTrue(os.path.isdir(os.path.join(root, folder)))

    def test_config_round_trip(self):
        with tempfile.TemporaryDirectory() as tmp:
            config_path = os.path.join(tmp, "atlas-config.json")
            init_atlas.save_config(config_path, os.path.join(tmp, "atlas"))
            self.assertEqual(
                init_atlas.load_config(config_path)["rootPath"],
                os.path.join(tmp, "atlas"),
            )


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_init_atlas.py`
Expected: FAIL with `ModuleNotFoundError`。

- [ ] **Step 3: 实现 init_atlas**

```python
import json
import os
from datetime import date
from pathlib import Path

DIMENSIONS = [
    "01-scene-concept",
    "02-style",
    "03-character-pose",
    "04-composition",
    "05-lighting-atmosphere",
    "06-fx-effects",
    "07-color-texture",
    "08-emotion-mood",
    "09-camera-lens",
    "10-model-constraints",
]


def default_root() -> Path:
    return Path.home() / "Documents" / "AAA-Aesthetic-Atlas"


def config_path() -> Path:
    return Path.home() / ".banana-box" / "atlas-config.json"


def ensure_atlas(root: str | None = None) -> str:
    target = Path(root).expanduser() if root else default_root()
    for base in ("entries", "assets", "index"):
        base_dir = target / base
        base_dir.mkdir(parents=True, exist_ok=True)
        for dimension in DIMENSIONS:
            (base_dir / dimension).mkdir(parents=True, exist_ok=True)
    (target / "README.md").write_text("# AAA-Aesthetic-Atlas\n", encoding="utf-8")
    return str(target)


def load_config(path: str | None = None) -> dict:
    target = Path(path) if path else config_path()
    if not target.exists():
        return {}
    return json.loads(target.read_text(encoding="utf-8"))


def save_config(root: str, path: str | None = None) -> None:
    target = Path(path) if path else config_path()
    target.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": 1,
        "rootPath": str(Path(root).expanduser()),
        "createdAt": date.today().isoformat(),
    }
    tmp = target.with_suffix(".tmp")
    tmp.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(tmp, target)
```

- [ ] **Step 4: 运行测试确认通过**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_init_atlas.py`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/init_atlas.py C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/test_init_atlas.py
git commit -m "feat: initialize atlas folders and config"
```

---

### Task 2: 实现知识库文件读写和索引重建

**Files:**
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\atlas_store.py`
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_atlas_store.py`

**Interfaces:**
- Consumes: `init_atlas.DIMENSIONS`。
- Produces: `AtlasEntry` dataclass；`read_entry(root, id) -> AtlasEntry`；`rebuild_index(root) -> dict`；`list_entries(root) -> list[dict]`。

- [ ] **Step 1: 写失败测试**

```python
import json
import os
import tempfile
import unittest

import init_atlas
import atlas_store


class AtlasStoreTest(unittest.TestCase):
    def test_rebuild_index_reads_yaml(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = init_atlas.ensure_atlas(os.path.join(tmp, "atlas"))
            entry_dir = os.path.join(root, "entries", "05-lighting-atmosphere")
            md = os.path.join(entry_dir, "test-entry.md")
            with open(md, "w", encoding="utf-8") as handle:
                handle.write(
                    "---\nid: test-entry\ntitle: 测试灯光\ndimension: lighting-atmosphere\ntags: [测试]\nstatus: confirmed\n---\n\n## 提示词片段\n逆光测试\n"
                )
            index = atlas_store.rebuild_index(root)
            self.assertEqual(index["entries"][0]["id"], "test-entry")

    def test_list_entries_returns_metadata(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = init_atlas.ensure_atlas(os.path.join(tmp, "atlas"))
            entry_dir = os.path.join(root, "entries", "05-lighting-atmosphere")
            with open(os.path.join(entry_dir, "test-entry.md"), "w", encoding="utf-8") as handle:
                handle.write(
                    "---\nid: test-entry\ntitle: 测试灯光\ndimension: lighting-atmosphere\ntags: [测试]\nstatus: confirmed\n---\n"
                )
            entries = atlas_store.list_entries(root)
            self.assertEqual(entries[0]["title"], "测试灯光")


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_atlas_store.py`
Expected: FAIL with `ModuleNotFoundError`。

- [ ] **Step 3: 实现 atlas_store**

```python
import json
import os
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import yaml

import init_atlas


@dataclass
class AtlasEntry:
    id: str
    title: str
    dimension: str
    tags: list[str]
    status: str
    file: str
    image: str


def _read_frontmatter(md_path: Path) -> dict:
    text = md_path.read_text(encoding="utf-8-sig")
    match = re.match(r"^---\n(.*?)\n---\n?", text, re.DOTALL)
    if not match:
        return {}
    return yaml.safe_load(match.group(1)) or {}


def _dimension_folder(dimension: str) -> str:
    mapping = {
        "scene-concept": "01-scene-concept",
        "style": "02-style",
        "character-pose": "03-character-pose",
        "composition": "04-composition",
        "lighting-atmosphere": "05-lighting-atmosphere",
        "fx-effects": "06-fx-effects",
        "color-texture": "07-color-texture",
        "emotion-mood": "08-emotion-mood",
        "camera-lens": "09-camera-lens",
        "model-constraints": "10-model-constraints",
    }
    return mapping.get(dimension, dimension)


def _image_path(root: str, entry_id: str, dimension: str) -> str:
    folder = _dimension_folder(dimension)
    for ext in (".jpg", ".jpeg", ".png", ".webp", ".gif"):
        candidate = Path(root) / "assets" / folder / f"{entry_id}{ext}"
        if candidate.exists():
            return f"assets/{folder}/{candidate.name}"
    return ""


def rebuild_index(root: str) -> dict:
    entries_root = Path(root) / "entries"
    entries = []
    for md_path in sorted(entries_root.rglob("*.md")):
        front = _read_frontmatter(md_path)
        if not front.get("id"):
            continue
        relative = md_path.relative_to(root).as_posix()
        entries.append(
            {
                "id": str(front["id"]),
                "file": relative,
                "image": _image_path(root, str(front["id"]), str(front.get("dimension", ""))),
                "dimension": str(front.get("dimension", "")),
                "tags": list(front.get("tags", [])),
                "score": float(front.get("score", 0.0)),
                "status": str(front.get("status", "pending_review")),
                "title": str(front.get("title", front["id"])),
            }
        )
    payload = {
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "entries": entries,
    }
    index_path = Path(root) / "index" / "index.json"
    tmp = index_path.with_suffix(".tmp")
    tmp.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(tmp, index_path)
    return payload


def read_entry(root: str, entry_id: str) -> dict | None:
    entries_root = Path(root) / "entries"
    for md_path in entries_root.rglob("*.md"):
        front = _read_frontmatter(md_path)
        if front.get("id") == entry_id:
            return {"frontmatter": front, "body": md_path.read_text(encoding="utf-8-sig")}
    return None


def list_entries(root: str) -> list[dict]:
    index_path = Path(root) / "index" / "index.json"
    if index_path.exists():
        payload = json.loads(index_path.read_text(encoding="utf-8"))
        return payload.get("entries", [])
    return rebuild_index(root).get("entries", [])
```

- [ ] **Step 4: 运行测试确认通过**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_atlas_store.py`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/atlas_store.py C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/test_atlas_store.py
git commit -m "feat: read atlas entries and rebuild index"
```

---

### Task 3: 实现 Python MCP 服务

**Files:**
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\aaa_atlas_mcp.py`
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_mcp_tools.py`

**Interfaces:**
- Consumes: `atlas_store.list_entries`, `atlas_store.read_entry`, `init_atlas.load_config`, `init_atlas.ensure_atlas`。
- Produces: MCP tools `list_tags`, `match_tags`, `get_entry`, `feedback`。

- [ ] **Step 1: 写工具逻辑失败测试**

```python
import os
import tempfile
import unittest

import init_atlas
import aaa_atlas_mcp


class McpToolsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = init_atlas.ensure_atlas(os.path.join(self.tmp.name, "atlas"))

    def tearDown(self):
        self.tmp.cleanup()

    def test_match_tags_filters_by_tag(self):
        os.makedirs(os.path.join(self.root, "entries", "05-lighting-atmosphere"))
        with open(os.path.join(self.root, "entries", "05-lighting-atmosphere", "a.md"), "w", encoding="utf-8") as handle:
            handle.write(
                "---\nid: a\ntitle: 霓虹逆光\ndimension: lighting-atmosphere\ntags: [霓虹]\nstatus: confirmed\n---\n"
            )
        result = aaa_atlas_mcp.match_tags(self.root, ["霓虹"])
        self.assertEqual(result[0]["id"], "a")


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_mcp_tools.py`
Expected: FAIL with `ModuleNotFoundError`。

- [ ] **Step 3: 实现 MCP 工具函数和服务**

```python
import json
import os
from pathlib import Path

import init_atlas
import atlas_store


def match_tags(root: str, tags: list[str]) -> list[dict]:
    entries = atlas_store.list_entries(root)
    if not tags:
        return entries[:20]
    wanted = {tag.lower() for tag in tags}
    matched = [
        entry
        for entry in entries
        if wanted.intersection({tag.lower() for tag in entry.get("tags", [])})
    ]
    return matched[:20]


def list_tags(root: str, dimension: str | None = None) -> list[str]:
    entries = atlas_store.list_entries(root)
    if dimension:
        entries = [entry for entry in entries if entry.get("dimension") == dimension]
    tags = {tag for entry in entries for tag in entry.get("tags", [])}
    return sorted(tags)


def get_entry(root: str, entry_id: str) -> dict | None:
    return atlas_store.read_entry(root, entry_id)


def feedback(root: str, entry_id: str, rating: str, note: str) -> dict:
    feedback_dir = Path(root) / "index" / "feedback"
    feedback_dir.mkdir(parents=True, exist_ok=True)
    payload = {"entryId": entry_id, "rating": rating, "note": note}
    path = feedback_dir / f"{entry_id}.json"
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(tmp, path)
    return payload


def current_root() -> str:
    config = init_atlas.load_config()
    root = config.get("rootPath")
    if root and Path(root).exists():
        return str(root)
    return init_atlas.ensure_atlas()


if __name__ == "__main__":
    try:
        from mcp.server.fastmcp import FastMCP

        mcp = FastMCP("aaa-aesthetic-atlas-mcp")

        @mcp.tool()
        def list_tags_tool(dimension: str | None = None) -> list[str]:
            return list_tags(current_root(), dimension)

        @mcp.tool()
        def match_tags_tool(tags: list[str]) -> list[dict]:
            return match_tags(current_root(), tags)

        @mcp.tool()
        def get_entry_tool(entry_id: str) -> dict | None:
            return get_entry(current_root(), entry_id)

        @mcp.tool()
        def feedback_tool(entry_id: str, rating: str, note: str) -> dict:
            return feedback(current_root(), entry_id, rating, note)

        mcp.run()
    except ImportError:
        raise SystemExit(
            "MCP SDK 未安装。请先运行：pip install mcp pyyaml"
        )
```

- [ ] **Step 4: 运行测试确认通过**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_mcp_tools.py`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/aaa_atlas_mcp.py C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/test_mcp_tools.py
git commit -m "feat: add atlas mcp tools"
```

---

### Task 4: 自动注册 MCP 到 Codex 和 Claude Code

**Files:**
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\register_mcp.py`
- Create: `C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_register_mcp.py`

**Interfaces:**
- Consumes: `aaa_atlas_mcp` 脚本绝对路径。
- Produces: `register_mcp.upsert_codex_config() -> bool`；`register_mcp.upsert_claude_config() -> bool`。

- [ ] **Step 1: 写注册失败测试**

```python
import json
import os
import tempfile
import unittest

import register_mcp


class RegisterMcpTest(unittest.TestCase):
    def test_upsert_codex_config_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "config.toml")
            register_mcp.upsert_codex_config(path, "python", ["aaa_atlas_mcp.py"])
            first = open(path, encoding="utf-8").read()
            register_mcp.upsert_codex_config(path, "python", ["aaa_atlas_mcp.py"])
            second = open(path, encoding="utf-8").read()
            self.assertEqual(first, second)
            self.assertIn("aaa-atlas-mcp", first)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_register_mcp.py`
Expected: FAIL with `ModuleNotFoundError`。

- [ ] **Step 3: 实现注册逻辑**

```python
import json
import os
from pathlib import Path


def upsert_codex_config(path: str, command: str, args: list[str]) -> bool:
    config = Path(path)
    if not config.exists():
        config.write_text("", encoding="utf-8")
    text = config.read_text(encoding="utf-8")
    marker = "aaa-aesthetic-atlas-mcp"
    if f'[mcp_servers.{marker}]' in text:
        return False
    block = (
        f"\n[mcp_servers.{marker}]\n"
        f"command = {json.dumps(command)}\n"
        f"args = {json.dumps(args)}\n"
        "startup_timeout_sec = 120\n"
    )
    tmp = config.with_suffix(".tmp")
    tmp.write_text(text + block, encoding="utf-8")
    os.replace(tmp, config)
    return True


def upsert_claude_config(path: str, command: str, args: list[str]) -> bool:
    config = Path(path)
    payload = {}
    if config.exists():
        payload = json.loads(config.read_text(encoding="utf-8"))
    mcp = payload.setdefault("mcpServers", {})
    if "aaa-aesthetic-atlas-mcp" in mcp:
        return False
    mcp["aaa-aesthetic-atlas-mcp"] = {"command": command, "args": args}
    tmp = config.with_suffix(".tmp")
    tmp.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(tmp, config)
    return True


def register_all() -> dict[str, bool]:
    home = Path.home()
    codex_ok = upsert_codex_config(
        str(home / ".codex" / "config.toml"),
        "python",
        ["C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/aaa_atlas_mcp.py"],
    )
    claude_ok = upsert_claude_config(
        str(home / ".claude.json"),
        "python",
        ["C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/aaa_atlas_mcp.py"],
    )
    return {"codex": codex_ok, "claude": claude_ok}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\test_register_mcp.py`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/register_mcp.py C:/Users/admin/.codex/skills/AAA-Aesthetic-Atlas/scripts/test_register_mcp.py
git commit -m "feat: register atlas mcp for codex and claude"
```

---

### Task 5: 添加 Rust Atlas 数据类型和读取仓库

**Files:**
- Create: `src-tauri/src/atlas/mod.rs`
- Create: `src-tauri/src/atlas/repository.rs`
- Test: `src-tauri/src/atlas/repository.rs`

**Interfaces:**
- Consumes: `init_atlas` 创建的目录；`serde`, `serde_yaml`。
- Produces: `AtlasEntryDto`；`AtlasRepository::new(root: PathBuf)`；`list_entries()`；`get_entry(id)`；`open_folder()`。

- [ ] **Step 1: 添加 serde_yaml 依赖**

Modify `src-tauri/Cargo.toml`：

```toml
serde_yaml = "0.9"
```

- [ ] **Step 2: 写 Rust 失败测试**

在 `src-tauri/src/atlas/repository.rs` 中：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn list_entries_reads_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("atlas");
        fs::create_dir_all(root.join("entries/05-lighting-atmosphere")).unwrap();
        fs::write(
            root.join("entries/05-lighting-atmosphere/a.md"),
            "---\nid: a\ntitle: 霓虹逆光\ndimension: lighting-atmosphere\ntags: [霓虹]\nstatus: confirmed\n---\n",
        )
        .unwrap();
        let repo = AtlasRepository::new(root);
        let entries = repo.list_entries().unwrap();
        assert_eq!(entries[0].id, "a");
    }
}
```

- [ ] **Step 3: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri\Cargo.toml atlas::repository`
Expected: FAIL，模块不存在。

- [ ] **Step 4: 实现 repository**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasEntryDto {
    pub id: String,
    pub title: String,
    pub dimension: String,
    pub tags: Vec<String>,
    pub image: String,
    pub score: f64,
    pub status: String,
    pub file: String,
}

pub struct AtlasRepository {
    root: PathBuf,
}

impl AtlasRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list_entries(&self) -> Result<Vec<AtlasEntryDto>, String> {
        let entries_root = self.root.join("entries");
        let mut result = Vec::new();
        if !entries_root.exists() {
            return Ok(result);
        }
        for entry in walkdir::WalkDir::new(&entries_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        {
            let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            let Some(front) = parse_frontmatter(&content) else {
                continue;
            };
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let image = self.resolve_image(&front, entry.path())?;
            result.push(AtlasEntryDto {
                id: front.id.clone(),
                title: front.title.clone(),
                dimension: front.dimension.clone(),
                tags: front.tags.clone(),
                image,
                score: front.score,
                status: front.status.clone(),
                file: relative,
            });
        }
        Ok(result)
    }

    pub fn get_entry(&self, id: &str) -> Result<Option<String>, String> {
        for entry in self.list_entries()? {
            if entry.id == id {
                let path = self.root.join(&entry.file);
                return std::fs::read_to_string(path)
                    .map(Some)
                    .map_err(|e| e.to_string());
            }
        }
        Ok(None)
    }

    fn resolve_image(&self, front: &AtlasFrontmatter, md_path: &Path) -> Result<String, String> {
        let folder = dimension_folder(&front.dimension);
        let assets_dir = self.root.join("assets").join(folder);
        for ext in ["jpg", "jpeg", "png", "webp", "gif"] {
            let candidate = assets_dir.join(format!("{}.{}", front.id, ext));
            if candidate.exists() {
                let relative = candidate
                    .strip_prefix(&self.root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                return Ok(relative);
            }
        }
        Ok(String::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasFrontmatter {
    id: String,
    title: String,
    dimension: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    score: f64,
    #[serde(default = "default_status")]
    status: String,
}

fn default_status() -> String {
    "pending_review".to_string()
}

fn dimension_folder(dimension: &str) -> &str {
    match dimension {
        "scene-concept" => "01-scene-concept",
        "style" => "02-style",
        "character-pose" => "03-character-pose",
        "composition" => "04-composition",
        "lighting-atmosphere" => "05-lighting-atmosphere",
        "fx-effects" => "06-fx-effects",
        "color-texture" => "07-color-texture",
        "emotion-mood" => "08-emotion-mood",
        "camera-lens" => "09-camera-lens",
        "model-constraints" => "10-model-constraints",
        _ => dimension,
    }
}

fn parse_frontmatter(content: &str) -> Option<AtlasFrontmatter> {
    let rest = content.trim_start().strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    serde_yaml::from_str(&rest[..end]).ok()
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri\Cargo.toml atlas::repository`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/atlas
git commit -m "feat: add atlas repository"
```

---

### Task 6: 暴露 Tauri 命令给前端

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/commands/atlas_commands.rs`

**Interfaces:**
- Consumes: `AtlasRepository`, `AppServices`, `StartupGate`。
- Produces: `load_atlas_entries()`, `load_atlas_entry(id)`, `open_atlas_folder()`。

- [ ] **Step 1: 注册模块**

在 `src-tauri/src/lib.rs` 增加：

```rust
mod atlas;
```

在 `src-tauri/src/commands.rs` 增加：

```rust
pub(crate) mod atlas_commands;
```

- [ ] **Step 2: 写命令**

```rust
use crate::{
    app_state::{AppServices, StartupGate},
    atlas::repository::{AtlasEntryDto, AtlasRepository},
    command_auth::MainArgs,
};
use tauri::Manager;

fn atlas_root() -> Result<std::path::PathBuf, String> {
    let home = dirs_home_dir().ok_or_else(|| "HOME_NOT_FOUND".to_string())?;
    let config = home.join(".banana-box").join("atlas-config.json");
    let root = if config.exists() {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(config).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        std::path::PathBuf::from(
            value
                .get("rootPath")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
        )
    } else {
        home.join("Documents").join("AAA-Aesthetic-Atlas")
    };
    Ok(root)
}

fn dirs_home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
}

#[tauri::command]
pub fn load_atlas_entries(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    _args: MainArgs<EmptyAtlasArgs>,
) -> Result<Vec<AtlasEntryDto>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.list_entries()
}

#[tauri::command]
pub fn load_atlas_entry(
    _window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    id: String,
) -> Result<Option<String>, String> {
    gate.require_ready()?;
    let repo = AtlasRepository::new(atlas_root()?);
    repo.get_entry(&id)
}

#[tauri::command]
pub fn open_atlas_folder(
    app: tauri::AppHandle,
    _gate: tauri::State<'_, StartupGate>,
) -> Result<(), String> {
    let root = atlas_root()?;
    app.opener().open_path(root.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyAtlasArgs {}
```

- [ ] **Step 3: 注册命令到 invoke handler**

在 `lib.rs` 的 `generate_handler!` 中追加：

```rust
atlas_commands::load_atlas_entries,
atlas_commands::load_atlas_entry,
atlas_commands::open_atlas_folder,
```

- [ ] **Step 4: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri\Cargo.toml`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src
git commit -m "feat: expose atlas commands"
```

---

### Task 7: 实现 Banana Box 本地 HTTP 入库服务

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/atlas/http_service.rs`

**Interfaces:**
- Consumes: `ProviderService`, `ProviderHttpClient`, `AtlasRepository`。
- Produces: `AtlasHttpService::start(root, token) -> JoinHandle<()>`；`stop()`；`/v1/health`；`/v1/ingest`。

- [ ] **Step 1: 添加 tiny_http 依赖**

```toml
tiny_http = "0.12"
```

- [ ] **Step 2: 实现服务结构**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct AtlasHttpService {
    running: Arc<AtomicBool>,
}

impl AtlasHttpService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, root: std::path::PathBuf, token: String) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let running = self.running.clone();
        std::thread::spawn(move || {
            let server = match tiny_http::Server::http("127.0.0.1:41773") {
                Ok(server) => server,
                Err(_) => {
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };
            while running.load(Ordering::SeqCst) {
                if let Ok(mut request) = server.recv() {
                    let path = request.url().to_string();
                    let response = if path == "/v1/health" {
                        tiny_http::Response::from_string("ok")
                    } else {
                        tiny_http::Response::from_string("not_found").with_status_code(404)
                    };
                    let _ = request.respond(response);
                }
            }
        });
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
```

- [ ] **Step 3: 补全 ingest 请求处理**

在循环中处理 `POST /v1/ingest`：

```rust
if path == "/v1/ingest" && request.method() == &tiny_http::Method::Post {
    let mut body = String::new();
    let _ = request.as_reader().read_to_string(&mut body);
    let payload: serde_json::Value = match serde_json::from_str(&body) {
        Ok(value) => value,
        Err(_) => {
            let _ = request.respond(tiny_http::Response::from_string("invalid_json").with_status_code(400));
            continue;
        }
    };
    let _ = request.respond(tiny_http::Response::from_string("queued"));
}
```

- [ ] **Step 4: 创建 ingest 文件操作函数**

Create `src-tauri/src/atlas/ingest.rs`：

```rust
use std::path::Path;

pub fn compress_image_to_webp(bytes: &[u8], max_dimension: u32) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let resized = image.resize(max_dimension, max_dimension, image::imageops::FilterType::Lanczos3);
    let mut output = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut output, image::ImageFormat::WebP).map_err(|e| e.to_string())?;
    Ok(output.into_inner())
}

pub fn save_ingested_asset(root: &Path, dimension: &str, entry_id: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = root.join("assets").join(dimension);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{entry_id}.webp"));
    let tmp = dir.join(format!("{entry_id}.webp.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().replace('\\', "/"))
}

pub fn write_entry_md(root: &Path, entry_id: &str, dimension: &str, frontmatter: &str, body: &str) -> Result<(), String> {
    let dir = root.join("entries").join(dimension);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{entry_id}.md"));
    let tmp = dir.join(format!("{entry_id}.md.tmp"));
    std::fs::write(&tmp, format!("---\n{frontmatter}\n---\n\n{body}")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn build_frontmatter(entry_id: &str, title: &str, dimension: &str, tags: &[String], status: &str) -> String {
    format!(
        "id: {entry_id}\ntitle: {title}\ndimension: {dimension}\ntags: {tags:?}\nstatus: {status}\n"
    )
}
```

- [ ] **Step 5: 写 ingest 文件操作测试**

在 `src-tauri/src/atlas/ingest.rs` 增加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_entry_md_creates_atomic_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_entry_md(root, "a", "05-lighting-atmosphere", "id: a\ntitle: 测试", "## 提示词片段\n逆光").unwrap();
        assert!(root.join("entries/05-lighting-atmosphere/a.md").exists());
        assert!(!root.join("entries/05-lighting-atmosphere/a.md.tmp").exists());
    }
}
```

- [ ] **Step 6: 接入 HTTP ingest 入口**

在 `http_service.rs` 中调用：

```rust
let dimension = payload.get("dimension").and_then(|v| v.as_str()).unwrap_or("05-lighting-atmosphere");
let entry_id = format!("atlas-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
let bytes = std::fs::read(local_path).map_err(|e| e.to_string())?;
let compressed = crate::atlas::ingest::compress_image_to_webp(&bytes, 1280)?;
crate::atlas::ingest::save_ingested_asset(&root, dimension, &entry_id, &compressed)?;
crate::atlas::ingest::write_entry_md(&root, &entry_id, dimension, "id: {entry_id}\ntitle: pending\ndimension: {dimension}\ntags: []\nstatus: pending_review", "## 分析\n待识图")?;
```

- [ ] **Step 7: 运行 Rust 测试**

Run: `cargo test --manifest-path src-tauri\Cargo.toml atlas`
Expected: PASS。

- [ ] **Step 8: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/atlas
git commit -m "feat: add atlas local http service"
```

---

### Task 8: 添加前端类型、IPC 和 store

**Files:**
- Create: `src/types/atlas.ts`
- Create: `src/lib/atlas-ipc.ts`
- Create: `src/stores/atlas.ts`
- Test: `tests/stores/atlas.test.ts`

**Interfaces:**
- Consumes: Tauri commands from Task 6。
- Produces: `AtlasEntry`、`loadAtlasEntries()`、`loadAtlasEntry(id)`、`openAtlasFolder()`、`useAtlasStore`。

- [ ] **Step 1: 写失败测试**

```ts
import { describe, it, expect } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAtlasStore } from '@/stores/atlas'

describe('atlas store', () => {
  it('hydrates entries', () => {
    setActivePinia(createPinia())
    const store = useAtlasStore()
    store.hydrate([{ id: 'a', title: '测试', dimension: 'lighting-atmosphere', tags: [], image: '', score: 1, status: 'confirmed', file: 'entries/a.md' }])
    expect(store.filteredEntries).toHaveLength(1)
  })
})
```

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm vitest run tests/stores/atlas.test.ts`
Expected: FAIL，模块不存在。

- [ ] **Step 3: 实现类型**

```ts
export interface AtlasEntry {
  id: string
  title: string
  dimension: string
  tags: string[]
  image: string
  score: number
  status: 'confirmed' | 'pending_review'
  file: string
}
```

- [ ] **Step 4: 实现 IPC**

```ts
import { invoke } from '@tauri-apps/api/core'
import type { AtlasEntry } from '@/types/atlas'

export async function loadAtlasEntries(): Promise<AtlasEntry[]> {
  return await invoke<AtlasEntry[]>('load_atlas_entries', {})
}

export async function loadAtlasEntry(id: string): Promise<string | null> {
  return await invoke<string | null>('load_atlas_entry', { id })
}

export async function openAtlasFolder(): Promise<void> {
  await invoke('open_atlas_folder')
}
```

- [ ] **Step 5: 实现 store**

```ts
import { defineStore } from 'pinia'
import type { AtlasEntry } from '@/types/atlas'
import * as atlasIpc from '@/lib/atlas-ipc'

export const useAtlasStore = defineStore('atlas', {
  state: () => ({
    entries: [] as AtlasEntry[],
    selectedId: null as string | null,
    search: '' as string,
    dimension: null as string | null,
    loaded: false,
  }),
  getters: {
    filteredEntries(state): AtlasEntry[] {
      const keyword = state.search.trim().toLowerCase()
      return state.entries
        .filter((entry) => !state.dimension || entry.dimension === state.dimension)
        .filter((entry) => !keyword || `${entry.title} ${entry.tags.join(' ')}`.toLowerCase().includes(keyword))
        .sort((a, b) => b.score - a.score)
    },
    selectedEntry(state): AtlasEntry | null {
      return state.entries.find((entry) => entry.id === state.selectedId) ?? null
    },
  },
  actions: {
    hydrate(entries: AtlasEntry[]) {
      this.entries = entries
      this.loaded = true
    },
    async load() {
      this.entries = await atlasIpc.loadAtlasEntries()
      this.loaded = true
    },
    select(id: string) {
      this.selectedId = id
    },
  },
})
```

- [ ] **Step 6: 运行测试确认通过**

Run: `pnpm vitest run tests/stores/atlas.test.ts`
Expected: PASS。

- [ ] **Step 7: 提交**

```bash
git add src/types/atlas.ts src/lib/atlas-ipc.ts src/stores/atlas.ts tests/stores/atlas.test.ts
git commit -m "feat: add atlas frontend store"
```

---

### Task 9: 添加 Banana Box 参考库页面

**Files:**
- Create: `src/components/atlas/AtlasLibraryPage.vue`
- Modify: `src/stores/ui.ts`
- Modify: `src/components/AppSidebar.vue`
- Modify: `src/App.vue`
- Test: `tests/components/AtlasLibraryPage.test.ts`

**Interfaces:**
- Consumes: `useAtlasStore`, `useUiStore`。
- Produces: `atlas` ActiveTool 页面。

- [ ] **Step 1: 扩展 ActiveTool**

在 `src/stores/ui.ts`：

```ts
export type ActiveTool =
  | 'atlas'
  | 'shared-library'
  | 'prompts'
  | 'reverse-image'
  | 'compression'
  | 'depth-video'
  | 'projects'
  | 'daily-tasks'
  | 'storyboard'
  | 'pi-web'
```

- [ ] **Step 2: 增加侧边栏入口**

在 `src/components/AppSidebar.vue` 的 `tools` 数组增加：

```ts
{ id: 'atlas', label: '参考库' }
```

- [ ] **Step 3: 挂载页面**

在 `src/App.vue` 中：

```vue
<AtlasLibraryPage v-else-if="ui.activeTool === 'atlas'" />
```

- [ ] **Step 4: 写页面**

```vue
<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useAtlasStore } from '@/stores/atlas'
import { openAtlasFolder } from '@/lib/atlas-ipc'

const atlas = useAtlasStore()

onMounted(async () => {
  await atlas.load()
})

const dimensions = [
  'scene-concept',
  'style',
  'character-pose',
  'composition',
  'lighting-atmosphere',
  'fx-effects',
  'color-texture',
  'emotion-mood',
  'camera-lens',
  'model-constraints',
]
</script>

<template>
  <section class="atlas-page">
    <header class="atlas-toolbar">
      <button type="button" @click="openAtlasFolder">打开知识库文件夹</button>
      <input v-model="atlas.search" placeholder="搜索标题或标签" />
    </header>
    <div class="atlas-body">
      <aside class="atlas-sidebar">
        <button type="button" @click="atlas.dimension = null">全部</button>
        <button
          v-for="dimension in dimensions"
          :key="dimension"
          type="button"
          @click="atlas.dimension = dimension"
        >
          {{ dimension }}
        </button>
      </aside>
      <main class="atlas-grid">
        <button
          v-for="entry in atlas.filteredEntries"
          :key="entry.id"
          type="button"
          class="atlas-card"
          @click="atlas.select(entry.id)"
        >
          <img v-if="entry.image" :src="`http://127.0.0.1:1422/${entry.image}`" />
          <strong>{{ entry.title }}</strong>
        </button>
      </main>
      <aside class="atlas-detail">
        <template v-if="atlas.selectedEntry">
          <strong>{{ atlas.selectedEntry.title }}</strong>
          <span>{{ atlas.selectedEntry.status }}</span>
        </template>
      </aside>
    </div>
  </section>
</template>
```

- [ ] **Step 5: 运行组件测试**

Run: `pnpm vitest run tests/components/AtlasLibraryPage.test.ts`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src/components/atlas src/stores/ui.ts src/components/AppSidebar.vue src/App.vue tests/components/AtlasLibraryPage.test.ts
git commit -m "feat: add atlas library page"
```

---

### Task 10: 接入悬浮窗拖图入库

**Files:**
- Modify: `src/components/FloatingActionDialog.vue`
- Modify: `src/stores/ui.ts`
- Create: `src/lib/atlas-ingest-ipc.ts`

**Interfaces:**
- Consumes: 现有 `FloatingActionFile`。
- Produces: `ingestAtlasImage(input: AtlasIngestInput)`；悬浮窗新增“存入审美参考库” action。

- [ ] **Step 1: 写 IPC**

```ts
import { invoke } from '@tauri-apps/api/core'

export interface AtlasIngestInput {
  localPath: string
  title?: string
  dimension?: string
  tags?: string[]
}

export async function ingestAtlasImage(input: AtlasIngestInput): Promise<void> {
  await invoke('ingest_atlas_image', { input })
}
```

- [ ] **Step 2: 增加浮窗按钮**

在 `FloatingActionDialog.vue` 的图片操作区增加：

```vue
<button
  type="button"
  class="action-button"
  data-action="atlas-ingest"
  @click="openAtlasIngest"
>
  <strong>存入审美参考库</strong>
  <span>识图、分类并写入 AAA-Aesthetic-Atlas</span>
</button>
```

```ts
async function openAtlasIngest() {
  if (!ui.floatingActionFile) return
  await ingestAtlasImage({ localPath: ui.floatingActionFile.filePath })
  ui.closeFloatingActionDialog()
  ui.showToast('已提交审美参考库入库')
}
```

- [ ] **Step 3: 运行前端检查**

Run: `pnpm typecheck && pnpm test`
Expected: PASS。

- [ ] **Step 4: 提交**

```bash
git add src/components/FloatingActionDialog.vue src/stores/ui.ts src/lib/atlas-ingest-ipc.ts
git commit -m "feat: add atlas ingest floating action"
```

---

### Task 11: 实现 Chrome 插件 MVP

**Files:**
- Create: `chrome-extension/manifest.json`
- Create: `chrome-extension/background.js`
- Create: `chrome-extension/content.js`
- Create: `chrome-extension/popup.html`
- Create: `chrome-extension/popup.js`

**Interfaces:**
- Consumes: `http://127.0.0.1:41773/v1/ingest`。
- Produces: 右键“分析入库”菜单；插件弹窗显示最后状态。

- [ ] **Step 1: 写 manifest**

```json
{
  "manifest_version": 3,
  "name": "AAA-Aesthetic-Atlas Capture",
  "version": "0.1.0",
  "permissions": ["contextMenus", "activeTab", "scripting"],
  "host_permissions": ["http://127.0.0.1/*"],
  "background": {
    "service_worker": "background.js"
  },
  "action": {
    "default_popup": "popup.html"
  }
}
```

- [ ] **Step 2: 写 background**

```js
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: 'atlas-ingest-image',
    title: '分析入库到 AAA-Aesthetic-Atlas',
    contexts: ['image'],
  })
})

chrome.contextMenus.onClicked.addListener(async (info) => {
  if (info.menuItemId === 'atlas-ingest-image' && info.srcUrl) {
    await fetch('http://127.0.0.1:41773/v1/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sourceUrl: info.srcUrl }),
    })
  }
})
```

- [ ] **Step 3: 写弹窗**

```html
<!doctype html>
<html>
  <body>
    <button id="capture">抓取当前网页图片</button>
    <pre id="status"></pre>
    <script src="popup.js"></script>
  </body>
</html>
```

```js
document.getElementById('capture').addEventListener('click', async () => {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
  await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    files: ['content.js'],
  })
})
```

- [ ] **Step 4: 手工测试**

在 Chrome 打开 `chrome://extensions`，启用开发者模式，加载 `chrome-extension` 文件夹，右键任意图片验证菜单。

- [ ] **Step 5: 提交**

```bash
git add chrome-extension
git commit -m "feat: add atlas chrome extension mvp"
```

---

### Task 12: 端到端验收

**Files:**
- Create: `docs/superpowers/verification/2026-09-03-atlas-e2e.md`

- [ ] **Step 1: 初始化知识库**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\init_atlas.py`
Expected: 创建 `Documents\AAA-Aesthetic-Atlas`。

- [ ] **Step 2: 启动 Banana Box 并打开参考库**

Run: `pnpm tauri dev`
Expected: 侧边栏出现“参考库”，页面显示空网格。

- [ ] **Step 3: 拖入图片**

Expected: 浮窗出现“存入审美参考库”，点击后自动压缩、识图、写入 `.md` 和 `index.json`。

- [ ] **Step 4: MCP 查询**

Run: `python C:\Users\admin\.codex\skills\AAA-Aesthetic-Atlas\scripts\aaa_atlas_mcp.py`
Expected: MCP 服务可被 Codex 调用，`get_entry` 返回刚入库的条目。

- [ ] **Step 5: Chrome 插件**

Expected: 右键网页图片能提交给 Banana Box 本地服务并生成新条目。

- [ ] **Step 6: 并发安全**

Expected: Banana Box 写入时，MCP 多次读取 `index.json` 不出现损坏或空数据。

- [ ] **Step 7: 记录结果并提交**

```bash
git add docs/superpowers/verification/2026-09-03-atlas-e2e.md
git commit -m "docs: record atlas end to end verification"
```
