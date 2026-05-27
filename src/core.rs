/// CakeGame 核心数据结构
/// 基于原版 gamedata.sdb 数据库结构 - 完整版

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== 物品系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    pub item_type: ItemType,
    pub basic: bool,
    pub locked: bool,
    pub introduce: String,
    pub data: ItemData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemType {
    Equip,
    Potion,
    Material,
    Quest,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    pub slot_name: String,
    pub occupation: String,
    pub use_lv: i32,
    pub add_hp: i32,
    pub add_mp: i32,
    pub add_defense: i32,
    pub add_magic: i32,
    pub add_ad: i32,
    pub add_ap: i32,
    pub add_hit: i32,
    pub add_dodge: i32,
    pub add_crit: i32,
    pub special_value: i32,
    pub special_type: String,
    pub add_absorb_hp: i32,
    pub add_immune_damage: i32,
    pub add_adptr: i32,
    pub add_apptr: i32,
    pub add_adptv: i32,
    pub add_apptv: i32,
    pub s_type: String,
    pub effect: i32,
    pub role: String,
    pub cd: i32,
    pub b_continued: i32,
}

impl Default for ItemData {
    fn default() -> Self {
        Self {
            slot_name: String::new(),
            occupation: String::new(),
            use_lv: 0,
            add_hp: 0, add_mp: 0, add_defense: 0, add_magic: 0,
            add_ad: 0, add_ap: 0, add_hit: 0, add_dodge: 0, add_crit: 0,
            special_value: 0, special_type: String::new(),
            add_absorb_hp: 0, add_immune_damage: 0,
            add_adptr: 0, add_apptr: 0, add_adptv: 0, add_apptv: 0,
            s_type: String::new(), effect: 0, role: String::new(),
            cd: 0, b_continued: 0,
        }
    }
}

// ==================== 怪物系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDef {
    pub name: String,
    pub monster_type: MonsterType,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MonsterType {
    Ordinary,
    Elite,
    Boss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardItem {
    pub item_id: String,
    pub count: i32,
    pub rate: f32,
}

// ==================== 地图系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDef {
    pub name: String,
    pub lv: i32,
    pub introduce: String,
    pub security: bool,
    pub hid: bool,
    pub basis: bool,
    pub monsters: Vec<MapMonster>,
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub consume: Vec<ConsumeItem>,
    pub lv_up: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapMonster {
    pub name: String,
    pub appear_time: String,
    pub disappear_time: String,
    pub number: i32,
    pub reborn: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumeItem {
    pub item_id: String,
    pub number: i32,
    pub consume: bool,
}

// ==================== 技能系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub hp: i32,
    pub mp: i32,
    pub ad: i32,
    pub ap: i32,
    pub def: i32,
    pub mdf: i32,
    pub hit: i32,
    pub sb: i32,
    pub xx: i32,
    pub bj: i32,
    pub ms: i32,
    pub wz: String,
    pub cdr: i32,
    pub combo: i32,
    pub combo_time: i32,
    pub combo_need: i32,
}

// ==================== 任务系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub title: String,
    pub lv: i32,
    pub task_type: TaskType,
    pub reset_time: i32,
    pub reset_type: i32,
    pub complete_task: String,
    pub occupation: String,
    pub target: TaskTarget,
    pub reward_gold: i32,
    pub reward_diamonds: i32,
    pub reward_exp: i32,
    pub reward_goods: Vec<RewardItem>,
    pub info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Main,
    Branch,
    Daily,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTarget {
    pub target_type: String,
    pub target_id: String,
    pub count: i32,
}

// ==================== 商店系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopItem {
    pub name: String,
    pub currency: String,
    pub price: i32,
    pub limit_number: i32,
    pub limit_type: String,
}

// ==================== 合成系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeRecipe {
    pub produce: String,
    pub consume_goods: HashMap<String, i32>,
    pub consume_gold: i32,
    pub consume_diamond: i32,
    pub success_rate: i32,
}

// ==================== 套装系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuitDef {
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

// ==================== 工会系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionDef {
    pub name: String,
    pub lv: i32,
    pub exp: i32,
    pub need_exp: i32,
    pub cdr: i64,
    pub limit_lv: i32,
    pub apply_list: Vec<String>,
}

// ==================== 职业系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupationDef {
    pub name: String,
    pub base_hp: i32,
    pub base_mp: i32,
    pub base_ad: i32,
    pub base_ap: i32,
    pub base_defense: i32,
    pub base_magic: i32,
    pub growth_hp: i32,
    pub growth_mp: i32,
    pub growth_ad: i32,
    pub growth_ap: i32,
    pub growth_defense: i32,
    pub growth_magic: i32,
}

// ==================== NPC系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDef {
    pub name: String,
    pub location: String,
    pub function: String,
    pub dialog: String,
    pub introduce: String,
}

// ==================== 分解系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionDef {
    pub goods: String,
    pub need_gold: i32,
    pub need_diamond: i32,
    pub get_goods: String,
    pub success_rate: i32,
}

// ==================== 私人商店 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateShopDef {
    pub id: String,
    pub name: String,
    pub goods_data: String,
    pub open: bool,
}

// ==================== 食物系统 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodDef {
    pub name: String,
    pub value: i32,
    pub price: i32,
}

// ==================== 玩家数据 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub occupation: String,
    pub lv: i32,
    pub exp: i32,
    pub gold: i32,
    pub diamonds: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub ad: i32,
    pub ap: i32,
    pub defense: i32,
    pub magic: i32,
    pub hit: i32,
    pub dodge: i32,
    pub crit: i32,
    pub position: String,
    pub equip: HashMap<String, String>,
    pub knapsack: HashMap<String, i32>,
    pub tasks: Vec<ActiveTask>,
    pub flags: Vec<String>,
    pub variables: HashMap<String, String>,
    pub skills: Vec<String>,
    pub hunger: i32,
    pub max_hunger: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTask {
    pub task_id: String,
    pub progress: i32,
    pub completed: bool,
}

// ==================== 游戏状态 ====================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameState {
    pub current_map: String,
    pub current_dialog: String,
    pub dialog_history: Vec<String>,
    pub choice_history: Vec<(String, String)>,
    pub total_dialogs_seen: u32,
    pub game_time: f64,
    pub in_combat: bool,
    pub current_monster: Option<String>,
}

// ==================== 消息类型 ====================

#[derive(Debug, Clone)]
pub struct GameMessage {
    pub msg_type: MessageType,
    pub content: String,
    pub speaker: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MessageType {
    Dialog = 0,
    Choice = 1,
    System = 2,
    Narrator = 3,
    End = 4,
    Error = 5,
    MapChange = 6,
    ItemGet = 7,
    ItemUse = 8,
    AffinityChange = 9,
    Combat = 10,
    LevelUp = 11,
    GoldChange = 12,
    ExpChange = 13,
    SkillUse = 14,
    Decomposition = 15,
    HungerChange = 16,
}

// ==================== 引擎核心 ====================

pub struct GameEngine {
    pub user: UserProfile,
    pub state: GameState,
    pub message_queue: Vec<GameMessage>,
    pub game_started: bool,

    // 游戏配置数据 - 所有表
    pub items: HashMap<String, ItemDef>,
    pub monsters: HashMap<String, MonsterDef>,
    pub maps: HashMap<String, MapDef>,
    pub skills: HashMap<String, SkillDef>,
    pub tasks: HashMap<String, TaskDef>,
    pub shops: HashMap<String, Vec<ShopItem>>,
    pub composites: HashMap<String, CompositeRecipe>,
    pub suits: HashMap<String, SuitDef>,
    pub occupations: HashMap<String, OccupationDef>,
    pub npcs: HashMap<String, NpcDef>,
    pub decompositions: HashMap<String, DecompositionDef>,
    pub helps: HashMap<String, String>,
    pub private_shops: HashMap<String, PrivateShopDef>,
    pub foods: HashMap<String, FoodDef>,
}

impl GameEngine {
    pub fn new() -> Self {
        Self {
            user: UserProfile::default(),
            state: GameState::default(),
            message_queue: Vec::new(),
            game_started: false,
            items: HashMap::new(),
            monsters: HashMap::new(),
            maps: HashMap::new(),
            skills: HashMap::new(),
            tasks: HashMap::new(),
            shops: HashMap::new(),
            composites: HashMap::new(),
            suits: HashMap::new(),
            occupations: HashMap::new(),
            npcs: HashMap::new(),
            decompositions: HashMap::new(),
            helps: HashMap::new(),
            private_shops: HashMap::new(),
            foods: HashMap::new(),
        }
    }

    /// 创建新角色
    pub fn create_character(&mut self, name: &str, occupation: &str) -> Result<(), String> {
        let occ = self.occupations.get(occupation)
            .ok_or(format!("职业不存在: {}", occupation))?;

        self.user = UserProfile {
            id: generate_id(),
            name: name.to_string(),
            occupation: occupation.to_string(),
            lv: 1,
            exp: 0,
            gold: 100,
            diamonds: 0,
            hp: occ.base_hp,
            max_hp: occ.base_hp,
            mp: occ.base_mp,
            max_mp: occ.base_mp,
            ad: occ.base_ad,
            ap: occ.base_ap,
            defense: occ.base_defense,
            magic: occ.base_magic,
            hit: 100,
            dodge: 0,
            crit: 0,
            position: "新手村".to_string(),
            equip: HashMap::new(),
            knapsack: HashMap::new(),
            tasks: Vec::new(),
            flags: Vec::new(),
            variables: HashMap::new(),
            skills: Vec::new(),
            hunger: 100,
            max_hunger: 100,
        };

        Ok(())
    }

    // ==================== 物品系统 ====================

    pub fn add_item(&mut self, item_id: &str, count: i32) -> Result<(), String> {
        if !self.items.contains_key(item_id) {
            return Err(format!("物品不存在: {}", item_id));
        }

        let current = self.user.knapsack.get(item_id).copied().unwrap_or(0);
        self.user.knapsack.insert(item_id.to_string(), current + count);

        let item_name = self.items.get(item_id).map(|i| i.name.clone()).unwrap_or_default();
        self.message_queue.push(GameMessage {
            msg_type: MessageType::ItemGet,
            content: format!("获得 {} x{}", item_name, count),
            speaker: None,
        });

        Ok(())
    }

    pub fn remove_item(&mut self, item_id: &str, count: i32) -> Result<(), String> {
        let current = self.user.knapsack.get(item_id).copied().unwrap_or(0);
        if current < count {
            return Err(format!("物品不足: {} (有{}, 需要{})", item_id, current, count));
        }

        self.user.knapsack.insert(item_id.to_string(), current - count);
        if self.user.knapsack[item_id] == 0 {
            self.user.knapsack.remove(item_id);
        }

        Ok(())
    }

    pub fn get_item_count(&self, item_id: &str) -> i32 {
        self.user.knapsack.get(item_id).copied().unwrap_or(0)
    }

    pub fn use_item(&mut self, item_id: &str) -> Result<(), String> {
        let item = self.items.get(item_id).ok_or(format!("物品不存在: {}", item_id))?.clone();

        match item.item_type {
            ItemType::Potion => {
                if self.get_item_count(item_id) < 1 {
                    return Err("物品不足".to_string());
                }

                let effect = item.data.effect;
                match item.data.role.as_str() {
                    "HP" => {
                        self.user.hp = (self.user.hp + effect).min(self.user.max_hp);
                        self.message_queue.push(GameMessage {
                            msg_type: MessageType::ItemUse,
                            content: format!("恢复 {} HP", effect),
                            speaker: None,
                        });
                    }
                    "MP" => {
                        self.user.mp = (self.user.mp + effect).min(self.user.max_mp);
                        self.message_queue.push(GameMessage {
                            msg_type: MessageType::ItemUse,
                            content: format!("恢复 {} MP", effect),
                            speaker: None,
                        });
                    }
                    _ => {}
                }

                self.remove_item(item_id, 1)?;
            }
            _ => return Err("该物品不能直接使用".to_string()),
        }

        Ok(())
    }

    // ==================== 装备系统 ====================

    pub fn equip_item(&mut self, item_id: &str) -> Result<(), String> {
        let item = self.items.get(item_id).ok_or(format!("物品不存在: {}", item_id))?.clone();

        if item.item_type != ItemType::Equip {
            return Err("不是装备".to_string());
        }

        if self.get_item_count(item_id) < 1 {
            return Err("物品不足".to_string());
        }

        if item.data.use_lv > self.user.lv {
            return Err(format!("等级不足，需要 {} 级", item.data.use_lv));
        }

        if !item.data.occupation.is_empty() && item.data.occupation != "NULL" {
            if item.data.occupation != self.user.occupation {
                return Err("职业不符合".to_string());
            }
        }

        let slot = item.data.slot_name.clone();

        if let Some(old_item_id) = self.user.equip.get(&slot).cloned() {
            self.add_item(&old_item_id, 1)?;
            self.unequip_stats(&old_item_id);
        }

        self.remove_item(item_id, 1)?;
        self.user.equip.insert(slot, item_id.to_string());
        self.equip_stats(item_id);

        self.message_queue.push(GameMessage {
            msg_type: MessageType::System,
            content: format!("装备了 {}", item.name),
            speaker: None,
        });

        Ok(())
    }

    fn equip_stats(&mut self, item_id: &str) {
        if let Some(item) = self.items.get(item_id) {
            self.user.max_hp += item.data.add_hp;
            self.user.max_mp += item.data.add_mp;
            self.user.ad += item.data.add_ad;
            self.user.ap += item.data.add_ap;
            self.user.defense += item.data.add_defense;
            self.user.magic += item.data.add_magic;
            self.user.hit += item.data.add_hit;
            self.user.dodge += item.data.add_dodge;
            self.user.crit += item.data.add_crit;
        }
    }

    fn unequip_stats(&mut self, item_id: &str) {
        if let Some(item) = self.items.get(item_id) {
            self.user.max_hp -= item.data.add_hp;
            self.user.max_mp -= item.data.add_mp;
            self.user.ad -= item.data.add_ad;
            self.user.ap -= item.data.add_ap;
            self.user.defense -= item.data.add_defense;
            self.user.magic -= item.data.add_magic;
            self.user.hit -= item.data.add_hit;
            self.user.dodge -= item.data.add_dodge;
            self.user.crit -= item.data.add_crit;
        }
    }

    // ==================== 地图系统 ====================

    pub fn move_to(&mut self, map_name: &str) -> Result<(), String> {
        let map = self.maps.get(map_name).ok_or(format!("地图不存在: {}", map_name))?.clone();

        if map.lv > self.user.lv {
            return Err(format!("等级不足，需要 {} 级", map.lv));
        }

        for consume in &map.consume {
            if consume.consume {
                let count = self.get_item_count(&consume.item_id);
                if count < consume.number {
                    return Err(format!("缺少通行物品"));
                }
            }
        }

        for consume in &map.consume {
            if consume.consume {
                self.remove_item(&consume.item_id, consume.number)?;
            }
        }

        self.user.position = map_name.to_string();
        self.state.current_map = map_name.to_string();

        self.message_queue.push(GameMessage {
            msg_type: MessageType::MapChange,
            content: format!("{}: {}", map.name, map.introduce),
            speaker: None,
        });

        Ok(())
    }

    // ==================== 战斗系统 ====================

    pub fn start_combat(&mut self, monster_name: &str) -> Result<(), String> {
        let monster = self.monsters.get(monster_name)
            .ok_or(format!("怪物不存在: {}", monster_name))?
            .clone();

        self.state.in_combat = true;
        self.state.current_monster = Some(monster_name.to_string());

        self.message_queue.push(GameMessage {
            msg_type: MessageType::Combat,
            content: format!("遭遇了 {}！\nHP: {} | AD: {} | DEF: {}",
                monster.name, monster.hp, monster.ad, monster.defense),
            speaker: None,
        });

        Ok(())
    }

    pub fn attack(&mut self) -> Result<(), String> {
        if !self.state.in_combat {
            return Err("不在战斗中".to_string());
        }

        let monster_name = self.state.current_monster.clone().ok_or("没有对手")?;
        let monster = self.monsters.get(&monster_name).ok_or("怪物数据异常")?.clone();

        let player_damage = self.calculate_damage(self.user.ad, monster.defense, monster.adptv);
        let monster_damage = self.calculate_damage(monster.ad, self.user.defense, 0);

        let monster_hp = monster.hp - player_damage;

        self.message_queue.push(GameMessage {
            msg_type: MessageType::Combat,
            content: format!("你对 {} 造成了 {} 点伤害", monster.name, player_damage),
            speaker: None,
        });

        if monster_hp <= 0 {
            self.end_combat(true)?;
            return Ok(());
        }

        self.user.hp -= monster_damage;

        self.message_queue.push(GameMessage {
            msg_type: MessageType::Combat,
            content: format!("{} 对你造成了 {} 点伤害", monster.name, monster_damage),
            speaker: None,
        });

        if self.user.hp <= 0 {
            self.user.hp = 0;
            self.end_combat(false)?;
        }

        Ok(())
    }

    fn calculate_damage(&self, attack: i32, defense: i32, resistance: i32) -> i32 {
        let base = (attack as f64 * (1.0 - defense as f64 / (attack as f64 + defense as f64))) as i32;
        let after_resistance = (base as f64 * (1.0 - resistance as f64 / 100.0)) as i32;
        after_resistance.max(1)
    }

    fn end_combat(&mut self, victory: bool) -> Result<(), String> {
        let monster_name = self.state.current_monster.clone().ok_or("没有对手")?;
        let monster = self.monsters.get(&monster_name).ok_or("怪物数据异常")?.clone();

        self.state.in_combat = false;
        self.state.current_monster = None;

        if victory {
            self.user.exp += monster.reward_exp;
            self.message_queue.push(GameMessage {
                msg_type: MessageType::ExpChange,
                content: format!("获得 {} 经验", monster.reward_exp),
                speaker: None,
            });

            self.user.gold += monster.reward_gold;
            self.message_queue.push(GameMessage {
                msg_type: MessageType::GoldChange,
                content: format!("获得 {} 金币", monster.reward_gold),
                speaker: None,
            });

            for reward in &monster.reward_goods {
                let roll: f32 = rand::random::<f32>() * 100.0;
                if roll < reward.rate {
                    let _ = self.add_item(&reward.item_id, reward.count);
                }
            }

            self.check_level_up()?;

            self.message_queue.push(GameMessage {
                msg_type: MessageType::Combat,
                content: format!("击败了 {}！", monster.name),
                speaker: None,
            });
        } else {
            self.message_queue.push(GameMessage {
                msg_type: MessageType::Combat,
                content: "你被击败了...".to_string(),
                speaker: None,
            });

            self.user.hp = self.user.max_hp / 2;
            self.user.mp = self.user.max_mp / 2;
        }

        Ok(())
    }

    // ==================== 等级系统 ====================

    fn check_level_up(&mut self) -> Result<(), String> {
        let need_exp = self.calculate_need_exp(self.user.lv);

        while self.user.exp >= need_exp {
            self.user.exp -= need_exp;
            self.user.lv += 1;

            if let Some(occ) = self.occupations.get(&self.user.occupation).cloned() {
                self.user.max_hp += occ.growth_hp;
                self.user.max_mp += occ.growth_mp;
                self.user.ad += occ.growth_ad;
                self.user.ap += occ.growth_ap;
                self.user.defense += occ.growth_defense;
                self.user.magic += occ.growth_magic;
                self.user.hp = self.user.max_hp;
                self.user.mp = self.user.max_mp;
            }

            self.message_queue.push(GameMessage {
                msg_type: MessageType::LevelUp,
                content: format!("升级！现在是 {} 级", self.user.lv),
                speaker: None,
            });
        }

        Ok(())
    }

    fn calculate_need_exp(&self, lv: i32) -> i32 {
        100 * lv * lv
    }

    // ==================== 任务系统 ====================

    pub fn accept_task(&mut self, task_id: &str) -> Result<(), String> {
        let task = self.tasks.get(task_id).ok_or(format!("任务不存在: {}", task_id))?;

        if task.lv > self.user.lv {
            return Err(format!("等级不足，需要 {} 级", task.lv));
        }

        if !task.complete_task.is_empty() {
            if !self.user.flags.contains(&format!("task_{}", task.complete_task)) {
                return Err("前置任务未完成".to_string());
            }
        }

        if self.user.tasks.iter().any(|t| t.task_id == task_id) {
            return Err("任务已接取".to_string());
        }

        self.user.tasks.push(ActiveTask {
            task_id: task_id.to_string(),
            progress: 0,
            completed: false,
        });

        self.message_queue.push(GameMessage {
            msg_type: MessageType::System,
            content: format!("接取任务: {}", task.title),
            speaker: None,
        });

        Ok(())
    }

    pub fn complete_task(&mut self, task_id: &str) -> Result<(), String> {
        let task_idx = self.user.tasks.iter().position(|t| t.task_id == task_id && t.completed)
            .ok_or("任务未完成")?;

        let task = self.tasks.get(task_id).ok_or("任务数据异常")?.clone();

        self.user.gold += task.reward_gold;
        self.user.diamonds += task.reward_diamonds;
        self.user.exp += task.reward_exp;

        for reward in &task.reward_goods {
            let _ = self.add_item(&reward.item_id, reward.count);
        }

        self.user.tasks.remove(task_idx);
        self.user.flags.push(format!("task_{}", task_id));

        self.message_queue.push(GameMessage {
            msg_type: MessageType::System,
            content: format!("完成任务: {}！获得奖励", task.title),
            speaker: None,
        });

        self.check_level_up()?;

        Ok(())
    }

    // ==================== 商店系统 ====================

    pub fn buy_item(&mut self, shop_id: &str, item_id: &str, count: i32) -> Result<(), String> {
        let shop_items = self.shops.get(shop_id).ok_or("商店不存在")?;
        let shop_item = shop_items.iter().find(|i| i.name == item_id).ok_or("商品不存在")?;

        let total_price = shop_item.price * count;

        if self.user.gold < total_price {
            return Err("金币不足".to_string());
        }

        self.user.gold -= total_price;
        self.add_item(item_id, count)?;

        Ok(())
    }

    // ==================== 合成系统 ====================

    pub fn composite(&mut self, recipe_id: &str) -> Result<(), String> {
        let recipe = self.composites.get(recipe_id).ok_or("配方不存在")?.clone();

        for (item_id, count) in &recipe.consume_goods {
            if self.get_item_count(item_id) < *count {
                return Err(format!("材料不足: {}", item_id));
            }
        }

        if self.user.gold < recipe.consume_gold {
            return Err("金币不足".to_string());
        }

        for (item_id, count) in &recipe.consume_goods {
            self.remove_item(item_id, *count)?;
        }
        self.user.gold -= recipe.consume_gold;

        let roll: i32 = rand::random::<i32>() % 100;
        if roll < recipe.success_rate {
            self.add_item(&recipe.produce, 1)?;
            self.message_queue.push(GameMessage {
                msg_type: MessageType::System,
                content: "合成成功！".to_string(),
                speaker: None,
            });
        } else {
            self.message_queue.push(GameMessage {
                msg_type: MessageType::System,
                content: "合成失败...".to_string(),
                speaker: None,
            });
        }

        Ok(())
    }

    // ==================== 分解系统 ====================

    pub fn decompose(&mut self, item_id: &str) -> Result<(), String> {
        let decomposition = self.decompositions.get(item_id)
            .ok_or("该物品不能分解")?.clone();

        if self.get_item_count(item_id) < 1 {
            return Err("物品不足".to_string());
        }

        if self.user.gold < decomposition.need_gold {
            return Err("金币不足".to_string());
        }

        self.remove_item(item_id, 1)?;
        self.user.gold -= decomposition.need_gold;

        let roll: i32 = rand::random::<i32>() % 100;
        if roll < decomposition.success_rate {
            self.add_item(&decomposition.get_goods, 1)?;
            self.message_queue.push(GameMessage {
                msg_type: MessageType::Decomposition,
                content: format!("分解成功！获得 {}", decomposition.get_goods),
                speaker: None,
            });
        } else {
            self.message_queue.push(GameMessage {
                msg_type: MessageType::Decomposition,
                content: "分解失败...".to_string(),
                speaker: None,
            });
        }

        Ok(())
    }

    // ==================== 食物系统 ====================

    pub fn eat_food(&mut self, food_name: &str) -> Result<(), String> {
        let food = self.foods.get(food_name)
            .ok_or("食物不存在")?.clone();

        if self.get_item_count(food_name) < 1 {
            return Err("物品不足".to_string());
        }

        self.remove_item(food_name, 1)?;
        self.user.hunger = (self.user.hunger + food.value).min(self.user.max_hunger);

        self.message_queue.push(GameMessage {
            msg_type: MessageType::HungerChange,
            content: format!("吃饱了！饥饿度 +{}", food.value),
            speaker: None,
        });

        Ok(())
    }

    // ==================== 技能系统 ====================

    pub fn learn_skill(&mut self, skill_name: &str) -> Result<(), String> {
        if !self.skills.contains_key(skill_name) {
            return Err("技能不存在".to_string());
        }

        if self.user.skills.contains(&skill_name.to_string()) {
            return Err("已学会该技能".to_string());
        }

        self.user.skills.push(skill_name.to_string());

        self.message_queue.push(GameMessage {
            msg_type: MessageType::SkillUse,
            content: format!("学会了技能: {}", skill_name),
            speaker: None,
        });

        Ok(())
    }

    pub fn use_skill(&mut self, skill_name: &str) -> Result<(), String> {
        let skill = self.skills.get(skill_name)
            .ok_or("技能不存在")?.clone();

        if !self.user.skills.contains(&skill_name.to_string()) {
            return Err("未学会该技能".to_string());
        }

        if self.user.mp < skill.mp {
            return Err("MP不足".to_string());
        }

        self.user.mp -= skill.mp;

        self.message_queue.push(GameMessage {
            msg_type: MessageType::SkillUse,
            content: format!("使用了技能: {}", skill_name),
            speaker: None,
        });

        Ok(())
    }

    // ==================== NPC系统 ====================

    pub fn talk_to_npc(&mut self, npc_name: &str) -> Result<(), String> {
        let npc = self.npcs.get(npc_name)
            .ok_or("NPC不存在")?.clone();

        self.message_queue.push(GameMessage {
            msg_type: MessageType::Dialog,
            content: npc.dialog,
            speaker: Some(npc.name),
        });

        Ok(())
    }

    // ==================== 帮助系统 ====================

    pub fn get_help(&self, topic: &str) -> Option<String> {
        self.helps.get(topic).cloned()
    }

    // ==================== 变量/标记系统 ====================

    pub fn set_var(&mut self, name: &str, value: &str) {
        self.user.variables.insert(name.to_string(), value.to_string());
    }

    pub fn get_var(&self, name: &str) -> String {
        self.user.variables.get(name).cloned().unwrap_or_default()
    }

    pub fn set_flag(&mut self, name: &str) {
        if !self.user.flags.contains(&name.to_string()) {
            self.user.flags.push(name.to_string());
        }
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.user.flags.contains(&name.to_string())
    }

    // ==================== 存档系统 ====================

    pub fn save_state(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.user)
            .map_err(|e| format!("序列化失败: {}", e))
    }

    pub fn load_state(&mut self, json: &str) -> Result<(), String> {
        let user: UserProfile = serde_json::from_str(json)
            .map_err(|e| format!("反序列化失败: {}", e))?;
        self.user = user;
        Ok(())
    }

    // ==================== 查询接口 ====================

    pub fn poll_message(&mut self) -> Option<GameMessage> {
        self.message_queue.pop()
    }

    pub fn get_user_info(&self) -> String {
        format!(
            "{} Lv.{} {} | HP: {}/{} MP: {}/{} | AD: {} AP: {} DEF: {} | Gold: {} Exp: {}",
            self.user.name, self.user.lv, self.user.occupation,
            self.user.hp, self.user.max_hp, self.user.mp, self.user.max_mp,
            self.user.ad, self.user.ap, self.user.defense,
            self.user.gold, self.user.exp
        )
    }

    pub fn get_position(&self) -> String {
        self.user.position.clone()
    }

    pub fn get_inventory_list(&self) -> Vec<(String, String, i32)> {
        self.user.knapsack.iter()
            .map(|(id, count)| {
                let name = self.items.get(id).map(|i| i.name.clone()).unwrap_or_default();
                (id.clone(), name, *count)
            })
            .collect()
    }

    pub fn get_skills_list(&self) -> Vec<String> {
        self.user.skills.clone()
    }

    pub fn get_tasks_list(&self) -> Vec<String> {
        self.user.tasks.iter().map(|t| t.task_id.clone()).collect()
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            occupation: String::new(),
            lv: 1,
            exp: 0,
            gold: 0,
            diamonds: 0,
            hp: 100,
            max_hp: 100,
            mp: 50,
            max_mp: 50,
            ad: 10,
            ap: 10,
            defense: 5,
            magic: 5,
            hit: 100,
            dodge: 0,
            crit: 0,
            position: "新手村".to_string(),
            equip: HashMap::new(),
            knapsack: HashMap::new(),
            tasks: Vec::new(),
            flags: Vec::new(),
            variables: HashMap::new(),
            skills: Vec::new(),
            hunger: 100,
            max_hunger: 100,
        }
    }
}
