"""CakeGame 全功能测试 v3 - 正确注册用户"""
import ctypes
import os

LIB_PATH = os.path.join(os.path.dirname(__file__), "target/release/libcakegame.so")
DB_PATH = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb"

lib = ctypes.CDLL(LIB_PATH)
lib.cg_init.argtypes = [ctypes.c_char_p]
lib.cg_init.restype = ctypes.c_int
lib.cg_process.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.cg_process.restype = ctypes.c_void_p
lib.cg_free_string.argtypes = [ctypes.c_void_p]
lib.cg_free_string.restype = None

def read_cstr(ptr):
    if ptr is None: return ""
    result = ctypes.cast(ptr, ctypes.c_char_p).value
    lib.cg_free_string(ptr)
    if result is None: return ""
    return result.decode("utf-8", errors="replace")

def cmd(msg, uid):
    ptr = lib.cg_process(msg.encode(), b"2", b"0", uid.encode())
    return read_cstr(ptr)

ret = lib.cg_init(DB_PATH.encode())
print(f"引擎初始化: {'OK' if ret == 0 else 'FAIL'} (code={ret})")

passed = failed = total = 0

def test(name, msg, uid, expect=None, not_empty=True):
    global passed, failed, total
    total += 1
    result = cmd(msg, uid)
    ok = True
    issues = []
    if not_empty and not result.strip():
        ok = False; issues.append("空返回")
    if expect:
        for s in (expect if isinstance(expect, list) else [expect]):
            if s not in result:
                ok = False; issues.append(f"无'{s}'")
    if ok:
        passed += 1
        print(f"  OK {name}")
    else:
        failed += 1
        print(f"  FAIL {name}: {', '.join(issues)}")
        print(f"     -> {result[:200]}")
    return result

# 预注册所有测试用户
print("\n===== 注册测试用户 =====")
for uid, nick in [
    ("t1_base", "基础测试"), ("t1_base2", "基础测试2"),
    ("t6_task", "任务测试"), ("t6_task2", "任务测试2"),
    ("t7_pvp", "PVP测试A"), ("t7_target", "PVP测试B"),
    ("t8_guild", "公会测试"), ("t8_guild2", "公会测试2"),
    ("t9_team", "队伍测试"), ("t9_team2", "队伍测试2"),
    ("t10_shop", "商店测试"), ("t10_shop2", "商店测试2"),
    ("t12_gm", "GM测试"),
]:
    r = cmd(f"注册+{nick}", uid)
    if "成功" in r or "已存在" in r:
        print(f"  OK 注册 {uid} ({nick})")
    else:
        print(f"  INFO {uid}: {r[:80]}")

# ==================== 1. 基础系统 ====================
print("\n===== 1. 基础系统 =====")
test("重复注册", "注册+测试", "t1_base", ["已存在", "已经注册"])
test("签到", "签到", "t1_base", "签到成功")
test("重复签到", "签到", "t1_base", "已经签到")
test("查看角色", "查看角色", "t1_base", "昵称")
test("帮助", "帮助", "t1_base", "帮助")
test("修改昵称", "修改昵称+新名ABC", "t1_base2", "昵称已修改")

# ==================== 2. 物品系统 ====================
print("\n===== 2. 物品系统 =====")
test("查看背包", "查看背包", "t1_base")
test("查看装备", "查看装备", "t1_base")
test("查看技能", "查看技能", "t1_base")
test("卸下无装备", "卸下+武器", "t1_base")
test("使用不存在物品", "使用+不存在物品XYZ", "t1_base")
test("丢弃不存在物品", "丢弃+不存在物品XYZ", "t1_base")

# ==================== 3. 地图系统 ====================
print("\n===== 3. 地图系统 =====")
test("查看地图", "查看地图", "t1_base")
test("位置信息", "位置信息", "t1_base")
test("进入地图", "进入+主城", "t1_base", ["主城", "位置", "进入"])
test("进入不存在地图", "进入+不存在地图XYZ", "t1_base")

# ==================== 4. 战斗系统 ====================
print("\n===== 4. 战斗系统 =====")
test("搜索怪物", "搜索怪物", "t1_base")
test("锁定怪物", "锁定目标+史莱姆", "t1_base", "锁定")
test("攻击", "攻击", "t1_base")
test("解锁目标", "解锁目标", "t1_base")
test("释放技能", "释放技能+初级疗伤", "t1_base")
test("自动攻击", "自动攻击", "t1_base")

# ==================== 5. 商店系统 ====================
print("\n===== 5. 商店系统 =====")
test("查看商店", "查看商店", "t1_base")
test("购买不存在商品", "购买+不存在商品XYZ", "t1_base")
test("合成列表", "合成列表", "t1_base")

# ==================== 6. 任务系统 ====================
print("\n===== 6. 任务系统 =====")
# 升级用户
cmd("发放奖励+t6_task+经验+100000", "t6_task")
cmd("发放奖励+t6_task2+经验+100000", "t6_task2")

test("我的任务", "我的任务", "t6_task")
test("我的任务第2页", "我的任务+2", "t6_task")
test("任务信息-无任务", "任务信息", "t6_task", "还未领取")
test("领取不存在任务", "领取任务+不存在任务XYZ", "t6_task", "不存在")
test("提交-无任务", "提交任务", "t6_task", "还未领取")
test("放弃-无任务", "放弃任务", "t6_task", "还未领取")

r = cmd("我的任务", "t6_task")
print(f"  可接: {r[:200]}")
r2 = cmd("领取任务+1", "t6_task")
print(f"  领取: {r2[:200]}")
if "成功领取" in r2:
    test("查看当前任务", "任务信息", "t6_task", "任务")
    test("重复领取", "领取任务+2", "t6_task", ["请先完成", "不存在", "暂未开放"])
    test("提交任务", "提交任务", "t6_task")
    test("放弃任务", "放弃任务", "t6_task")
else:
    # 可能没有合适等级的任务
    r3 = cmd("领取任务+初出茅庐", "t6_task")
    print(f"  领取指定: {r3[:200]}")

# ==================== 7. PVP系统 ====================
print("\n===== 7. PVP系统 =====")
test("锁定不存在玩家", "锁定玩家+999999999", "t7_pvp", "未注册")
test("锁定自己", "锁定玩家+t7_pvp", "t7_pvp", "不能锁定自己")
test("攻击无目标", "攻击玩家", "t7_pvp", "请先锁定")

r = cmd("锁定玩家+t7_target", "t7_pvp")
print(f"  锁定: {r[:200]}")
if "成功锁定" in r:
    test("攻击玩家", "攻击玩家", "t7_pvp", ["PVP", "回合"])
else:
    print(f"  跳过攻击测试: {r[:100]}")

# ==================== 8. 公会系统 ====================
print("\n===== 8. 公会系统 =====")
test("公会列表", "公会列表", "t8_guild")
test("我的公会-无", "我的公会", "t8_guild")
r = cmd("创建公会+测试公会", "t8_guild")
print(f"  创建: {r[:200]}")
test("创建公会", "创建公会+测试公会", "t8_guild", ["成功", "已存在", "已经"])
test("我的公会-有", "我的公会", "t8_guild", "测试公会")
test("解散公会", "解散公会", "t8_guild", ["解散", "未加入"])

# ==================== 9. 队伍系统 ====================
print("\n===== 9. 队伍系统 =====")
r = cmd("创建队伍", "t9_team")
print(f"  创建: {r[:200]}")
test("创建队伍", "创建队伍", "t9_team", ["成功", "已经"])
test("队伍成员", "队伍成员", "t9_team")
test("退出队伍", "退出队伍", "t9_team", "退出")

# ==================== 10. 私人商店 ====================
print("\n===== 10. 私人商店 =====")
test("商店列表", "商店列表", "t10_shop")
test("我的商店-无", "我的商店", "t10_shop")
test("商店改名", "商店改名+测试商店", "t10_shop", "改名")
test("我的商店", "我的商店", "t10_shop", "测试商店")
test("开启商店", "开启商店", "t10_shop", "开启")
test("商店列表-有", "商店列表", "t10_shop")
test("关闭商店", "关闭商店", "t10_shop", "关闭")
test("上架无物品", "上架商品+不存在物品*100*金币", "t10_shop", "没有")
test("下架空商店", "下架商品+1", "t10_shop")
test("进入不存在商店", "进入商店+999", "t10_shop", "不存在")
test("退出商店", "退出商店", "t10_shop", "退出")

# ==================== 11. 排行榜 ====================
print("\n===== 11. 排行榜 =====")
for rt in ["等级", "生命", "魔法", "物攻", "魔攻", "防御", "魔抗", "金币", "钻石"]:
    test(f"{rt}排行", f"{rt}排行", "t12_gm")

# ==================== 12. GM指令 ====================
print("\n===== 12. GM指令 =====")
test("发放金币", "发放奖励+t12_gm+金币+500", "t12_gm")
test("发放经验", "发放奖励+t12_gm+经验+1000", "t12_gm")

# ==================== 总结 ====================
print(f"\n{'='*50}")
print(f"结果: {passed}/{total} 通过, {failed} 失败 ({passed*100//total}%)")
print(f"{'='*50}")
