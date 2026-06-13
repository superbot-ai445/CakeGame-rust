#!/usr/bin/env python3
"""Quick batch test for CakeGame HF Space"""
import urllib.request, json, urllib.parse

BASE = "https://smithjacki-hermes.hf.space"
def api_cmd(msg, uid="checker"):
    url = BASE + "/api/cmd?" + urllib.parse.urlencode({"msg": msg, "uid": uid})
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "test"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:
        return {"error": str(e)}

tests = [
    ("健康检查", lambda: json.loads(urllib.request.urlopen(BASE + "/api/health", timeout=10).read())),
    ("注册", lambda: api_cmd("注册+检查员", "chk_002")),
    ("签到", lambda: api_cmd("签到", "chk_002")),
    ("查看角色", lambda: api_cmd("查看角色", "chk_002")),
    ("帮助", lambda: api_cmd("帮助", "chk_002")),
    ("查看背包", lambda: api_cmd("查看背包", "chk_002")),
    ("查看装备", lambda: api_cmd("查看装备", "chk_002")),
    ("查看技能", lambda: api_cmd("查看技能", "chk_002")),
    ("查看地图", lambda: api_cmd("查看地图", "chk_002")),
    ("搜索怪物", lambda: api_cmd("搜索怪物", "chk_002")),
    ("攻击", lambda: api_cmd("攻击", "chk_002")),
    ("属性重置", lambda: api_cmd("属性重置", "chk_002")),
    ("重置记录", lambda: api_cmd("重置记录", "chk_002")),
    ("查看特技", lambda: api_cmd("查看特技", "chk_002")),
    ("查看增益", lambda: api_cmd("查看增益", "chk_002")),
    ("战力", lambda: api_cmd("战力", "chk_002")),
    ("全服统计", lambda: api_cmd("全服统计", "chk_002")),
    ("离线收益", lambda: api_cmd("离线收益", "chk_002")),
]

passed = 0
failed = 0
for name, test_fn in tests:
    try:
        result = test_fn()
        if isinstance(result, dict):
            r = result.get("result", "")
            ok = result.get("ok", False) and r != "（无返回）"
        else:
            ok = True
        status = "✅" if ok else "⚠️"
        if ok:
            passed += 1
        else:
            failed += 1
        display = str(result.get("result", ""))[:80] if isinstance(result, dict) else str(result)[:80]
        print(f"{status} {name}: {display}")
    except Exception as e:
        failed += 1
        print(f"❌ {name}: {e}")

print(f"\n=== 结果: {passed}/{passed+failed} 通过 ({100*passed//(passed+failed)}%) ===")
