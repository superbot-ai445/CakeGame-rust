#!/usr/bin/env python3
"""CakeGame-RS Gradio 调试器 v3
基于 Rust 引擎的 FFI 接口，提供现代化 Web 调试界面
"""

import ctypes
import json
import sqlite3
import os
import sys
import gradio as gr

# ============== 配置 ==============
LIB_PATH = "/opt/data/cakegame-rs/target/release/libcakegame.so"
DB_PATH = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb"
SAVE_PATH = "/opt/data/cakegame-rs/debug_save.json"

# ============== 加载 Rust 库 ==============
try:
    lib = ctypes.CDLL(LIB_PATH)
except OSError as e:
    print(f"❌ 无法加载 Rust 库: {e}")
    print(f"   请确保已编译: cd cakegame-rs && cargo build --release")
    sys.exit(1)

# FFI 函数签名定义
lib.Cake_Initialize.restype = ctypes.c_int
lib.Cake_Initialize.argtypes = []

lib.Cake_GetVersion.restype = ctypes.c_int
lib.Cake_GetVersion.argtypes = []

lib.Cake_GetVersionString.restype = ctypes.c_char_p
lib.Cake_GetVersionString.argtypes = []

lib.Cake_Cleanup.restype = None
lib.Cake_Cleanup.argtypes = []

lib.Cake_FreeString.restype = None
lib.Cake_FreeString.argtypes = [ctypes.c_void_p]

lib.Cake_LoadDatabase.restype = ctypes.c_int
lib.Cake_LoadDatabase.argtypes = [ctypes.c_char_p]

lib.Cake_CreateCharacter.restype = ctypes.c_int
lib.Cake_CreateCharacter.argtypes = [ctypes.c_char_p, ctypes.c_char_p]

lib.Cake_MoveTo.restype = ctypes.c_int
lib.Cake_MoveTo.argtypes = [ctypes.c_char_p]

lib.Cake_StartCombat.restype = ctypes.c_int
lib.Cake_StartCombat.argtypes = [ctypes.c_char_p]

lib.Cake_Attack.restype = ctypes.c_int
lib.Cake_Attack.argtypes = []

lib.Cake_AddItem.restype = ctypes.c_int
lib.Cake_AddItem.argtypes = [ctypes.c_char_p, ctypes.c_int]

lib.Cake_RemoveItem.restype = ctypes.c_int
lib.Cake_RemoveItem.argtypes = [ctypes.c_char_p, ctypes.c_int]

lib.Cake_GetItemCount.restype = ctypes.c_int
lib.Cake_GetItemCount.argtypes = [ctypes.c_char_p]

lib.Cake_UseItem.restype = ctypes.c_int
lib.Cake_UseItem.argtypes = [ctypes.c_char_p]

lib.Cake_EquipItem.restype = ctypes.c_int
lib.Cake_EquipItem.argtypes = [ctypes.c_char_p]

lib.Cake_AcceptTask.restype = ctypes.c_int
lib.Cake_AcceptTask.argtypes = [ctypes.c_char_p]

lib.Cake_CompleteTask.restype = ctypes.c_int
lib.Cake_CompleteTask.argtypes = [ctypes.c_char_p]

lib.Cake_BuyItem.restype = ctypes.c_int
lib.Cake_BuyItem.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_int]

lib.Cake_Composite.restype = ctypes.c_int
lib.Cake_Composite.argtypes = [ctypes.c_char_p]

lib.Cake_Decompose.restype = ctypes.c_int
lib.Cake_Decompose.argtypes = [ctypes.c_char_p]

lib.Cake_EatFood.restype = ctypes.c_int
lib.Cake_EatFood.argtypes = [ctypes.c_char_p]

lib.Cake_LearnSkill.restype = ctypes.c_int
lib.Cake_LearnSkill.argtypes = [ctypes.c_char_p]

lib.Cake_UseSkill.restype = ctypes.c_int
lib.Cake_UseSkill.argtypes = [ctypes.c_char_p]

lib.Cake_TalkToNpc.restype = ctypes.c_int
lib.Cake_TalkToNpc.argtypes = [ctypes.c_char_p]

lib.Cake_SetVar.restype = ctypes.c_int
lib.Cake_SetVar.argtypes = [ctypes.c_char_p, ctypes.c_char_p]

lib.Cake_GetVar.restype = ctypes.c_char_p
lib.Cake_GetVar.argtypes = [ctypes.c_char_p]

lib.Cake_SetFlag.restype = ctypes.c_int
lib.Cake_SetFlag.argtypes = [ctypes.c_char_p]

lib.Cake_HasFlag.restype = ctypes.c_int
lib.Cake_HasFlag.argtypes = [ctypes.c_char_p]

lib.Cake_SaveState.restype = ctypes.c_char_p
lib.Cake_SaveState.argtypes = []

lib.Cake_LoadState.restype = ctypes.c_int
lib.Cake_LoadState.argtypes = [ctypes.c_char_p]

lib.Cake_GetUserInfo.restype = ctypes.c_char_p
lib.Cake_GetUserInfo.argtypes = []

lib.Cake_GetPosition.restype = ctypes.c_char_p
lib.Cake_GetPosition.argtypes = []

lib.Cake_GetHelp.restype = ctypes.c_char_p
lib.Cake_GetHelp.argtypes = [ctypes.c_char_p]

lib.Cake_PollMessage.restype = ctypes.c_int
lib.Cake_PollMessage.argtypes = [
    ctypes.POINTER(ctypes.c_int),
    ctypes.POINTER(ctypes.c_void_p),
    ctypes.POINTER(ctypes.c_void_p)
]


# ============== 辅助函数 ==============
def safe_str(b):
    """安全转换 ctypes 返回的字节串"""
    if b is None:
        return ""
    if isinstance(b, bytes):
        return b.decode("utf-8", errors="replace")
    return str(b)


def free_str(ptr):
    """释放 Rust 分配的字符串"""
    if ptr:
        lib.Cake_FreeString(ptr)


def poll_all_messages():
    """轮询所有待处理消息"""
    messages = []
    msg_type = ctypes.c_int()
    content = ctypes.c_void_p()
    speaker = ctypes.c_void_p()

    type_names = {
        1: "💬 对话", 2: "🔀 选项", 3: "📖 旁白",
        4: "⚔️ 战斗", 5: "📈 经验", 6: "💰 金币",
        7: "📦 物品", 8: "🎉 升级", 9: "⚠️ 系统",
        10: "🗺️ 场景", 11: "❤️ 好感",
    }

    for _ in range(200):  # 最多200条
        result = lib.Cake_PollMessage(
            ctypes.byref(msg_type),
            ctypes.byref(content),
            ctypes.byref(speaker)
        )
        if result == 0:
            break

        content_str = safe_str(content.value)
        speaker_str = safe_str(speaker.value)
        type_name = type_names.get(msg_type.value, f"❓ 类型{msg_type.value}")

        if speaker_str:
            messages.append(f"[{type_name}] {speaker_str}: {content_str}")
        else:
            messages.append(f"[{type_name}] {content_str}")

        free_str(content)
        free_str(speaker)

    return messages


# ============== 游戏数据缓存 ==============
_game_data = {}


def load_game_data():
    """从数据库加载游戏数据供下拉选择"""
    global _game_data
    if _game_data:
        return _game_data

    try:
        conn = sqlite3.connect(DB_PATH)

        # 物品
        rows = conn.execute("SELECT Name, ID FROM Config_Goods").fetchall()
        _game_data["items"] = {safe_str(r[0]): safe_str(r[1]) for r in rows if r[0]}
        _game_data["items_inv"] = {v: k for k, v in _game_data["items"].items()}

        # 怪物
        rows = conn.execute("SELECT Monster_Name FROM Config_Monster").fetchall()
        _game_data["monsters"] = [safe_str(r[0]) for r in rows if r[0]]

        # 地图
        rows = conn.execute("SELECT Name, LV FROM Config_Map").fetchall()
        _game_data["maps"] = [(safe_str(r[0]), safe_str(r[1])) for r in rows if r[0]]

        # 技能
        rows = conn.execute("SELECT Name FROM Config_Skills").fetchall()
        _game_data["skills"] = [safe_str(r[0]) for r in rows if r[0]]

        # 任务
        rows = conn.execute("SELECT Title FROM Config_Task").fetchall()
        _game_data["tasks"] = [safe_str(r[0]) for r in rows if r[0]]

        # NPC
        rows = conn.execute("SELECT Name FROM Ext_NPC_Info").fetchall()
        _game_data["npcs"] = [safe_str(r[0]) for r in rows if r[0]]

        # 职业
        rows = conn.execute("SELECT Name FROM Config_Occupation").fetchall()
        _game_data["occupations"] = [safe_str(r[0]) for r in rows if r[0]]

        # 食物
        rows = conn.execute("SELECT Name FROM Config_Goods WHERE Type='食物' OR Type='食品'").fetchall()
        _game_data["foods"] = [safe_str(r[0]) for r in rows if r[0]]
        if not _game_data["foods"]:
            _game_data["foods"] = ["面包", "苹果", "烤肉"]

        # 配方
        rows = conn.execute("SELECT Produce FROM Config_Composite").fetchall()
        _game_data["composites"] = [safe_str(r[0]) for r in rows if r[0]]

        # 分解
        rows = conn.execute("SELECT Goods FROM Config_Decomposition").fetchall()
        _game_data["decomps"] = [safe_str(r[0]) for r in rows if r[0]]

        conn.close()
    except Exception as e:
        print(f"⚠️ 加载游戏数据失败: {e}")
        _game_data = {
            "items": {}, "items_inv": {}, "monsters": [], "maps": [],
            "skills": [], "tasks": [], "npcs": [], "occupations": [],
            "foods": ["面包", "苹果", "烤肉"], "composites": [], "decomps": []
        }

    return _game_data


# ============== 引擎状态 ==============
_initialized = False


def ensure_init():
    global _initialized
    if not _initialized:
        lib.Cake_Initialize()
        result = lib.Cake_LoadDatabase(DB_PATH.encode("utf-8"))
        if result != 0:
            raise RuntimeError("加载数据库失败")
        _initialized = True


# ============== Gradio 回调 ==============

def init_engine():
    """初始化引擎并加载数据库"""
    global _initialized
    try:
        lib.Cake_Cleanup()
        _initialized = False
        ensure_init()
        data = load_game_data()
        version = safe_str(lib.Cake_GetVersionString())
        user_info = safe_str(lib.Cake_GetUserInfo())

        map_list = "\n".join([f"  🗺️ {name} (Lv.{lv})" for name, lv in data["maps"]])

        return (
            f"✅ 引擎初始化成功！版本: {version}\n\n"
            f"📊 数据库统计:\n"
            f"  🎒 物品: {len(data['items'])} 个\n"
            f"  👾 怪物: {len(data['monsters'])} 个\n"
            f"  🗺️ 地图: {len(data['maps'])} 个\n"
            f"  ✨ 技能: {len(data['skills'])} 个\n"
            f"  📜 任务: {len(data['tasks'])} 个\n"
            f"  👤 NPC: {len(data['npcs'])} 个\n"
            f"  🍖 食物: {len(data['foods'])} 个\n"
            f"  👔 职业: {len(data['occupations'])} 个\n\n"
            f"🗺️ 地图列表:\n{map_list}"
        )
    except Exception as e:
        return f"❌ 初始化失败: {e}"


def create_character(name, occupation):
    """创建角色"""
    try:
        ensure_init()
        lib.Cake_Cleanup()
        lib.Cake_Initialize()
        lib.Cake_LoadDatabase(DB_PATH.encode("utf-8"))

        result = lib.Cake_CreateCharacter(
            name.encode("utf-8"),
            occupation.encode("utf-8")
        )
        messages = poll_all_messages()
        user_info = safe_str(lib.Cake_GetUserInfo())

        if result == 0:
            return f"✅ 角色创建成功！\n\n{user_info}", "\n".join(messages) if messages else "（无消息）"
        else:
            return f"❌ 创建失败 (错误码: {result})\n{user_info}", "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", ""


def refresh_user_info():
    """刷新角色信息"""
    try:
        ensure_init()
        info = safe_str(lib.Cake_GetUserInfo())
        pos = safe_str(lib.Cake_GetPosition())
        return f"{info}\n📍 位置: {pos}" if pos else info
    except:
        return "⚠️ 引擎未初始化，请先创建角色"


def move_to_map(map_name):
    """移动到地图"""
    try:
        ensure_init()
        result = lib.Cake_MoveTo(map_name.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 移动到: {map_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def start_combat(monster_name):
    """开始战斗"""
    try:
        ensure_init()
        result = lib.Cake_StartCombat(monster_name.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 遭遇: {monster_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def do_attack():
    """执行攻击"""
    try:
        ensure_init()
        result = lib.Cake_Attack()
        messages = poll_all_messages()
        user_info = refresh_user_info()
        return user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return "", f"❌ 错误: {e}"


def add_item(item_name, count):
    """添加物品"""
    try:
        ensure_init()
        data = load_game_data()
        item_id = data["items"].get(item_name, item_name)
        result = lib.Cake_AddItem(item_id.encode("utf-8"), int(count))
        messages = poll_all_messages()
        status = "✅" if result == 0 else "❌"
        return f"{status} 添加物品: {item_name} x{count}", "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", ""


def equip_item(item_name):
    """装备物品"""
    try:
        ensure_init()
        data = load_game_data()
        item_id = data["items"].get(item_name, item_name)
        result = lib.Cake_EquipItem(item_id.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 装备: {item_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def use_item(item_name):
    """使用物品"""
    try:
        ensure_init()
        data = load_game_data()
        item_id = data["items"].get(item_name, item_name)
        result = lib.Cake_UseItem(item_id.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 使用: {item_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def learn_skill(skill_name):
    """学习技能"""
    try:
        ensure_init()
        result = lib.Cake_LearnSkill(skill_name.encode("utf-8"))
        messages = poll_all_messages()
        status = "✅" if result == 0 else "❌"
        return f"{status} 学习技能: {skill_name}", "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", ""


def use_skill(skill_name):
    """使用技能（在战斗中）"""
    try:
        ensure_init()
        result = lib.Cake_UseSkill(skill_name.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 使用技能: {skill_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def accept_task(task_name):
    """接取任务"""
    try:
        ensure_init()
        result = lib.Cake_AcceptTask(task_name.encode("utf-8"))
        messages = poll_all_messages()
        status = "✅" if result == 0 else "❌"
        return f"{status} 接取任务: {task_name}", "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", ""


def talk_npc(npc_name):
    """与NPC对话"""
    try:
        ensure_init()
        result = lib.Cake_TalkToNpc(npc_name.encode("utf-8"))
        messages = poll_all_messages()
        status = "✅" if result == 0 else "❌"
        return f"{status} 对话: {npc_name}", "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", ""


def eat_food(food_name):
    """吃食物"""
    try:
        ensure_init()
        result = lib.Cake_EatFood(food_name.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        status = "✅" if result == 0 else "❌"
        return f"{status} 食用: {food_name}", user_info, "\n".join(messages) if messages else "（无消息）"
    except Exception as e:
        return f"❌ 错误: {e}", "", ""


def set_variable(name, value):
    """设置变量"""
    try:
        ensure_init()
        lib.Cake_SetVar(name.encode("utf-8"), value.encode("utf-8"))
        new_val = safe_str(lib.Cake_GetVar(name.encode("utf-8")))
        return f"✅ 变量 {name} = {new_val}"
    except Exception as e:
        return f"❌ 错误: {e}"


def get_variable(name):
    """获取变量"""
    try:
        ensure_init()
        val = safe_str(lib.Cake_GetVar(name.encode("utf-8")))
        return f"📌 {name} = {val}"
    except Exception as e:
        return f"❌ 错误: {e}"


def toggle_flag(flag_name, action):
    """设置/移除标记"""
    try:
        ensure_init()
        if action == "设置":
            lib.Cake_SetFlag(flag_name.encode("utf-8"))
            return f"✅ 标记已设置: {flag_name}"
        else:
            result = lib.Cake_HasFlag(flag_name.encode("utf-8"))
            return f"📌 {flag_name}: {'✅ 已设置' if result else '❌ 未设置'}"
    except Exception as e:
        return f"❌ 错误: {e}"


def save_game():
    """保存游戏状态"""
    try:
        ensure_init()
        json_data = lib.Cake_SaveState()
        if json_data:
            state = safe_str(json_data)
            with open(SAVE_PATH, "w", encoding="utf-8") as f:
                f.write(state)
            parsed = json.loads(state)
            formatted = json.dumps(parsed, ensure_ascii=False, indent=2)
            return f"✅ 已保存到 {SAVE_PATH}\n\n{formatted}"
        return "❌ 保存失败"
    except Exception as e:
        return f"❌ 错误: {e}"


def load_game():
    """加载游戏状态"""
    try:
        ensure_init()
        if not os.path.exists(SAVE_PATH):
            return "❌ 没有存档文件"
        with open(SAVE_PATH, "r", encoding="utf-8") as f:
            state = f.read()
        result = lib.Cake_LoadState(state.encode("utf-8"))
        messages = poll_all_messages()
        user_info = refresh_user_info()
        if result == 0:
            return f"✅ 存档加载成功\n\n{user_info}"
        return f"❌ 加载失败 (错误码: {result})"
    except Exception as e:
        return f"❌ 错误: {e}"


def browse_table(table_name, limit=50):
    """浏览数据库表"""
    try:
        conn = sqlite3.connect(DB_PATH)
        conn.row_factory = sqlite3.Row
        rows = conn.execute(f"SELECT * FROM [{table_name}] LIMIT {int(limit)}").fetchall()
        if not rows:
            return f"表 {table_name} 为空"

        headers = rows[0].keys()
        lines = [f"📊 表: {table_name} (显示 {len(rows)} 行)\n"]
        lines.append(" | ".join(headers))
        lines.append("-" * 80)
        for row in rows:
            vals = [str(row[h])[:30] for h in headers]
            lines.append(" | ".join(vals))

        conn.close()
        return "\n".join(lines)
    except Exception as e:
        return f"❌ 错误: {e}"


def get_table_list():
    """获取所有表名"""
    try:
        conn = sqlite3.connect(DB_PATH)
        tables = conn.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").fetchall()
        conn.close()
        lines = ["📋 数据库表列表:\n"]
        for t in tables:
            conn2 = sqlite3.connect(DB_PATH)
            count = conn2.execute(f"SELECT COUNT(*) FROM [{t[0]}]").fetchone()[0]
            conn2.close()
            lines.append(f"  {t[0]}: {count} 行")
        return "\n".join(lines)
    except Exception as e:
        return f"❌ 错误: {e}"


def run_custom_action(action, param1, param2, param3):
    """执行自定义操作"""
    try:
        ensure_init()
        results = []

        if action == "购买物品":
            result = lib.Cake_BuyItem(param1.encode("utf-8"), param2.encode("utf-8"), int(param3 or 1))
            messages = poll_all_messages()
            results.append(f"购买结果: {'成功' if result == 0 else '失败'}")
            if messages:
                results.extend(messages)

        elif action == "合成":
            result = lib.Cake_Composite(param1.encode("utf-8"))
            messages = poll_all_messages()
            results.append(f"合成结果: {'成功' if result == 0 else '失败'}")
            if messages:
                results.extend(messages)

        elif action == "分解":
            result = lib.Cake_Decompose(param1.encode("utf-8"))
            messages = poll_all_messages()
            results.append(f"分解结果: {'成功' if result == 0 else '失败'}")
            if messages:
                results.extend(messages)

        user_info = refresh_user_info()
        return user_info, "\n".join(results) if results else "（无结果）"
    except Exception as e:
        return "", f"❌ 错误: {e}"


# ============== 自定义 CSS ==============
CUSTOM_CSS = """
/* 全局样式 */
.gradio-container {
    max-width: 1200px !important;
    margin: 0 auto;
}

/* 标题样式 */
.header {
    text-align: center;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    font-size: 2em;
    font-weight: bold;
    margin-bottom: 10px;
}

/* 状态卡片 */
.status-card {
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
    border-radius: 10px;
    padding: 15px;
    border-left: 4px solid #667eea;
}

/* 日志框 */
.log-box {
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace !important;
    font-size: 13px !important;
    background-color: #1e1e1e !important;
    color: #d4d4d4 !important;
    border-radius: 8px !important;
}

/* 按钮样式 */
.primary-btn {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%) !important;
    border: none !important;
    color: white !important;
    font-weight: bold !important;
    transition: all 0.3s ease !important;
}

.primary-btn:hover {
    transform: translateY(-2px) !important;
    box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4) !important;
}

/* Tab 样式 */
.tabs > .tab-nav {
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
    border-radius: 10px 10px 0 0;
    padding: 5px;
}

.tabs > .tab-nav > button {
    border-radius: 8px !important;
    font-weight: 500 !important;
}

.tabs > .tab-nav > button.selected {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%) !important;
    color: white !important;
}
"""


# ============== 构建 UI ==============
def build_ui():
    data = load_game_data()

    with gr.Blocks(
        title="CakeGame-RS 调试器 v3",
        theme=gr.themes.Soft(),
        css=CUSTOM_CSS
    ) as app:

        gr.Markdown("# 🎮 CakeGame-RS 调试器 v3\n基于 Rust 引擎的现代化 Web 调试界面", elem_classes="header")

        # === 状态栏 ===
        with gr.Row():
            with gr.Column(scale=1):
                init_btn = gr.Button("🚀 初始化引擎", variant="primary", elem_classes="primary-btn")
            with gr.Column(scale=3):
                engine_status = gr.Textbox(
                    label="引擎状态",
                    lines=12,
                    interactive=False,
                    elem_classes="status-card"
                )

        # === 主功能 Tabs ===
        with gr.Tabs():

            # ---- Tab 1: 角色 & 状态 ----
            with gr.Tab("👤 角色"):
                with gr.Row():
                    with gr.Column(scale=1):
                        gr.Markdown("### 🎭 创建角色")
                        char_name = gr.Textbox(label="角色名", value="测试勇者")
                        char_occ = gr.Dropdown(
                            choices=data["occupations"],
                            label="职业",
                            value=data["occupations"][0] if data["occupations"] else None,
                            allow_custom_value=True
                        )
                        create_btn = gr.Button("🎭 创建角色", variant="primary", elem_classes="primary-btn")

                    with gr.Column(scale=2):
                        user_info_display = gr.Textbox(label="角色信息", lines=5, interactive=False)
                        char_messages = gr.Textbox(label="消息日志", lines=5, interactive=False, elem_classes="log-box")

                refresh_btn = gr.Button("🔄 刷新状态")

            # ---- Tab 2: 探索 & 战斗 ----
            with gr.Tab("⚔️ 战斗"):
                with gr.Row():
                    with gr.Column():
                        gr.Markdown("### 🗺️ 移动")
                        map_dropdown = gr.Dropdown(
                            choices=[f"{name} (Lv.{lv})" for name, lv in data["maps"]],
                            label="选择地图",
                            allow_custom_value=True
                        )
                        move_btn = gr.Button("🚶 移动", variant="secondary")

                        gr.Markdown("### ⚔️ 战斗")
                        monster_dropdown = gr.Dropdown(
                            choices=data["monsters"],
                            label="选择怪物",
                            allow_custom_value=True
                        )
                        combat_btn = gr.Button("⚔️ 开始战斗", variant="secondary")
                        attack_btn = gr.Button("🗡️ 攻击", variant="primary", elem_classes="primary-btn")

                        gr.Markdown("### ✨ 技能")
                        skill_dropdown = gr.Dropdown(
                            choices=data["skills"],
                            label="选择技能",
                            allow_custom_value=True
                        )
                        learn_skill_btn = gr.Button("📖 学习技能")
                        use_skill_btn = gr.Button("🔥 使用技能（战斗中）")

                    with gr.Column():
                        battle_user_info = gr.Textbox(label="角色状态", lines=5, interactive=False)
                        battle_log = gr.Textbox(label="战斗日志", lines=15, interactive=False, elem_classes="log-box")

            # ---- Tab 3: 物品 ----
            with gr.Tab("🎒 物品"):
                with gr.Row():
                    with gr.Column():
                        gr.Markdown("### 🎒 物品管理")
                        item_dropdown = gr.Dropdown(
                            choices=list(data["items"].keys()),
                            label="选择物品",
                            allow_custom_value=True,
                            filterable=True
                        )
                        item_count = gr.Number(label="数量", value=1, minimum=1, maximum=999)
                        with gr.Row():
                            add_item_btn = gr.Button("➕ 添加物品", variant="primary", elem_classes="primary-btn")
                            equip_btn = gr.Button("🗡️ 装备")
                            use_item_btn = gr.Button("🧪 使用")

                    with gr.Column():
                        item_user_info = gr.Textbox(label="角色状态", lines=5, interactive=False)
                        item_messages = gr.Textbox(label="物品日志", lines=8, interactive=False, elem_classes="log-box")

                gr.Markdown("### 🍖 食物")
                with gr.Row():
                    food_dropdown = gr.Dropdown(
                        choices=data["foods"],
                        label="选择食物",
                        allow_custom_value=True,
                        filterable=True
                    )
                    eat_btn = gr.Button("🍽️ 食用")

            # ---- Tab 4: 任务 & NPC ----
            with gr.Tab("📜 任务"):
                with gr.Row():
                    with gr.Column():
                        gr.Markdown("### 📜 任务")
                        task_dropdown = gr.Dropdown(
                            choices=data["tasks"],
                            label="选择任务",
                            allow_custom_value=True,
                            filterable=True
                        )
                        with gr.Row():
                            accept_task_btn = gr.Button("📋 接取任务")

                        gr.Markdown("### 👤 NPC")
                        npc_dropdown = gr.Dropdown(
                            choices=data["npcs"],
                            label="选择NPC",
                            allow_custom_value=True
                        )
                        talk_btn = gr.Button("💬 对话")

                    with gr.Column():
                        quest_log = gr.Textbox(label="任务/NPC 日志", lines=15, interactive=False, elem_classes="log-box")

            # ---- Tab 5: 变量 & 标记 ----
            with gr.Tab("🔧 变量"):
                with gr.Row():
                    with gr.Column():
                        gr.Markdown("### 📌 变量操作")
                        var_name = gr.Textbox(label="变量名")
                        var_value = gr.Textbox(label="变量值")
                        with gr.Row():
                            set_var_btn = gr.Button("✏️ 设置")
                            get_var_btn = gr.Button("🔍 查询")
                        var_result = gr.Textbox(label="结果", interactive=False)

                    with gr.Column():
                        gr.Markdown("### 🚩 标记操作")
                        flag_name = gr.Textbox(label="标记名")
                        with gr.Row():
                            set_flag_btn = gr.Button("✅ 设置标记")
                            check_flag_btn = gr.Button("🔍 检查标记")
                        flag_result = gr.Textbox(label="结果", interactive=False)

            # ---- Tab 6: 存档 ----
            with gr.Tab("💾 存档"):
                gr.Markdown("### 💾 游戏存档管理")
                with gr.Row():
                    save_btn = gr.Button("💾 保存存档", variant="primary", elem_classes="primary-btn")
                    load_btn = gr.Button("📂 加载存档", variant="secondary")
                save_display = gr.Textbox(label="存档数据", lines=20, interactive=False, elem_classes="log-box")

            # ---- Tab 7: 高级操作 ----
            with gr.Tab("⚙️ 高级"):
                gr.Markdown("### ⚙️ 高级操作")
                with gr.Row():
                    with gr.Column():
                        gr.Markdown("### 🛒 购买物品")
                        shop_id = gr.Textbox(label="商店ID")
                        shop_item = gr.Textbox(label="物品ID")
                        shop_count = gr.Number(label="数量", value=1, minimum=1)
                        buy_btn = gr.Button("💰 购买")

                    with gr.Column():
                        gr.Markdown("### 🔨 合成 / 分解")
                        recipe_id = gr.Textbox(label="配方/物品ID")
                        with gr.Row():
                            composite_btn = gr.Button("🔨 合成")
                            decompose_btn = gr.Button("♻️ 分解")

                advanced_user = gr.Textbox(label="角色状态", lines=5, interactive=False)
                advanced_log = gr.Textbox(label="操作日志", lines=8, interactive=False, elem_classes="log-box")

            # ---- Tab 8: 数据库浏览器 ----
            with gr.Tab("🗄️ 数据库"):
                gr.Markdown("### 🗄️ SQLite 数据库浏览器")
                with gr.Row():
                    table_list_btn = gr.Button("📋 列出所有表")
                    table_name_input = gr.Textbox(label="表名")
                    table_limit = gr.Number(label="显示行数", value=50, minimum=1, maximum=500)
                    browse_btn = gr.Button("🔍 浏览", variant="primary", elem_classes="primary-btn")
                db_display = gr.Textbox(label="查询结果", lines=25, interactive=False, elem_classes="log-box")

        # ============== 事件绑定 ==============

        init_btn.click(init_engine, outputs=[engine_status])
        create_btn.click(create_character, inputs=[char_name, char_occ], outputs=[user_info_display, char_messages])
        refresh_btn.click(refresh_user_info, outputs=[user_info_display])

        def move_click(map_choice):
            name = map_choice.split(" (Lv.")[0] if " (Lv." in map_choice else map_choice
            return move_to_map(name)

        move_btn.click(move_click, inputs=[map_dropdown], outputs=[battle_log, battle_user_info, battle_log])
        combat_btn.click(start_combat, inputs=[monster_dropdown], outputs=[battle_log, battle_user_info, battle_log])
        attack_btn.click(do_attack, outputs=[battle_user_info, battle_log])

        def add_item_click(name, count):
            status, msg = add_item(name, count)
            info = refresh_user_info()
            return status, info, msg

        add_item_btn.click(add_item_click, inputs=[item_dropdown, item_count], outputs=[item_messages, item_user_info, item_messages])
        equip_btn.click(equip_item, inputs=[item_dropdown], outputs=[item_messages, item_user_info, item_messages])
        use_item_btn.click(use_item, inputs=[item_dropdown], outputs=[item_messages, item_user_info, item_messages])
        eat_btn.click(eat_food, inputs=[food_dropdown], outputs=[item_messages, item_user_info, item_messages])

        learn_skill_btn.click(learn_skill, inputs=[skill_dropdown], outputs=[battle_log])
        use_skill_btn.click(use_skill, inputs=[skill_dropdown], outputs=[battle_log, battle_user_info, battle_log])

        accept_task_btn.click(accept_task, inputs=[task_dropdown], outputs=[quest_log])
        talk_btn.click(talk_npc, inputs=[npc_dropdown], outputs=[quest_log])

        set_var_btn.click(set_variable, inputs=[var_name, var_value], outputs=[var_result])
        get_var_btn.click(get_variable, inputs=[var_name], outputs=[var_result])

        set_flag_btn.click(lambda n: toggle_flag(n, "设置"), inputs=[flag_name], outputs=[flag_result])
        check_flag_btn.click(lambda n: toggle_flag(n, "查询"), inputs=[flag_name], outputs=[flag_result])

        save_btn.click(save_game, outputs=[save_display])
        load_btn.click(load_game, outputs=[save_display])

        buy_btn.click(
            lambda s, i, c: run_custom_action("购买物品", s, i, str(int(c))),
            inputs=[shop_id, shop_item, shop_count],
            outputs=[advanced_user, advanced_log]
        )
        composite_btn.click(
            lambda r: run_custom_action("合成", r, "", "1"),
            inputs=[recipe_id],
            outputs=[advanced_user, advanced_log]
        )
        decompose_btn.click(
            lambda r: run_custom_action("分解", r, "", "1"),
            inputs=[recipe_id],
            outputs=[advanced_user, advanced_log]
        )

        table_list_btn.click(get_table_list, outputs=[db_display])
        browse_btn.click(browse_table, inputs=[table_name_input, table_limit], outputs=[db_display])

    return app


# ============== 启动 ==============
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="CakeGame-RS 调试器 v3")
    parser.add_argument("--share", action="store_true", help="启用公网分享链接")
    parser.add_argument("--port", type=int, default=7860, help="服务器端口 (默认: 7860)")
    parser.add_argument("--host", type=str, default="0.0.0.0", help="服务器地址 (默认: 0.0.0.0)")
    args = parser.parse_args()

    print("🎮 CakeGame-RS 调试器 v3")
    print("=" * 40)

    app = build_ui()

    print(f"\n🚀 启动服务器...")
    print(f"   地址: http://{args.host}:{args.port}")
    if args.share:
        print(f"   公网: 尝试创建分享链接...")
    print("=" * 40)

    app.launch(
        server_name=args.host,
        server_port=args.port,
        share=args.share,
        show_error=True,
    )
