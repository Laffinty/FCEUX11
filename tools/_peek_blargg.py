import json

# blargg_result.json structure
try:
    with open("blargg_result.json", encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, dict):
        print("keys:", list(data.keys())[:10])
        for k, v in data.items():
            if isinstance(v, list):
                print(f"{k}: list of {len(v)}")
            else:
                print(f"{k}: {v}")
    elif isinstance(data, list):
        print("list of", len(data))
        if data:
            print("first item:", json.dumps(data[0])[:300])
except Exception as e:
    print("parse error:", e)
