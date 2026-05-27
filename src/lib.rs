/// CakeGame Rust - FFI 接口
/// 基于原版 gamedata.sdb 的完整 RPG 引擎

pub mod core;
pub mod editor;
pub mod db_loader;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use core::GameEngine;

static mut ENGINE: Option<GameEngine> = None;

unsafe fn engine() -> &'static mut GameEngine {
    ENGINE.get_or_insert_with(GameEngine::new)
}

fn from_c(s: *const c_char) -> String {
    if s.is_null() { return String::new(); }
    unsafe { CStr::from_ptr(s).to_string_lossy().into_owned() }
}

fn to_c(s: String) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

// ==================== 引擎初始化 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_Initialize() -> i32 {
    let _ = engine();
    0
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetVersion() -> i32 {
    5
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetVersionString() -> *mut c_char {
    to_c("0.5.0".to_string())
}

#[no_mangle]
pub unsafe extern "C" fn Cake_FreeString(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_Cleanup() {
    ENGINE = None;
}

// ==================== 数据库加载 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_LoadDatabase(db_path: *const c_char) -> i32 {
    let path = from_c(db_path);
    match db_loader::load_database(&path) {
        Ok(eng) => {
            ENGINE = Some(eng);
            0
        }
        Err(e) => {
            eprintln!("加载数据库失败: {}", e);
            -1
        }
    }
}

// ==================== 角色创建 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_CreateCharacter(name: *const c_char, occupation: *const c_char) -> i32 {
    let name = from_c(name);
    let occupation = from_c(occupation);
    match engine().create_character(&name, &occupation) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("创建角色失败: {}", e);
            -1
        }
    }
}

// ==================== 移动系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_MoveTo(map_name: *const c_char) -> i32 {
    let map_name = from_c(map_name);
    match engine().move_to(&map_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("移动失败: {}", e);
            -1
        }
    }
}

// ==================== 战斗系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_StartCombat(monster_name: *const c_char) -> i32 {
    let monster_name = from_c(monster_name);
    match engine().start_combat(&monster_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("战斗失败: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_Attack() -> i32 {
    match engine().attack() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("攻击失败: {}", e);
            -1
        }
    }
}

// ==================== 物品系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_AddItem(id: *const c_char, count: i32) -> i32 {
    let id = from_c(id);
    match engine().add_item(&id, count) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_RemoveItem(id: *const c_char, count: i32) -> i32 {
    let id = from_c(id);
    match engine().remove_item(&id, count) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetItemCount(id: *const c_char) -> i32 {
    let id = from_c(id);
    engine().get_item_count(&id)
}

#[no_mangle]
pub unsafe extern "C" fn Cake_UseItem(id: *const c_char) -> i32 {
    let id = from_c(id);
    match engine().use_item(&id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("使用物品失败: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_EquipItem(id: *const c_char) -> i32 {
    let id = from_c(id);
    match engine().equip_item(&id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("装备失败: {}", e);
            -1
        }
    }
}

// ==================== 任务系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_AcceptTask(task_id: *const c_char) -> i32 {
    let task_id = from_c(task_id);
    match engine().accept_task(&task_id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("接取任务失败: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_CompleteTask(task_id: *const c_char) -> i32 {
    let task_id = from_c(task_id);
    match engine().complete_task(&task_id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("完成任务失败: {}", e);
            -1
        }
    }
}

// ==================== 商店系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_BuyItem(shop_id: *const c_char, item_id: *const c_char, count: i32) -> i32 {
    let shop_id = from_c(shop_id);
    let item_id = from_c(item_id);
    match engine().buy_item(&shop_id, &item_id, count) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("购买失败: {}", e);
            -1
        }
    }
}

// ==================== 合成系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_Composite(recipe_id: *const c_char) -> i32 {
    let recipe_id = from_c(recipe_id);
    match engine().composite(&recipe_id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("合成失败: {}", e);
            -1
        }
    }
}

// ==================== 分解系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_Decompose(item_id: *const c_char) -> i32 {
    let item_id = from_c(item_id);
    match engine().decompose(&item_id) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("分解失败: {}", e);
            -1
        }
    }
}

// ==================== 食物系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_EatFood(food_name: *const c_char) -> i32 {
    let food_name = from_c(food_name);
    match engine().eat_food(&food_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("吃食物失败: {}", e);
            -1
        }
    }
}

// ==================== 技能系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_LearnSkill(skill_name: *const c_char) -> i32 {
    let skill_name = from_c(skill_name);
    match engine().learn_skill(&skill_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("学习技能失败: {}", e);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_UseSkill(skill_name: *const c_char) -> i32 {
    let skill_name = from_c(skill_name);
    match engine().use_skill(&skill_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("使用技能失败: {}", e);
            -1
        }
    }
}

// ==================== NPC系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_TalkToNpc(npc_name: *const c_char) -> i32 {
    let npc_name = from_c(npc_name);
    match engine().talk_to_npc(&npc_name) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("对话失败: {}", e);
            -1
        }
    }
}

// ==================== 消息轮询 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_PollMessage(
    msg_type: *mut i32,
    content: *mut *mut c_char,
    speaker: *mut *mut c_char,
) -> i32 {
    match engine().poll_message() {
        Some(msg) => {
            if !msg_type.is_null() { *msg_type = msg.msg_type as i32; }
            if !content.is_null() { *content = to_c(msg.content); }
            if !speaker.is_null() {
                *speaker = match msg.speaker {
                    Some(s) => to_c(s),
                    None => ptr::null_mut(),
                };
            }
            1
        }
        None => 0,
    }
}

// ==================== 变量/标记 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_SetVar(name: *const c_char, value: *const c_char) -> i32 {
    engine().set_var(&from_c(name), &from_c(value));
    0
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetVar(name: *const c_char) -> *mut c_char {
    to_c(engine().get_var(&from_c(name)))
}

#[no_mangle]
pub unsafe extern "C" fn Cake_SetFlag(name: *const c_char) -> i32 {
    engine().set_flag(&from_c(name));
    0
}

#[no_mangle]
pub unsafe extern "C" fn Cake_HasFlag(name: *const c_char) -> i32 {
    if engine().has_flag(&from_c(name)) { 1 } else { 0 }
}

// ==================== 存档系统 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_SaveState() -> *mut c_char {
    match engine().save_state() {
        Ok(json) => to_c(json),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn Cake_LoadState(json: *const c_char) -> i32 {
    match engine().load_state(&from_c(json)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ==================== 查询接口 ====================

#[no_mangle]
pub unsafe extern "C" fn Cake_GetUserInfo() -> *mut c_char {
    to_c(engine().get_user_info())
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetPosition() -> *mut c_char {
    to_c(engine().get_position())
}

#[no_mangle]
pub unsafe extern "C" fn Cake_GetHelp(topic: *const c_char) -> *mut c_char {
    let topic = from_c(topic);
    match engine().get_help(&topic) {
        Some(help) => to_c(help),
        None => ptr::null_mut(),
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::core::GameEngine;
    use super::db_loader;

    #[test]
    fn test_decode_hex_gbk() {
        assert_eq!(db_loader::decode_hex_gbk("4850"), "HP");
        assert_eq!(db_loader::decode_hex_gbk(""), "");
        assert_eq!(db_loader::decode_hex_gbk("[NULL]"), "");
    }

    #[test]
    fn test_item_system() {
        let mut engine = GameEngine::new();

        engine.items.insert("sword".to_string(), super::core::ItemDef {
            id: "sword".to_string(),
            name: "铁剑".to_string(),
            item_type: super::core::ItemType::Equip,
            basic: false,
            locked: false,
            introduce: "一把普通的铁剑".to_string(),
            data: super::core::ItemData {
                slot_name: "武器".to_string(),
                add_ad: 10,
                ..Default::default()
            },
        });

        assert!(engine.add_item("sword", 1).is_ok());
        assert_eq!(engine.get_item_count("sword"), 1);

        assert!(engine.equip_item("sword").is_ok());
        assert_eq!(engine.user.ad, 20);
    }

    #[test]
    fn test_combat_system() {
        let mut engine = GameEngine::new();

        engine.monsters.insert("史莱姆".to_string(), super::core::MonsterDef {
            name: "史莱姆".to_string(),
            monster_type: super::core::MonsterType::Ordinary,
            ad: 10,
            ap: 0,
            hp: 100,
            defense: 5,
            absorb_hp: 0,
            adptv: 0,
            adptr: 0,
            apptr: 0,
            apptv: 0,
            immune_damage: 0,
            skills: Vec::new(),
            reward_goods: Vec::new(),
            reward_exp: 50,
            reward_gold: 20,
            introduce: "普通史莱姆".to_string(),
            attack_effect: String::new(),
            attack_tips: String::new(),
            magic_resistance: 0,
            hit: 80,
            dodge: 0,
            ignore_shield: false,
        });

        assert!(engine.start_combat("史莱姆").is_ok());
        assert!(engine.state.in_combat);

        assert!(engine.attack().is_ok());
    }

    #[test]
    fn test_skill_system() {
        let mut engine = GameEngine::new();

        engine.skills.insert("火球术".to_string(), super::core::SkillDef {
            name: "火球术".to_string(),
            hp: 0,
            mp: 10,
            ad: 0,
            ap: 50,
            def: 0,
            mdf: 0,
            hit: 100,
            sb: 0,
            xx: 0,
            bj: 0,
            ms: 0,
            wz: String::new(),
            cdr: 5,
            combo: 0,
            combo_time: 0,
            combo_need: 0,
        });

        assert!(engine.learn_skill("火球术").is_ok());
        assert!(engine.user.skills.contains(&"火球术".to_string()));

        engine.user.mp = 50;
        assert!(engine.use_skill("火球术").is_ok());
        assert_eq!(engine.user.mp, 40);
    }

    #[test]
    fn test_npc_system() {
        let mut engine = GameEngine::new();

        engine.npcs.insert("村长".to_string(), super::core::NpcDef {
            name: "村长".to_string(),
            location: "新手村".to_string(),
            function: "对话".to_string(),
            dialog: "欢迎来到新手村！".to_string(),
            introduce: "村长是一个和蔼的老人".to_string(),
        });

        assert!(engine.talk_to_npc("村长").is_ok());
        let msg = engine.poll_message().unwrap();
        assert_eq!(msg.content, "欢迎来到新手村！");
    }

    #[test]
    fn test_food_system() {
        let mut engine = GameEngine::new();

        engine.foods.insert("面包".to_string(), super::core::FoodDef {
            name: "面包".to_string(),
            value: 30,
            price: 10,
        });

        engine.user.hunger = 50;
        engine.user.knapsack.insert("面包".to_string(), 1);

        assert!(engine.eat_food("面包").is_ok());
        assert_eq!(engine.user.hunger, 80);
    }

    #[test]
    fn test_decomposition_system() {
        let mut engine = GameEngine::new();

        // 先添加物品到items列表
        engine.items.insert("铁剑".to_string(), super::core::ItemDef {
            id: "铁剑".to_string(),
            name: "铁剑".to_string(),
            item_type: super::core::ItemType::Equip,
            basic: false,
            locked: false,
            introduce: "".to_string(),
            data: super::core::ItemData::default(),
        });

        engine.items.insert("铁矿石".to_string(), super::core::ItemDef {
            id: "铁矿石".to_string(),
            name: "铁矿石".to_string(),
            item_type: super::core::ItemType::Material,
            basic: false,
            locked: false,
            introduce: "".to_string(),
            data: super::core::ItemData::default(),
        });

        engine.decompositions.insert("铁剑".to_string(), super::core::DecompositionDef {
            goods: "铁剑".to_string(),
            need_gold: 10,
            need_diamond: 0,
            get_goods: "铁矿石".to_string(),
            success_rate: 100,
        });

        engine.user.gold = 100;
        engine.user.knapsack.insert("铁剑".to_string(), 1);

        assert!(engine.decompose("铁剑").is_ok());
        assert_eq!(engine.get_item_count("铁矿石"), 1);
    }
}
