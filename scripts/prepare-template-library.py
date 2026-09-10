import json
import os
import sys
from pathlib import Path
from PIL import Image

SRC = Path("C:/Users/admin/AppData/Local/Temp/awesome-gpt-image-2-review-a45e62befbaa41a7be3068cafbb67f4a")
OUT = Path("C:/Users/admin/banana-box-main/src-tauri/resources/template-library")
OUT.mkdir(parents=True, exist_ok=True)
(OUT / "images").mkdir(parents=True, exist_ok=True)

MAX_EDGE = 720
QUALITY = 80

cases = json.loads((SRC / "data" / "cases.json").read_text(encoding="utf-8"))
styles = json.loads((SRC / "data" / "style-library.json").read_text(encoding="utf-8"))

# category value -> zh title
zh_by_value = {}
for cat in styles["categories"]:
    zh_by_value[cat["value"]] = cat["title"]["zh"]

categories = []
for cat in styles["categories"]:
    categories.append({
        "id": cat["id"],
        "name": cat["title"]["zh"],
        "value": cat["value"],
    })

records = []
missing = []
converted = 0
for case in cases["cases"]:
    src_img_name = case.get("image", "").lstrip("/").replace("images/", "")
    src_img = SRC / "data" / "images" / src_img_name
    out_img_name = None
    if src_img.exists():
        try:
            im = Image.open(src_img)
            im = im.convert("RGB")
            w, h = im.size
            scale = MAX_EDGE / max(w, h)
            if scale < 1:
                im = im.resize((max(1, int(w * scale)), max(1, int(h * scale))))
            out_img_name = f"case{case['id']}.webp"
            im.save(OUT / "images" / out_img_name, "WEBP", quality=QUALITY, method=4)
            converted += 1
        except Exception as exc:
            missing.append((case["id"], str(exc)))
            out_img_name = None
    else:
        missing.append((case["id"], "missing source image"))

    tags = []
    for t in case.get("styles", []) + case.get("scenes", []):
        if t not in tags:
            tags.append(t)

    records.append({
        "id": case["id"],
        "title": case.get("title", ""),
        "category": case.get("category", "Other Use Cases"),
        "image": f"images/{out_img_name}" if out_img_name else "",
        "tags": tags,
        "promptEn": case.get("prompt", ""),
        "promptZh": "",
        "sourceLabel": case.get("sourceLabel", ""),
        "sourceUrl": case.get("sourceUrl", ""),
    })

payload = {
    "version": 1,
    "categories": categories,
    "cases": records,
}

out_json = OUT / "cases.json"
out_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")

print("cases:", len(records))
print("converted images:", converted)
print("missing/failed:", len(missing))
for item in missing[:20]:
    print("  ", item)
print("output json:", out_json)
print("output bytes:", out_json.stat().st_size)
imgs = [p for p in (OUT / "images").iterdir()]
print("image files:", len(imgs))
print("images total MB:", round(sum(p.stat().st_size for p in imgs) / 1024 / 1024, 2))
