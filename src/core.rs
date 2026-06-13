/// CakeGame 核心数据结构
/// 基于 gamedata.sdb 数据库结构
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 自定义反序列化：支持 JSON 中数字以字符串形式存储 (如 "100" → 100)
fn de_str_to_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Visitor;
    use std::fmt;

    struct StrOrI32;

    impl<'de> Visitor<'de> for StrOrI32 {
        type Value = i32;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("integer or string integer")
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<i32, E> {
            Ok(v as i32)
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<i32, E> {
            Ok(v as i32)
        }

        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<i32, E> {
            Ok(v as i32)
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<i32, E> {
            v.parse().or_else(|_| {
                // 尝试解析为 f64 再转 i32
                v.parse::<f64>().map(|f| f as i32).map_err(serde::de::Error::custom)
            })
        }
    }

    deserializer.deserialize_any(StrOrI32)
}

// ==================== 常量定义 ====================

// 基础信息节点
pub const NODE_BASIC: &str = "基础信息";
pub const NODE_CURRENCY: &str = "货币";
pub const NODE_USER_DATA: &str = "用户数据";
#[allow(dead_code)]
pub const NODE_EQUIP: &str = "装备";

// 基础信息项
pub const ITEM_NAME: &str = "名称";
pub const ITEM_LEVEL: &str = "等级";
pub const ITEM_OCCUPATION: &str = "职业";
pub const ITEM_LOCATION: &str = "位置";
pub const ITEM_TARGET: &str = "目标";
pub const ITEM_TASK: &str = "当前任务";
pub const ITEM_GUILD: &str = "公会";
pub const ITEM_HP: &str = "生命";
pub const ITEM_HP_CURRENT: &str = "生命_剩余";
pub const ITEM_MP: &str = "魔法";
pub const ITEM_MP_CURRENT: &str = "魔法_剩余";
pub const ITEM_EXP: &str = "经验";
pub const ITEM_EXP_NEED: &str = "经验_需求";
pub const ITEM_AD: &str = "物攻";
pub const ITEM_AP: &str = "魔攻";
pub const ITEM_DEFENSE: &str = "防御";
pub const ITEM_MAGIC_RES: &str = "魔抗";
pub const ITEM_HIT: &str = "命中";
pub const ITEM_DODGE: &str = "闪避";
pub const ITEM_CRIT: &str = "暴击";
pub const ITEM_ABSORB_HP: &str = "吸血";
pub const ITEM_AD_PTV: &str = "物穿";
pub const ITEM_AD_PTR: &str = "物穿比";
pub const ITEM_AP_PTV: &str = "法穿";
pub const ITEM_AP_PTR: &str = "法穿比";
pub const ITEM_IMMUNE: &str = "免伤";

// 货币
pub const CURRENCY_GOLD: &str = "金币";
pub const CURRENCY_DIAMOND: &str = "钻石";

// 物品类型
#[allow(dead_code)]
pub const TYPE_EQUIP: &str = "装备";
#[allow(dead_code)]
pub const TYPE_POTION: &str = "药剂";
#[allow(dead_code)]
pub const TYPE_MATERIAL: &str = "材料";
#[allow(dead_code)]
pub const TYPE_PACKAGE: &str = "礼包";
#[allow(dead_code)]
pub const TYPE_SKILL_BOOK: &str = "技能书";
#[allow(dead_code)]
pub const TYPE_CURRENCY_BAG: &str = "货币袋";

// 装备槽位
pub const SLOT_WEAPON: &str = "武器";
pub const SLOT_HELMET: &str = "头盔";
pub const SLOT_ARMOR: &str = "铠甲";
pub const SLOT_LEG: &str = "护腿";
pub const SLOT_BOOTS: &str = "靴子";
pub const SLOT_NECKLACE: &str = "项链";
pub const SLOT_RING: &str = "戒指";
pub const SLOT_WING: &str = "翅膀";
pub const SLOT_FASHION: &str = "时装";
pub const SLOT_TITLE: &str = "称号";

// 操作类型
pub const OP_ADD: &str = "增加";
pub const OP_SUB: &str = "减少";
pub const OP_SET: &str = "设置";

// 空值
pub const EMPTY: &str = "";

// ==================== 数据结构 ====================

/// 用户基础信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub level: i32,
    pub occupation: String,
    pub location: String,
    pub target: String,
    pub task: String,
    pub guild: String,
    pub hp: i32,
    pub hp_max: i32,
    pub mp: i32,
    pub mp_max: i32,
    pub exp: i64,
    pub exp_need: i64,
    pub ad: i32,
    pub ap: i32,
    pub defense: i32,
    pub magic_res: i32,
    pub hit: i32,
    pub dodge: i32,
    pub crit: i32,
    pub absorb_hp: i32,
    pub ad_ptv: i32,
    pub ad_ptr: i32,
    pub ap_ptv: i32,
    pub ap_ptr: i32,
    pub immune: i32,
    pub shield: i32,
    pub gold: i64,
    pub diamond: i64,
}

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            level: 1,
            occupation: String::new(),
            location: String::new(),
            target: String::new(),
            task: String::new(),
            guild: String::new(),
            hp: 0,
            hp_max: 0,
            mp: 0,
            mp_max: 0,
            exp: 0,
            exp_need: 100,
            ad: 0,
            ap: 0,
            defense: 0,
            magic_res: 0,
            hit: 0,
            dodge: 0,
            crit: 0,
            absorb_hp: 0,
            ad_ptv: 0,
            ad_ptr: 0,
            ap_ptv: 0,
            ap_ptr: 0,
            immune: 0,
            shield: 0,
            gold: 0,
            diamond: 0,
        }
    }
}

/// 物品定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub basic: bool,
    pub locked: bool,
    pub introduce: String,
    pub data: ItemData,
}

/// 物品数据（从 LtemData JSON 解析）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ItemData {
    #[serde(default)]
    pub slot_name: String,
    #[serde(default)]
    pub occupation: String,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub use_lv: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_hp: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_mp: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_defense: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_magic: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_ad: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_ap: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_hit: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_dodge: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_crit: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub special_value: i32,
    #[serde(default)]
    pub special_type: String,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_absorb_hp: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_immune_damage: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_adptr: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_apptr: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_adptv: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub add_apptv: i32,
    #[serde(default)]
    pub s_type: String,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub effect: i32,
    #[serde(default)]
    pub role: String,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub cd: i32,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub b_continued: i32,
    #[serde(default)]
    pub occupation_limit: String,
    #[serde(default, deserialize_with = "de_str_to_i32")]
    pub lv_limit: i32,
}

/// 怪物定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDef {
    pub name: String,
    pub monster_type: String,
    pub ad: i32,
    pub ap: i32,
    pub hp: i32,
    pub defense: i32,
    pub absorb_hp: i32,
    pub adptv: i32,
    pub adptr: i32,
    pub apptr: i32,
    pub apptv: i32,
    pub immune_damage: i32,
    pub skills: Vec<String>,
    pub reward_goods: Vec<RewardItem>,
    pub reward_exp: i32,
    pub reward_gold: i32,
    pub introduce: String,
    pub attack_effect: String,
    pub attack_tips: String,
    pub magic_resistance: i32,
    pub hit: i32,
    pub dodge: i32,
    pub ignore_shield: bool,
}

/// 奖励物品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardItem {
    pub name: String,
    pub count: i32,
    pub rate: f64,
}

/// 地图定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDef {
    pub name: String,
    pub level: i32,
    pub introduce: String,
    pub security: bool,
    pub monsters: Vec<String>,
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub consume: String,
}

/// 技能定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub skill_type: String,
    pub consume: i32,
    pub effect: i32,
    pub effect_other: String,
    pub level: i32,
    pub consume_type: String,
    pub cooling: i32,
    pub accurate: i32,
    pub attack_tips: String,
    pub introduce: String,
    pub acs: String,
    pub shield: i32,
    pub ignore_shield: bool,
    pub ignore_immune: bool,
    pub ignore_re: bool,
    pub ban_absorb: bool,
    pub ban_multiple_shot: bool,
    pub prohibit_uo: bool,
    pub consumable_goods: String,
    pub continued_round: i32,
    pub continued_type: String,
    pub continued_effect: String,
}

/// 职业定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupationDef {
    pub name: String,
    pub basics: String,
    pub hp: i32,
    pub mp: i32,
    pub ad: i32,
    pub ap: i32,
    pub defense: i32,
    pub hit: i32,
    pub dodge: i32,
    pub crit: i32,
    pub absorb_hp: i32,
    pub adptv: i32,
    pub adptr: i32,
    pub apptr: i32,
    pub apptv: i32,
    pub immune_damage: i32,
    pub intro: String,
    pub exclusive_skills: Vec<String>,
    pub transfer_demand: String,
    pub transfer_level: i32,
    pub former_occupation: String,
    pub belong: String,
    pub attack_effect: String,
    pub attack_tips: String,
    pub magic_resistance: i32,
    pub ignore_shield: bool,
}

/// 背包物品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnapsackItem {
    pub name: String,
    pub quantity: i32,
}

/// 装备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipInfo {
    pub slot: String,
    pub name: String,
    pub add_hp: i32,
    pub add_mp: i32,
    pub add_defense: i32,
    pub add_magic: i32,
    pub add_ad: i32,
    pub add_ap: i32,
    pub add_hit: i32,
    pub add_dodge: i32,
    pub add_crit: i32,
    pub add_absorb_hp: i32,
    pub add_adptv: i32,
    pub add_adptr: i32,
    pub add_apptr: i32,
    pub add_apptv: i32,
    pub add_immune_damage: i32,
    pub special_type: String,
    pub special_value: i32,
}

/// 指令定义
#[derive(Debug, Clone)]
pub struct CommandDef {
    pub name: String,
    pub trigger: String,
    pub enabled: bool,
    pub handler_name: String,
}

/// 消息模板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MessageTemplate {
    pub name: String,
    pub data: String,
}

/// 全局配置
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GlobalConfig {
    pub trigger_prefix: String,
    pub name_limit: i32,
    pub default_occupation: String,
    pub default_city: String,
    pub novice_reward: String,
    pub max_auto_attack: i32,
    pub sign_reward: String,
    pub attribute_aliases: HashMap<String, String>,
    pub currency_aliases: HashMap<String, String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            trigger_prefix: String::new(),
            name_limit: 12,
            default_occupation: "战士".to_string(),
            default_city: "主城".to_string(),
            novice_reward: "药剂礼包*1".to_string(),
            max_auto_attack: 6,
            sign_reward: String::new(),
            attribute_aliases: HashMap::new(),
            currency_aliases: HashMap::new(),
        }
    }
}

/// 战斗结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CombatResult {
    pub success: bool,
    pub user_hp_lost: i32,
    pub monster_hp_lost: i32,
    pub user_hp_remaining: i32,
    pub monster_hp_remaining: i32,
    pub is_critical: bool,
    pub is_dodged: bool,
    pub absorb_hp: i32,
    pub damage_dealt: i32,
    pub damage_received: i32,
    pub rewards: CombatRewards,
    pub log: Vec<String>,
    pub is_finished: bool,
    pub user_won: bool,
}

/// 战斗奖励
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct CombatRewards {
    pub exp: i32,
    pub gold: i32,
    pub items: Vec<RewardItem>,
}

/// 排行条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankEntry {
    pub rank: i32,
    pub user_id: String,
    pub name: String,
    pub value: String,
}

/// 任务定义 (Config_Task 表)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub title: String,
    pub level_min: i32,
    pub level_max: i32,
    pub task_type: String,
    pub reset_time: i32,       // -1 = 一次性任务, >0 = 可重复
    pub reset_type: i32,       // 重置类型 (天/小时等)
    pub complete_task: String, // 前置任务
    pub occupation: String,    // 职业限制
    pub target_type: String,   // 任务对象类型 (Monster/Goods)
    pub data: String,          // 任务数据 (config格式)
    pub reward_gold: i64,
    pub reward_diamond: i64,
    pub reward_exp: i64,
    pub reward_goods: Vec<RewardItem>,
    pub info: String,
}

/// 任务登记记录 (Task_Register 表)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TaskRecord {
    pub user: String,
    pub task_name: String,
    pub date: String,
    pub target: String,
    pub data: String,
    pub complete: bool,
}

/// 私人商店 (Config_PrivateShops 表)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateShop {
    pub owner: String,
    pub name: String,
    pub goods_data: String,
    pub open: bool,
}

/// 操作结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpResult {
    pub success: bool,
    pub message: String,
}

impl OpResult {
    #[allow(dead_code)]
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
        }
    }

    #[allow(dead_code)]
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: msg.into(),
        }
    }
}

/// 烹饪配方
#[derive(Debug, Clone, Default)]
pub struct CookingRecipe {
    pub name: String,
    pub time: i32,
    pub foodstuff: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_userinfo_default() {
        let u = UserInfo::default();
        assert_eq!(u.id, "");
        assert_eq!(u.name, "");
        assert_eq!(u.level, 1);
        assert_eq!(u.hp, 0);
        assert_eq!(u.hp_max, 0);
        assert_eq!(u.mp, 0);
        assert_eq!(u.mp_max, 0);
        assert_eq!(u.exp, 0);
        assert_eq!(u.exp_need, 100);
        assert_eq!(u.gold, 0);
        assert_eq!(u.diamond, 0);
        assert_eq!(u.ad, 0);
        assert_eq!(u.ap, 0);
        assert_eq!(u.defense, 0);
        assert_eq!(u.magic_res, 0);
        assert_eq!(u.hit, 0);
        assert_eq!(u.dodge, 0);
        assert_eq!(u.crit, 0);
        assert_eq!(u.absorb_hp, 0);
        assert_eq!(u.ad_ptv, 0);
        assert_eq!(u.ad_ptr, 0);
        assert_eq!(u.ap_ptv, 0);
        assert_eq!(u.ap_ptr, 0);
        assert_eq!(u.immune, 0);
        assert_eq!(u.shield, 0);
    }

    #[test]
    fn test_itemdata_default() {
        let d = ItemData::default();
        assert_eq!(d.slot_name, "");
        assert_eq!(d.occupation, "");
        assert_eq!(d.use_lv, 0);
        assert_eq!(d.add_hp, 0);
        assert_eq!(d.add_mp, 0);
        assert_eq!(d.add_defense, 0);
        assert_eq!(d.add_ad, 0);
        assert_eq!(d.add_ap, 0);
        assert_eq!(d.special_type, "");
        assert_eq!(d.s_type, "");
    }

    #[test]
    fn test_op_result_ok() {
        let r = OpResult::ok("success");
        assert!(r.success);
        assert_eq!(r.message, "success");
    }

    #[test]
    fn test_op_result_err() {
        let r = OpResult::err("failure");
        assert!(!r.success);
        assert_eq!(r.message, "failure");
    }

    #[test]
    fn test_node_constants() {
        assert_eq!(NODE_BASIC, "基础信息");
        assert_eq!(NODE_CURRENCY, "货币");
        assert_eq!(NODE_USER_DATA, "用户数据");
        assert_eq!(NODE_EQUIP, "装备");
    }

    #[test]
    fn test_item_constants() {
        assert_eq!(ITEM_NAME, "名称");
        assert_eq!(ITEM_LEVEL, "等级");
        assert_eq!(ITEM_OCCUPATION, "职业");
        assert_eq!(ITEM_HP, "生命");
        assert_eq!(ITEM_MP, "魔法");
        assert_eq!(ITEM_EXP, "经验");
        assert_eq!(ITEM_AD, "物攻");
        assert_eq!(ITEM_AP, "魔攻");
        assert_eq!(ITEM_DEFENSE, "防御");
        assert_eq!(ITEM_CRIT, "暴击");
        assert_eq!(ITEM_IMMUNE, "免伤");
    }

    #[test]
    fn test_currency_constants() {
        assert_eq!(CURRENCY_GOLD, "金币");
        assert_eq!(CURRENCY_DIAMOND, "钻石");
    }

    #[test]
    fn test_slot_constants() {
        assert_eq!(SLOT_WEAPON, "武器");
        assert_eq!(SLOT_HELMET, "头盔");
        assert_eq!(SLOT_ARMOR, "铠甲");
        assert_eq!(SLOT_LEG, "护腿");
        assert_eq!(SLOT_BOOTS, "靴子");
        assert_eq!(SLOT_NECKLACE, "项链");
        assert_eq!(SLOT_RING, "戒指");
        assert_eq!(SLOT_WING, "翅膀");
        assert_eq!(SLOT_FASHION, "时装");
        assert_eq!(SLOT_TITLE, "称号");
    }

    #[test]
    fn test_op_constants() {
        assert_eq!(OP_ADD, "增加");
        assert_eq!(OP_SUB, "减少");
        assert_eq!(OP_SET, "设置");
    }

    #[test]
    fn test_type_constants() {
        assert_eq!(TYPE_EQUIP, "装备");
        assert_eq!(TYPE_POTION, "药剂");
        assert_eq!(TYPE_MATERIAL, "材料");
        assert_eq!(TYPE_PACKAGE, "礼包");
        assert_eq!(TYPE_SKILL_BOOK, "技能书");
        assert_eq!(TYPE_CURRENCY_BAG, "货币袋");
    }

    #[test]
    fn test_combat_rewards_default() {
        let r = CombatRewards::default();
        assert_eq!(r.exp, 0);
        assert_eq!(r.gold, 0);
        assert!(r.items.is_empty());
    }

    #[test]
    fn test_global_config_default() {
        let c = GlobalConfig::default();
        assert_eq!(c.name_limit, 12);
        assert_eq!(c.default_occupation, "战士");
        assert_eq!(c.default_city, "主城");
        assert_eq!(c.novice_reward, "药剂礼包*1");
        assert_eq!(c.max_auto_attack, 6);
    }

    #[test]
    fn test_reward_item_serde_roundtrip() {
        let item = RewardItem {
            name: "测试药剂".to_string(),
            count: 5,
            rate: 25.5,
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: RewardItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "测试药剂");
        assert_eq!(parsed.count, 5);
        assert!((parsed.rate - 25.5).abs() < 0.01);
    }

    #[test]
    fn test_knapsack_item_serde_roundtrip() {
        let item = KnapsackItem {
            name: "初级药剂".to_string(),
            quantity: 10,
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: KnapsackItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "初级药剂");
        assert_eq!(parsed.quantity, 10);
    }

    #[test]
    fn test_cooking_recipe_default() {
        let r = CookingRecipe::default();
        assert_eq!(r.name, "");
        assert_eq!(r.time, 0);
        assert_eq!(r.foodstuff, "");
    }

    #[test]
    fn test_userinfo_serde_roundtrip() {
        let mut u = UserInfo::default();
        u.id = "test123".to_string();
        u.name = "测试玩家".to_string();
        u.level = 42;
        u.hp_max = 5000;
        u.gold = 100000;
        u.diamond = 500;
        let json = serde_json::to_string(&u).unwrap();
        let parsed: UserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test123");
        assert_eq!(parsed.name, "测试玩家");
        assert_eq!(parsed.level, 42);
        assert_eq!(parsed.hp_max, 5000);
        assert_eq!(parsed.gold, 100000);
        assert_eq!(parsed.diamond, 500);
    }
}
