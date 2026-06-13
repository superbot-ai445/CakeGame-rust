"""CakeGame Gradio 调试 UI v2 - 通过 FFI 调用 Rust 引擎"""
import ctypes
import os
import json
import gradio as gr

# ==================== FFI 加载 ====================

LIB_PATH = os.path.join(os.path.dirname(__file__), "target/release/libcakegame.so")
DB_PATH = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb"

lib = ctypes.CDLL(LIB_PATH)

# cg_init(db_path: *const c_char) -> i32
lib.cg_init.argtypes = [ctypes.c_char_p]
lib.cg_init.restype = ctypes.c_int

# cg_process(msg, msg_type, group, user_id) -> *mut c_char
lib.cg_process.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.cg_process.restype = ctypes.c_void_p

# cg_free_string(s: *mut c_char)
lib.cg_free_string.argtypes = [ctypes.c_void_p]
lib.cg_free_string.restype = None

# cg_get_user_info(user_id) -> *mut c_char
lib.cg_get_user_info.argtypes = [ctypes.c_char_p]
lib.cg_get_user_info.restype = ctypes.c_void_p

# cg_get_knapsack(user_id) -> *mut c_char
lib.cg_get_knapsack.argtypes = [ctypes.c_char_p]
lib.cg_get_knapsack.restype = ctypes.c_void_p

# cg_get_equips(user_id) -> *mut c_char
lib.cg_get_equips.argtypes = [ctypes.c_char_p]
lib.cg_get_equips.restype = ctypes.c_void_p

# cg_shutdown()
lib.cg_shutdown.argtypes = []
lib.cg_shutdown.restype = None


def read_cstr(ptr):
    """读取 C 字符串并释放内存"""
    if ptr is None:
        return ""
    result = ctypes.cast(ptr, ctypes.c_char_p).value
    lib.cg_free_string(ptr)
    if result is None:
        return ""
    return result.decode("utf-8", errors="replace")


def init_engine():
    """初始化引擎"""
    ret = lib.cg_init(DB_PATH.encode("utf-8"))
    if ret == 0:
        return "✅ 引擎初始化成功"
    else:
        return f"❌ 引擎初始化失败 (错误码: {ret})"


def process_command(msg, user_id, msg_type="2", group="0"):
    """处理指令"""
    if not msg.strip():
        return ""
    result_ptr = lib.cg_process(
        msg.encode("utf-8"),
        msg_type.encode("utf-8"),
        group.encode("utf-8"),
        user_id.encode("utf-8"),
    )
    return read_cstr(result_ptr)


def get_user_info(user_id):
    """获取用户信息 JSON"""
    result_ptr = lib.cg_get_user_info(user_id.encode("utf-8"))
    return read_cstr(result_ptr)


def get_knapsack(user_id):
    """获取背包 JSON"""
    result_ptr = lib.cg_get_knapsack(user_id.encode("utf-8"))
    return read_cstr(result_ptr)


def get_equips(user_id):
    """获取装备 JSON"""
    result_ptr = lib.cg_get_equips(user_id.encode("utf-8"))
    return read_cstr(result_ptr)


# ==================== Gradio UI ====================

def handle_command(msg, user_id):
    """处理指令并返回结果"""
    if not user_id.strip():
        return "请输入用户ID", "", ""
    result = process_command(msg, user_id)
    info_json = get_user_info(user_id)
    knapsack_json = get_knapsack(user_id)
    return result, info_json, knapsack_json


def handle_quick_cmd(cmd, user_id):
    """快捷指令"""
    if not user_id.strip():
        return "请输入用户ID", "", ""
    result = process_command(cmd, user_id)
    info_json = get_user_info(user_id)
    knapsack_json = get_knapsack(user_id)
    return result, info_json, knapsack_json


def view_equips(user_id):
    """查看装备"""
    if not user_id.strip():
        return "请输入用户ID"
    result = process_command("查看装备", user_id)
    return result


# 初始化引擎
init_status = init_engine()

# ==================== 界面 ====================

with gr.Blocks(title="CakeGame 调试器 v2", theme=gr.themes.Soft()) as demo:
    gr.Markdown("# 🎮 CakeGame Rust 引擎调试器")
    gr.Markdown(f"**引擎状态**: {init_status}  |  **数据库**: `{DB_PATH}`")

    with gr.Row():
        user_id = gr.Textbox(label="用户ID (QQ号)", value="test_user", scale=1)

    with gr.Tabs():
        # ===== Tab 1: 指令控制台 =====
        with gr.Tab("📋 指令控制台"):
            with gr.Row():
                with gr.Column(scale=3):
                    msg_input = gr.Textbox(label="输入指令", placeholder="例: 注册+测试昵称", lines=1)
                    cmd_submit = gr.Button("发送指令", variant="primary")
                    cmd_output = gr.Textbox(label="指令结果", lines=12, interactive=False)

                with gr.Column(scale=2):
                    user_info_display = gr.Code(label="用户信息 (JSON)", language="json", lines=8)
                    knapsack_display = gr.Code(label="背包 (JSON)", language="json", lines=8)

            cmd_submit.click(
                handle_command,
                inputs=[msg_input, user_id],
                outputs=[cmd_output, user_info_display, knapsack_display],
            )
            msg_input.submit(
                handle_command,
                inputs=[msg_input, user_id],
                outputs=[cmd_output, user_info_display, knapsack_display],
            )

        # ===== Tab 2: 快捷操作 =====
        with gr.Tab("⚡ 快捷操作"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### 基础操作")
                    with gr.Row():
                        btn_reg = gr.Button("注册", variant="secondary")
                        btn_sign = gr.Button("签到", variant="secondary")
                        btn_char = gr.Button("查看角色", variant="secondary")
                        btn_help = gr.Button("帮助", variant="secondary")

                    gr.Markdown("### 物品操作")
                    with gr.Row():
                        btn_bag = gr.Button("查看背包", variant="secondary")
                        btn_equip = gr.Button("查看装备", variant="secondary")
                        btn_skill = gr.Button("查看技能", variant="secondary")

                    gr.Markdown("### 地图操作")
                    with gr.Row():
                        btn_map = gr.Button("查看地图", variant="secondary")
                        btn_location = gr.Button("位置信息", variant="secondary")
                        btn_search = gr.Button("搜索怪物", variant="secondary")

                    gr.Markdown("### 排行榜")
                    with gr.Row():
                        btn_rank_lv = gr.Button("等级排行", variant="secondary")
                        btn_rank_gold = gr.Button("金币排行", variant="secondary")
                        btn_rank_atk = gr.Button("物攻排行", variant="secondary")

                with gr.Column():
                    quick_output = gr.Textbox(label="操作结果", lines=20, interactive=False)
                    quick_user_info = gr.Code(label="用户信息", language="json", lines=8)

            # 绑定快捷按钮
            all_outputs = [quick_output, quick_user_info, knapsack_display]

            btn_reg.click(lambda uid: handle_quick_cmd("注册+测试昵称", uid), [user_id], all_outputs)
            btn_sign.click(lambda uid: handle_quick_cmd("签到", uid), [user_id], all_outputs)
            btn_char.click(lambda uid: handle_quick_cmd("查看角色", uid), [user_id], all_outputs)
            btn_help.click(lambda uid: handle_quick_cmd("帮助", uid), [user_id], all_outputs)
            btn_bag.click(lambda uid: handle_quick_cmd("查看背包", uid), [user_id], all_outputs)
            btn_equip.click(lambda uid: (view_equips(uid), get_user_info(uid), get_knapsack(uid)), [user_id], all_outputs)
            btn_skill.click(lambda uid: handle_quick_cmd("查看技能", uid), [user_id], all_outputs)
            btn_map.click(lambda uid: handle_quick_cmd("查看地图", uid), [user_id], all_outputs)
            btn_location.click(lambda uid: handle_quick_cmd("位置信息", uid), [user_id], all_outputs)
            btn_search.click(lambda uid: handle_quick_cmd("搜索怪物", uid), [user_id], all_outputs)
            btn_rank_lv.click(lambda uid: handle_quick_cmd("等级排行", uid), [user_id], all_outputs)
            btn_rank_gold.click(lambda uid: handle_quick_cmd("金币排行", uid), [user_id], all_outputs)
            btn_rank_atk.click(lambda uid: handle_quick_cmd("物攻排行", uid), [user_id], all_outputs)

        # ===== Tab 3: 装备管理 =====
        with gr.Tab("🛡️ 装备管理"):
            with gr.Row():
                with gr.Column():
                    equip_output = gr.Textbox(label="装备详情", lines=15, interactive=False)
                    btn_refresh_equip = gr.Button("刷新装备", variant="primary")

                with gr.Column():
                    with gr.Row():
                        unequip_slot = gr.Textbox(label="卸下槽位", placeholder="例: 武器")
                        btn_unequip = gr.Button("卸下装备", variant="secondary")

                    use_item_name = gr.Textbox(label="使用物品", placeholder="例: 初级疗伤药剂")
                    btn_use = gr.Button("使用", variant="secondary")

                    drop_item_name = gr.Textbox(label="丢弃物品", placeholder="例: 破旧的剑")
                    btn_drop = gr.Button("丢弃", variant="secondary")

                    unequip_result = gr.Textbox(label="操作结果", lines=5, interactive=False)

            btn_refresh_equip.click(lambda uid: (view_equips(uid), get_user_info(uid), get_knapsack(uid)), [user_id], [equip_output, quick_user_info, knapsack_display])
            btn_unequip.click(lambda uid, slot: (process_command(f"卸下+{slot}", uid), view_equips(uid)), [user_id, unequip_slot], [unequip_result, equip_output])
            btn_use.click(lambda uid, name: (process_command(f"使用+{name}", uid), get_knapsack(uid)), [user_id, use_item_name], [unequip_result, knapsack_display])
            btn_drop.click(lambda uid, name: (process_command(f"丢弃+{name}", uid), get_knapsack(uid)), [user_id, drop_item_name], [unequip_result, knapsack_display])

        # ===== Tab 4: 战斗系统 =====
        with gr.Tab("⚔️ 战斗系统"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### 战斗操作")
                    with gr.Row():
                        btn_attack = gr.Button("攻击", variant="primary")
                        btn_auto_attack = gr.Button("自动攻击", variant="secondary")

                    skill_name_input = gr.Textbox(label="技能名称", placeholder="例: 初级重击")
                    btn_use_skill = gr.Button("释放技能", variant="secondary")

                    target_input = gr.Textbox(label="锁定目标", placeholder="例: 史莱姆")
                    with gr.Row():
                        btn_lock = gr.Button("锁定目标", variant="secondary")
                        btn_unlock = gr.Button("解锁目标", variant="secondary")

                with gr.Column():
                    combat_output = gr.Textbox(label="战斗结果", lines=20, interactive=False)

            btn_attack.click(lambda uid: process_command("攻击", uid), [user_id], [combat_output])
            btn_auto_attack.click(lambda uid: process_command("自动攻击", uid), [user_id], [combat_output])
            btn_use_skill.click(lambda uid, skill: process_command(f"释放技能+{skill}", uid), [user_id, skill_name_input], [combat_output])
            btn_lock.click(lambda uid, target: process_command(f"锁定目标+{target}", uid), [user_id, target_input], [combat_output])
            btn_unlock.click(lambda uid: process_command("解锁目标", uid), [user_id], [combat_output])

        # ===== Tab 5: 地图系统 =====
        with gr.Tab("🗺️ 地图系统"):
            with gr.Row():
                with gr.Column():
                    map_output = gr.Textbox(label="地图信息", lines=15, interactive=False)
                    btn_map_refresh = gr.Button("刷新地图", variant="primary")

                with gr.Column():
                    direction_input = gr.Textbox(label="移动方向/地图名", placeholder="例: 上 或 主城")
                    btn_move = gr.Button("移动", variant="secondary")
                    move_result = gr.Textbox(label="移动结果", lines=5, interactive=False)

            btn_map_refresh.click(lambda uid: (process_command("查看地图", uid), process_command("位置信息", uid)), [user_id], [map_output, move_result])
            btn_move.click(lambda uid, d: (process_command(f"进入+{d}", uid), process_command("位置信息", uid)), [user_id, direction_input], [move_result, map_output])

        # ===== Tab 6: 任务系统 =====
        with gr.Tab("📜 任务系统"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### 任务操作")
                    with gr.Row():
                        btn_my_tasks = gr.Button("我的任务", variant="primary")
                        btn_task_info = gr.Button("任务信息", variant="secondary")

                    task_name_input = gr.Textbox(label="任务名称/序号", placeholder="例: 初出茅庐 或 1")
                    with gr.Row():
                        btn_accept_task = gr.Button("领取任务", variant="secondary")
                        btn_submit_task = gr.Button("提交任务", variant="secondary")
                        btn_abandon_task = gr.Button("放弃任务", variant="secondary")

                with gr.Column():
                    task_output = gr.Textbox(label="任务结果", lines=20, interactive=False)

            btn_my_tasks.click(lambda uid: process_command("我的任务", uid), [user_id], [task_output])
            btn_task_info.click(lambda uid, name: process_command(f"任务信息+{name}" if name else "任务信息", uid), [user_id, task_name_input], [task_output])
            btn_accept_task.click(lambda uid, name: process_command(f"领取任务+{name}", uid), [user_id, task_name_input], [task_output])
            btn_submit_task.click(lambda uid: process_command("提交任务", uid), [user_id], [task_output])
            btn_abandon_task.click(lambda uid: process_command("放弃任务", uid), [user_id], [task_output])

        # ===== Tab 7: PVP战斗 =====
        with gr.Tab("💀 PVP战斗"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### PVP操作")
                    pvp_target_input = gr.Textbox(label="目标玩家ID", placeholder="例: 123456789")
                    with gr.Row():
                        btn_lock_player = gr.Button("锁定玩家", variant="secondary")
                        btn_attack_player = gr.Button("攻击玩家", variant="primary")

                    gr.Markdown("### 说明")
                    gr.Markdown("""
                    1. 先锁定目标玩家
                    2. 然后发起攻击
                    3. 攻击会自动进行回合制战斗
                    4. 胜利可获得金币和物品掉落
                    """)

                with gr.Column():
                    pvp_output = gr.Textbox(label="PVP结果", lines=20, interactive=False)

            btn_lock_player.click(lambda uid, target: process_command(f"锁定玩家+{target}", uid), [user_id, pvp_target_input], [pvp_output])
            btn_attack_player.click(lambda uid: process_command("攻击玩家", uid), [user_id], [pvp_output])

        # ===== Tab 8: 公会系统 =====
        with gr.Tab("🏰 公会系统"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### 公会操作")
                    guild_name_input = gr.Textbox(label="公会名称", placeholder="例: 勇者公会")
                    with gr.Row():
                        btn_create_guild = gr.Button("创建公会", variant="primary")
                        btn_my_guild = gr.Button("我的公会", variant="secondary")
                        btn_guild_list = gr.Button("公会列表", variant="secondary")

                    with gr.Row():
                        btn_leave_guild = gr.Button("退出公会", variant="secondary")
                        btn_disband_guild = gr.Button("解散公会", variant="secondary")

                    guild_donate_input = gr.Textbox(label="捐献金额", placeholder="例: 100")
                    btn_guild_donate = gr.Button("公会捐献", variant="secondary")

                with gr.Column():
                    guild_output = gr.Textbox(label="公会结果", lines=20, interactive=False)

            btn_create_guild.click(lambda uid, name: process_command(f"创建公会+{name}", uid), [user_id, guild_name_input], [guild_output])
            btn_my_guild.click(lambda uid: process_command("我的公会", uid), [user_id], [guild_output])
            btn_guild_list.click(lambda uid: process_command("公会列表", uid), [user_id], [guild_output])
            btn_leave_guild.click(lambda uid: process_command("退出公会", uid), [user_id], [guild_output])
            btn_disband_guild.click(lambda uid: process_command("解散公会", uid), [user_id], [guild_output])
            btn_guild_donate.click(lambda uid, amount: process_command(f"公会捐献+{amount}", uid), [user_id, guild_donate_input], [guild_output])

        # ===== Tab 9: 私人商店 =====
        with gr.Tab("🏪 私人商店"):
            with gr.Row():
                with gr.Column():
                    gr.Markdown("### 商店操作")
                    with gr.Row():
                        btn_shop_list = gr.Button("商店列表", variant="primary")
                        btn_my_shop = gr.Button("我的商店", variant="secondary")

                    shop_enter_input = gr.Textbox(label="进入商店", placeholder="例: 1 或 商店名")
                    btn_enter_shop = gr.Button("进入商店", variant="secondary")
                    btn_exit_shop = gr.Button("退出商店", variant="secondary")

                    gr.Markdown("### 商品管理")
                    shop_item_input = gr.Textbox(label="上架商品", placeholder="例: 初级药剂*100*金币")
                    btn_add_item = gr.Button("上架商品", variant="secondary")

                    shop_remove_input = gr.Textbox(label="下架商品序号", placeholder="例: 1")
                    btn_remove_item = gr.Button("下架商品", variant="secondary")

                    with gr.Row():
                        btn_open_shop = gr.Button("开启商店", variant="secondary")
                        btn_close_shop = gr.Button("关闭商店", variant="secondary")

                    shop_rename_input = gr.Textbox(label="商店改名", placeholder="例: 我的商店")
                    btn_rename_shop = gr.Button("商店改名", variant="secondary")

                with gr.Column():
                    shop_output = gr.Textbox(label="商店结果", lines=20, interactive=False)

            btn_shop_list.click(lambda uid: process_command("商店列表", uid), [user_id], [shop_output])
            btn_my_shop.click(lambda uid: process_command("我的商店", uid), [user_id], [shop_output])
            btn_enter_shop.click(lambda uid, name: process_command(f"进入商店+{name}", uid), [user_id, shop_enter_input], [shop_output])
            btn_exit_shop.click(lambda uid: process_command("退出商店", uid), [user_id], [shop_output])
            btn_add_item.click(lambda uid, item: process_command(f"上架商品+{item}", uid), [user_id, shop_item_input], [shop_output])
            btn_remove_item.click(lambda uid, idx: process_command(f"下架商品+{idx}", uid), [user_id, shop_remove_input], [shop_output])
            btn_open_shop.click(lambda uid: process_command("开启商店", uid), [user_id], [shop_output])
            btn_close_shop.click(lambda uid: process_command("关闭商店", uid), [user_id], [shop_output])
            btn_rename_shop.click(lambda uid, name: process_command(f"商店改名+{name}", uid), [user_id, shop_rename_input], [shop_output])

        # ===== Tab 10: 指令列表 =====
        with gr.Tab("📖 指令列表"):
            gr.Markdown("""
## 可用指令

### 基础指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 注册 | `注册+昵称` | 注册新用户 |
| 签到 | `签到` | 每日签到 |
| 查看角色 | `查看角色` | 查看角色信息 |
| 帮助 | `帮助` | 显示帮助 |
| 修改昵称 | `修改昵称+新昵称` | 修改昵称 |

### 物品指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 查看背包 | `查看背包` | 查看背包物品 |
| 使用物品 | `使用+物品名` | 使用物品 |
| 丢弃物品 | `丢弃+物品名` | 丢弃物品 |
| 查看装备 | `查看装备` | 查看装备栏 |
| 卸下装备 | `卸下+槽位名` | 卸下装备 |
| 查看技能 | `查看技能` | 查看技能列表 |

### 战斗指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 搜索怪物 | `搜索怪物` | 搜索并锁定怪物 |
| 攻击 | `攻击` | 攻击当前目标 |
| 自动攻击 | `自动攻击` | 自动战斗 |
| 释放技能 | `释放技能+技能名` | 使用技能 |
| 锁定目标 | `锁定目标+目标名` | 锁定攻击目标 |
| 解锁目标 | `解锁目标` | 解锁当前目标 |

### 地图指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 查看地图 | `查看地图` | 查看当前位置 |
| 进入地图 | `进入+方向/地图名` | 移动到地图 |
| 位置信息 | `位置信息` | 当前位置详情 |

### 任务指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 我的任务 | `我的任务+页码` | 查看可接任务列表 |
| 任务信息 | `任务信息+任务名` | 查看任务详情 |
| 领取任务 | `领取任务+任务名` | 领取任务 |
| 提交任务 | `提交任务` | 提交完成的任务 |
| 放弃任务 | `放弃任务` | 放弃当前任务 |

### PVP指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 锁定玩家 | `锁定玩家+玩家ID` | 锁定要攻击的玩家 |
| 攻击玩家 | `攻击玩家` | 攻击锁定的玩家 |

### 公会指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 创建公会 | `创建公会+公会名` | 创建公会 |
| 我的公会 | `我的公会` | 查看公会信息 |
| 公会列表 | `公会列表` | 查看所有公会 |
| 退出公会 | `退出公会` | 退出公会 |
| 解散公会 | `解散公会` | 解散公会 |
| 公会捐献 | `公会捐献+金额` | 捐献金币 |

### 队伍指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 创建队伍 | `创建队伍` | 创建队伍 |
| 加入队伍 | `加入队伍+队伍ID` | 加入队伍 |
| 退出队伍 | `退出队伍` | 退出队伍 |
| 队伍成员 | `队伍成员` | 查看队伍成员 |

### 商店指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 查看商店 | `查看商店` | 查看系统商店 |
| 购买商品 | `购买+商品名` | 购买商品 |
| 查看合成 | `查看合成+物品名` | 查看合成配方 |
| 合成物品 | `合成+物品名` | 合成物品 |
| 合成列表 | `合成列表` | 查看可合成物品 |

### 私人商店指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 商店列表 | `商店列表` | 查看所有商店 |
| 进入商店 | `进入商店+序号/店名` | 进入商店 |
| 退出商店 | `退出商店` | 退出商店 |
| 我的商店 | `我的商店` | 查看自己的商店 |
| 上架商品 | `上架商品+物品*价格*货币` | 上架商品 |
| 下架商品 | `下架商品+序号` | 下架商品 |
| 开启商店 | `开启商店` | 开启商店 |
| 关闭商店 | `关闭商店` | 关闭商店 |
| 商店改名 | `商店改名+新名称` | 商店改名 |

### 排行榜指令
| 指令 | 格式 | 说明 |
|------|------|------|
| 等级排行 | `等级排行` | 等级排行榜 |
| 生命排行 | `生命排行` | 生命排行榜 |
| 魔法排行 | `魔法排行` | 魔法排行榜 |
| 物攻排行 | `物攻排行` | 物攻排行榜 |
| 魔攻排行 | `魔攻排行` | 魔攻排行榜 |
| 防御排行 | `防御排行` | 防御排行榜 |
| 魔抗排行 | `魔抗排行` | 魔抗排行榜 |
| 金币排行 | `金币排行` | 金币排行榜 |
| 钻石排行 | `钻石排行` | 钻石排行榜 |
""")

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860, share=False)
