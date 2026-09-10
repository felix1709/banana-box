import json
import os
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

PATH = "C:/Users/admin/banana-box-main/src-tauri/resources/template-library/cases.json"
API_URL = "https://ai.leihuo.netease.com/v1/chat/completions"
MODEL = "doubao-seed-1-6-vision-250815"
SYSTEM = (
    "你是一名专业的 AI 生图提示词翻译。把英文提示词翻译成准确、自然、可直接用于生图的中文提示词。"
    "保留方括号占位符（如 [FRUIT]）和专有名词；不要添加解释，只输出翻译后的提示词。"
)

data = json.loads(open(PATH, encoding="utf-8").read())
cases = data["cases"]
api_key = os.environ.get("LEIHUO_VISION_API_KEY")
if not api_key:
    raise SystemExit("缺少 LEIHUO_VISION_API_KEY")


def translate_one(item):
    prompt = item["promptEn"]
    if not prompt.strip():
        return item["id"], ""
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": prompt},
        ],
    }
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": "Bearer " + api_key, "Content-Type": "application/json"},
    )
    for attempt in range(2):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                body = json.loads(resp.read().decode("utf-8"))
            return item["id"], body["choices"][0]["message"]["content"].strip()
        except Exception as exc:
            if attempt == 0:
                time.sleep(1.2)
                continue
            return item["id"], ""
    return item["id"], ""


todo = [c for c in cases if not (c.get("promptZh") or "").strip()]
print("total:", len(cases), "todo:", len(todo))

done = 0
failed = 0
start = time.time()

with ThreadPoolExecutor(max_workers=12) as pool:
    futures = {pool.submit(translate_one, item): item for item in todo}
    for future in as_completed(futures):
        cid, text = future.result()
        item = futures[future]
        item["promptZh"] = text
        done += 1
        if not text:
            failed += 1
        if done % 20 == 0 or done == len(todo):
            elapsed = time.time() - start
            print(f"done {done}/{len(todo)} failed {failed} elapsed {int(elapsed)}s", flush=True)
            open(PATH, "w", encoding="utf-8").write(json.dumps(data, ensure_ascii=False, indent=2))

open(PATH, "w", encoding="utf-8").write(json.dumps(data, ensure_ascii=False, indent=2))
print("written", PATH)
print("empty zh:", sum(1 for c in cases if not (c.get('promptZh') or '').strip()))
