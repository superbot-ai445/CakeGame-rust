/// CakeGame 数据库操作层
/// 直接读取 gamedata.sdb (SQLite)
use crate::core::*;
use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// 打开数据库
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|e| format!("打开数据库失败: {}", e))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// 获取数据库连接锁（自动恢复 poisoned mutex）
    pub fn lock_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
    // ==================== 用户数据操作 ====================

    /// 用户是否存在
    pub fn user_exists(&self, user_id: &str) -> bool {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM Basic_User WHERE ID=?1 AND Node=?2")
            .unwrap();
        let count: i32 = stmt
            .query_row(params![user_id, NODE_BASIC], |row| row.get(0))
            .unwrap_or(0);
        count > 0
    }

    /// 获取所有已注册用户ID
    pub fn all_users(&self) -> Vec<String> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT DISTINCT ID FROM Basic_User WHERE Node=?1")
            .unwrap();
        stmt.query_map(params![NODE_BASIC], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    /// 读取用户基础信息 (Node=基础信息)
    pub fn read_basic(&self, user_id: &str, item: &str) -> String {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Data FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3")
            .unwrap();
        stmt.query_row(params![user_id, NODE_BASIC, item], |row| row.get(0))
            .unwrap_or_default()
    }

    /// 写入用户基础信息
    pub fn write_basic(&self, user_id: &str, item: &str, data: &str) -> bool {
        let conn = self.lock_conn();
        // 先尝试更新
        let updated = conn
            .execute(
                "UPDATE Basic_User SET Data=?1 WHERE ID=?2 AND Node=?3 AND Item=?4",
                params![data, user_id, NODE_BASIC, item],
            )
            .unwrap_or(0);
        if updated == 0 {
            // 不存在则插入
            conn.execute(
                "INSERT INTO Basic_User (ID, Node, Item, Data) VALUES (?1, ?2, ?3, ?4)",
                params![user_id, NODE_BASIC, item, data],
            )
            .is_ok()
        } else {
            true
        }
    }

    /// 写入用户基础信息（整数）
    pub fn write_basic_int(&self, user_id: &str, item: &str, value: i32) -> bool {
        self.write_basic(user_id, item, &value.to_string())
    }

    /// 读取用户货币
    pub fn read_currency(&self, user_id: &str, currency: &str) -> i64 {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Data FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3")
            .unwrap();
        let data: String = stmt
            .query_row(params![user_id, NODE_CURRENCY, currency], |row| row.get(0))
            .unwrap_or_default();
        data.parse().unwrap_or(0)
    }

    /// 写入用户货币
    pub fn write_currency(&self, user_id: &str, currency: &str, value: i64) -> bool {
        let conn = self.lock_conn();
        let updated = conn
            .execute(
                "UPDATE Basic_User SET Data=?1 WHERE ID=?2 AND Node=?3 AND Item=?4",
                params![value.to_string(), user_id, NODE_CURRENCY, currency],
            )
            .unwrap_or(0);
        if updated == 0 {
            conn.execute(
                "INSERT INTO Basic_User (ID, Node, Item, Data) VALUES (?1, ?2, ?3, ?4)",
                params![user_id, NODE_CURRENCY, currency, value.to_string()],
            )
            .is_ok()
        } else {
            true
        }
    }

    /// 修改货币（增加/减少/设置）
    pub fn modify_currency(&self, user_id: &str, currency: &str, op: &str, amount: i64) -> i64 {
        let current = self.read_currency(user_id, currency);
        let new_value = match op {
            OP_ADD => current + amount,
            OP_SUB => (current - amount).max(0),
            OP_SET => amount,
            _ => current,
        };
        self.write_currency(user_id, currency, new_value);
        new_value
    }

    /// 读取用户自定义数据 (Node=用户数据)
    pub fn read_user_data(&self, user_id: &str, key: &str) -> String {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Data FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3")
            .unwrap();
        stmt.query_row(params![user_id, NODE_USER_DATA, key], |row| row.get(0))
            .unwrap_or_default()
    }

    /// 写入用户自定义数据
    pub fn write_user_data(&self, user_id: &str, key: &str, value: &str) -> bool {
        let conn = self.lock_conn();
        let updated = conn
            .execute(
                "UPDATE Basic_User SET Data=?1 WHERE ID=?2 AND Node=?3 AND Item=?4",
                params![value, user_id, NODE_USER_DATA, key],
            )
            .unwrap_or(0);
        if updated == 0 {
            conn.execute(
                "INSERT INTO Basic_User (ID, Node, Item, Data) VALUES (?1, ?2, ?3, ?4)",
                params![user_id, NODE_USER_DATA, key, value],
            )
            .is_ok()
        } else {
            true
        }
    }

    /// 删除用户数据
    #[allow(dead_code)]
    pub fn delete_user_data(&self, user_id: &str, key: &str) {
        let conn = self.lock_conn();
        let _ = conn.execute(
            "DELETE FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3",
            params![user_id, NODE_USER_DATA, key],
        );
    }

    // ==================== 背包操作 ====================

    /// 获取物品数量
    pub fn knapsack_quantity(&self, user_id: &str, item_name: &str) -> i32 {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Quantity FROM Basic_knapsack WHERE ID=?1 AND Name=?2")
            .unwrap();
        let qty: String = stmt
            .query_row(params![user_id, item_name], |row| row.get(0))
            .unwrap_or_default();
        qty.parse().unwrap_or(0)
    }

    /// 添加物品
    pub fn knapsack_add(&self, user_id: &str, item_name: &str, quantity: i32) -> bool {
        let current = self.knapsack_quantity(user_id, item_name);
        let conn = self.lock_conn();
        if current > 0 {
            conn.execute(
                "UPDATE Basic_knapsack SET Quantity=?1 WHERE ID=?2 AND Name=?3",
                params![(current + quantity).to_string(), user_id, item_name],
            )
            .is_ok()
        } else {
            conn.execute(
                "INSERT INTO Basic_knapsack (ID, Name, Quantity) VALUES (?1, ?2, ?3)",
                params![user_id, item_name, quantity.to_string()],
            )
            .is_ok()
        }
    }

    /// 获取用户背包物品列表
    pub fn get_knapsack_items(&self, user_id: &str) -> Vec<KnapsackItem> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Name, Quantity FROM Basic_knapsack WHERE ID=?1")
            .unwrap();
        let rows = stmt
            .query_map(params![user_id], |row| {
                let qty_str: String = row.get(1)?;
                Ok(KnapsackItem {
                    name: row.get(0)?,
                    quantity: qty_str.parse().unwrap_or(0),
                })
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// 删除物品
    pub fn knapsack_remove(&self, user_id: &str, item_name: &str, quantity: i32) -> bool {
        let current = self.knapsack_quantity(user_id, item_name);
        if current < quantity {
            return false;
        }
        let conn = self.lock_conn();
        if current == quantity {
            conn.execute(
                "DELETE FROM Basic_knapsack WHERE ID=?1 AND Name=?2",
                params![user_id, item_name],
            )
            .is_ok()
        } else {
            conn.execute(
                "UPDATE Basic_knapsack SET Quantity=?1 WHERE ID=?2 AND Name=?3",
                params![(current - quantity).to_string(), user_id, item_name],
            )
            .is_ok()
        }
    }

    /// 获取全部背包物品
    pub fn knapsack_all(&self, user_id: &str) -> Vec<KnapsackItem> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Name, Quantity FROM Basic_knapsack WHERE ID=?1")
            .unwrap();
        let rows = stmt
            .query_map(params![user_id], |row| {
                let name: String = row.get(0)?;
                let qty_str: String = row.get(1)?;
                Ok(KnapsackItem {
                    name,
                    quantity: qty_str.parse().unwrap_or(0),
                })
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    // ==================== 装备操作 ====================

    /// 读取装备槽位
    pub fn equip_read(&self, user_id: &str, slot: &str) -> Option<EquipInfo> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT EquipName, Add_HP, Add_MP, Add_Defense, Add_Magic, Add_AD, Add_AP, \
                 Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, Add_ADPTV, Add_ADPTR, \
                 Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value \
                 FROM Equip_Register WHERE User=?1 AND SlotName=?2",
            )
            .unwrap();
        stmt.query_row(params![user_id, slot], |row| {
            Ok(EquipInfo {
                slot: slot.to_string(),
                name: row.get(0)?,
                add_hp: row.get::<_, String>(1)?.parse().unwrap_or(0),
                add_mp: row.get::<_, String>(2)?.parse().unwrap_or(0),
                add_defense: row.get::<_, String>(3)?.parse().unwrap_or(0),
                add_magic: row.get::<_, String>(4)?.parse().unwrap_or(0),
                add_ad: row.get::<_, String>(5)?.parse().unwrap_or(0),
                add_ap: row.get::<_, String>(6)?.parse().unwrap_or(0),
                add_hit: row.get::<_, String>(7)?.parse().unwrap_or(0),
                add_dodge: row.get::<_, String>(8)?.parse().unwrap_or(0),
                add_crit: row.get::<_, String>(9)?.parse().unwrap_or(0),
                add_absorb_hp: row.get::<_, String>(10)?.parse().unwrap_or(0),
                add_adptv: row.get::<_, String>(11)?.parse().unwrap_or(0),
                add_adptr: row.get::<_, String>(12)?.parse().unwrap_or(0),
                add_apptr: row.get::<_, String>(13)?.parse().unwrap_or(0),
                add_apptv: row.get::<_, String>(14)?.parse().unwrap_or(0),
                add_immune_damage: row.get::<_, String>(15)?.parse().unwrap_or(0),
                special_type: row.get(16)?,
                special_value: row.get::<_, String>(17)?.parse().unwrap_or(0),
            })
        })
        .ok()
    }

    /// 读取装备槽位名称（用于攻击提示显示武器名）
    pub fn read_equip_name(&self, user_id: &str, slot: &str) -> String {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT EquipName FROM Equip_Register WHERE User=?1 AND SlotName=?2")
            .unwrap();
        stmt.query_row(params![user_id, slot], |row| {
            let name: String = row.get(0)?;
            Ok(crate::encoding::smart_decode(&name))
        })
        .unwrap_or_else(|_| "拳头".to_string())
    }

    /// 装备物品到槽位
    pub fn equip_set(&self, user_id: &str, slot: &str, item: &ItemDef) -> bool {
        let conn = self.lock_conn();
        // 先删除旧装备
        let _ = conn.execute(
            "DELETE FROM Equip_Register WHERE User=?1 AND SlotName=?2",
            params![user_id, slot],
        );
        // 插入新装备
        conn.execute(
            "INSERT INTO Equip_Register (User, SlotName, EquipName, Add_HP, Add_MP, Add_Defense, \
             Add_Magic, Add_AD, Add_AP, Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, \
             Add_ADPTV, Add_ADPTR, Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                user_id,
                slot,
                item.name,
                item.data.add_hp.to_string(),
                item.data.add_mp.to_string(),
                item.data.add_defense.to_string(),
                item.data.add_magic.to_string(),
                item.data.add_ad.to_string(),
                item.data.add_ap.to_string(),
                item.data.add_hit.to_string(),
                item.data.add_dodge.to_string(),
                item.data.add_crit.to_string(),
                item.data.add_absorb_hp.to_string(),
                item.data.add_adptv.to_string(),
                item.data.add_adptr.to_string(),
                item.data.add_apptr.to_string(),
                item.data.add_apptv.to_string(),
                item.data.add_immune_damage.to_string(),
                item.data.special_type,
                item.data.special_value.to_string(),
            ],
        )
        .is_ok()
    }

    /// 卸下装备
    pub fn equip_remove(&self, user_id: &str, slot: &str) -> Option<String> {
        let info = self.equip_read(user_id, slot)?;
        let conn = self.lock_conn();
        let _ = conn.execute(
            "DELETE FROM Equip_Register WHERE User=?1 AND SlotName=?2",
            params![user_id, slot],
        );
        Some(info.name)
    }

    /// 获取全部装备
    pub fn equip_all(&self, user_id: &str) -> Vec<EquipInfo> {
        let slots = vec![
            SLOT_WEAPON,
            SLOT_HELMET,
            SLOT_ARMOR,
            SLOT_LEG,
            SLOT_BOOTS,
            SLOT_NECKLACE,
            SLOT_RING,
            SLOT_WING,
            SLOT_FASHION,
            SLOT_TITLE,
        ];
        slots.iter().filter_map(|s| self.equip_read(user_id, s)).collect()
    }

    // ==================== 技能操作 ====================

    /// 用户是否已学习技能
    pub fn skill_has(&self, user_id: &str, skill_name: &str) -> bool {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM Skill_Register WHERE User=?1 AND SkillName=?2")
            .unwrap();
        let count: i32 = stmt
            .query_row(params![user_id, skill_name], |row| row.get(0))
            .unwrap_or(0);
        count > 0
    }

    /// 学习技能
    pub fn skill_learn(&self, user_id: &str, skill_name: &str, proficiency: i32, occupation: &str) -> bool {
        if self.skill_has(user_id, skill_name) {
            return false;
        }
        let conn = self.lock_conn();
        conn.execute(
            "INSERT INTO Skill_Register (User, SkillName, Proficiency, Occupation) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, skill_name, proficiency.to_string(), occupation],
        )
        .is_ok()
    }

    /// 获取用户全部技能
    pub fn skill_all(&self, user_id: &str) -> Vec<(String, i32)> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT SkillName, Proficiency FROM Skill_Register WHERE User=?1")
            .unwrap();
        let rows = stmt
            .query_map(params![user_id], |row| {
                let name: String = row.get(0)?;
                let prof: String = row.get(1)?;
                Ok((name, prof.parse().unwrap_or(0)))
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    // ==================== 配置读取 ====================

    /// 读取全局数据 (Global 表)
    pub fn global_get(&self, section: &str, id: &str) -> String {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT DATA FROM Global WHERE SECTION=?1 AND ID=?2")
            .unwrap();
        stmt.query_row(params![section, id], |row| row.get(0))
            .unwrap_or_default()
    }

    /// 写入全局数据
    pub fn global_set(&self, section: &str, id: &str, data: &str) -> bool {
        let conn = self.lock_conn();
        let updated = conn
            .execute(
                "UPDATE Global SET DATA=?1 WHERE SECTION=?2 AND ID=?3",
                params![data, section, id],
            )
            .unwrap_or(0);
        if updated == 0 {
            conn.execute(
                "INSERT INTO Global (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
                params![id, section, data],
            )
            .is_ok()
        } else {
            true
        }
    }

    /// 读取消息模板 (MessageTemplate 表)
    pub fn template_get(&self, name: &str) -> String {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT Data FROM MessageTemplate WHERE Name=?1").unwrap();
        let raw: String = stmt.query_row(params![name], |row| row.get(0)).unwrap_or_default();
        raw
    }

    /// 读取物品定义 (Config_Goods 表)
    pub fn item_get(&self, name: &str) -> Option<ItemDef> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ID, Name, Type, Basic, Lock, LtemData, Introduce FROM Config_Goods WHERE Name=?1")
            .unwrap();
        stmt.query_row(params![name], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let item_type: String = row.get(2)?;
            let basic_str: String = row.get(3)?;
            let lock_str: String = row.get(4)?;
            let data_str: String = row.get(5)?;
            let introduce: String = row.get(6)?;
            let data: ItemData = serde_json::from_str(&data_str).unwrap_or_default();
            Ok(ItemDef {
                id,
                name,
                item_type,
                basic: basic_str == "TRUE",
                locked: lock_str == "TRUE",
                introduce,
                data,
            })
        })
        .ok()
    }

    /// 读取怪物定义 (Config_Monster 表)
    pub fn monster_get(&self, name: &str) -> Option<MonsterDef> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT Monster_Name, Monster_Type, Monster_AD, Monster_AP, Monster_HP, \
             Monster_Defense, Monster_AbsorbHP, Monster_ADPTV, Monster_ADPTR, \
             Monster_APPTR, Monster_APPTV, Monster_ImmuneDamage, Skills, \
             Reward_Goods, Reward_Exp, Reward_Gold, Introduce, AttackEffect, \
             AttackTips, MagicResistance, Hit, Dodge, IgnoreShield \
             FROM Config_Monster WHERE Monster_Name=?1",
            )
            .unwrap();
        stmt.query_row(params![name], |row| {
            let skills_str: String = row.get(12)?;
            let rewards_str: String = row.get(13)?;
            let ignore_str: String = row.get(22)?;
            Ok(MonsterDef {
                name: row.get(0)?,
                monster_type: row.get(1)?,
                ad: row.get::<_, String>(2)?.parse().unwrap_or(0),
                ap: row.get::<_, String>(3)?.parse().unwrap_or(0),
                hp: row.get::<_, String>(4)?.parse().unwrap_or(0),
                defense: row.get::<_, String>(5)?.parse().unwrap_or(0),
                absorb_hp: row.get::<_, String>(6)?.parse().unwrap_or(0),
                adptv: row.get::<_, String>(7)?.parse().unwrap_or(0),
                adptr: row.get::<_, String>(8)?.parse().unwrap_or(0),
                apptr: row.get::<_, String>(9)?.parse().unwrap_or(0),
                apptv: row.get::<_, String>(10)?.parse().unwrap_or(0),
                immune_damage: row.get::<_, String>(11)?.parse().unwrap_or(0),
                skills: skills_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                reward_goods: parse_reward_goods(&rewards_str),
                reward_exp: row.get::<_, String>(14)?.parse().unwrap_or(0),
                reward_gold: row.get::<_, String>(15)?.parse().unwrap_or(0),
                introduce: row.get(16)?,
                attack_effect: row.get(17)?,
                attack_tips: row.get(18)?,
                magic_resistance: row.get::<_, String>(19)?.parse().unwrap_or(0),
                hit: row.get::<_, String>(20)?.parse().unwrap_or(0),
                dodge: row.get::<_, String>(21)?.parse().unwrap_or(0),
                ignore_shield: ignore_str == "TRUE",
            })
        })
        .ok()
    }

    /// 读取地图定义 (Config_Map 表)
    pub fn map_get(&self, name: &str) -> Option<MapDef> {
        let conn = self.lock_conn();
        // 地图名在 DB 中可能是 hex GBK 编码，需要遍历解码匹配
        let mut stmt = conn
            .prepare("SELECT Name, LV, Introduce, Security, Monster, UP, Down, Left, Right, Consume FROM Config_Map")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                let raw_name: String = row.get(0)?;
                let monster_str: String = row.get(4)?;
                Ok((
                    raw_name.clone(),
                    MapDef {
                        name: crate::encoding::smart_decode(&raw_name),
                        level: row.get::<_, String>(1)?.parse().unwrap_or(0),
                        introduce: crate::encoding::smart_decode(&row.get::<_, String>(2)?),
                        security: row.get::<_, String>(3)? == "TRUE",
                        monsters: {
                            // Monster format: [name]\nconfig...
                            let decoded = crate::encoding::smart_decode(&monster_str);
                            let mut monsters = Vec::new();
                            for line in decoded.lines() {
                                let line = line.trim();
                                if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
                                    monsters.push(line[1..line.len() - 1].to_string());
                                }
                            }
                            monsters
                        },
                        up: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(5)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        down: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(6)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        left: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(7)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        right: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(8)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        consume: row.get(9)?,
                    },
                ))
            })
            .unwrap();
        for (raw, map) in rows.flatten() {
            // 匹配原始名或解码后的名
            if raw == name || map.name == name {
                return Some(map);
            }
        }
        None
    }

    /// 读取所有地图定义
    pub fn map_get_all(&self) -> Vec<(String, MapDef)> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Name, LV, Introduce, Security, Monster, UP, Down, Left, Right, Consume FROM Config_Map")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                let raw_name: String = row.get(0)?;
                let monster_str: String = row.get(4)?;
                Ok((
                    crate::encoding::smart_decode(&raw_name),
                    MapDef {
                        name: crate::encoding::smart_decode(&raw_name),
                        level: row.get::<_, String>(1)?.parse().unwrap_or(0),
                        introduce: crate::encoding::smart_decode(&row.get::<_, String>(2)?),
                        security: row.get::<_, String>(3)? == "TRUE",
                        monsters: {
                            let decoded = crate::encoding::smart_decode(&monster_str);
                            let mut monsters = Vec::new();
                            for line in decoded.lines() {
                                let line = line.trim();
                                if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
                                    monsters.push(line[1..line.len() - 1].to_string());
                                }
                            }
                            monsters
                        },
                        up: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(5)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        down: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(6)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        left: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(7)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        right: {
                            let v = crate::encoding::smart_decode(&row.get::<_, String>(8)?);
                            if v == "[NULL]" || v.is_empty() {
                                String::new()
                            } else {
                                v
                            }
                        },
                        consume: row.get(9)?,
                    },
                ))
            })
            .unwrap();
        rows.flatten().collect()
    }

    /// 读取技能定义 (Config_Skills 表)
    pub fn skill_get(&self, name: &str) -> Option<SkillDef> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT Name, Type, Consume, Effect, EO, LV, ConsumeType, Cooling, Accurate, \
                 AttackTips, Introduce, ACS, Shield, IgnoreShield, IgnoreIM, IgnoreRE, \
                 BanAbsorb, BanMultipleShot, ProhibitUO, ConsumableGoods, \
                 Continued_Round, Continued_Type, Continued_Effect \
                 FROM Config_Skills WHERE Name=?1",
            )
            .unwrap();
        stmt.query_row(params![name], |row| {
            Ok(SkillDef {
                name: row.get(0)?,
                skill_type: row.get(1)?,
                consume: row.get::<_, String>(2)?.parse().unwrap_or(0),
                effect: row.get::<_, String>(3)?.parse().unwrap_or(0),
                effect_other: row.get(4)?,
                level: row.get::<_, String>(5)?.parse().unwrap_or(0),
                consume_type: row.get(6)?,
                cooling: row.get::<_, String>(7)?.parse().unwrap_or(0),
                accurate: row.get::<_, String>(8)?.parse().unwrap_or(100),
                attack_tips: row.get(9)?,
                introduce: row.get(10)?,
                acs: row.get(11)?,
                shield: row.get::<_, String>(12)?.parse().unwrap_or(0),
                ignore_shield: row.get::<_, String>(13)? == "TRUE",
                ignore_immune: row.get::<_, String>(14)? == "TRUE",
                ignore_re: row.get::<_, String>(15)? == "TRUE",
                ban_absorb: row.get::<_, String>(16)? == "TRUE",
                ban_multiple_shot: row.get::<_, String>(17)? == "TRUE",
                prohibit_uo: row.get::<_, String>(18)? == "TRUE",
                consumable_goods: row.get(19)?,
                continued_round: row.get::<_, String>(20)?.parse().unwrap_or(0),
                continued_type: row.get(21)?,
                continued_effect: row.get(22)?,
            })
        })
        .ok()
    }

    /// 读取职业定义 (Config_Occupation 表)
    pub fn occupation_get(&self, name: &str) -> Option<OccupationDef> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT Name, Basics, HP, MP, AD, AP, Defense, Hit, Dodge, Crit, AbsorbHP, \
                 ADPTV, ADPTR, APPTR, APPTV, ImmuneDamage, Intro, ExclusiveSkills, \
                 TransferDemand, TransferLevel, FormerOccupation, Belong, AttackEffect, \
                 AttackTips, MagicResistance, IgnoreShield \
                 FROM Config_Occupation",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                let raw_name: String = row.get(0)?;
                let skills_str: String = row.get(17)?;
                Ok((
                    raw_name.clone(),
                    OccupationDef {
                        name: crate::encoding::smart_decode(&raw_name),
                        basics: row.get(1)?,
                        hp: crate::encoding::smart_decode(&row.get::<_, String>(2)?)
                            .parse()
                            .unwrap_or(0),
                        mp: crate::encoding::smart_decode(&row.get::<_, String>(3)?)
                            .parse()
                            .unwrap_or(0),
                        ad: crate::encoding::smart_decode(&row.get::<_, String>(4)?)
                            .parse()
                            .unwrap_or(0),
                        ap: crate::encoding::smart_decode(&row.get::<_, String>(5)?)
                            .parse()
                            .unwrap_or(0),
                        defense: crate::encoding::smart_decode(&row.get::<_, String>(6)?)
                            .parse()
                            .unwrap_or(0),
                        hit: crate::encoding::smart_decode(&row.get::<_, String>(7)?)
                            .parse()
                            .unwrap_or(0),
                        dodge: crate::encoding::smart_decode(&row.get::<_, String>(8)?)
                            .parse()
                            .unwrap_or(0),
                        crit: crate::encoding::smart_decode(&row.get::<_, String>(9)?)
                            .parse()
                            .unwrap_or(0),
                        absorb_hp: crate::encoding::smart_decode(&row.get::<_, String>(10)?)
                            .parse()
                            .unwrap_or(0),
                        adptv: crate::encoding::smart_decode(&row.get::<_, String>(11)?)
                            .parse()
                            .unwrap_or(0),
                        adptr: crate::encoding::smart_decode(&row.get::<_, String>(12)?)
                            .parse()
                            .unwrap_or(0),
                        apptr: crate::encoding::smart_decode(&row.get::<_, String>(13)?)
                            .parse()
                            .unwrap_or(0),
                        apptv: crate::encoding::smart_decode(&row.get::<_, String>(14)?)
                            .parse()
                            .unwrap_or(0),
                        immune_damage: crate::encoding::smart_decode(&row.get::<_, String>(15)?)
                            .parse()
                            .unwrap_or(0),
                        intro: crate::encoding::smart_decode(&row.get::<_, String>(16)?),
                        exclusive_skills: crate::encoding::smart_decode(&skills_str)
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                        transfer_demand: crate::encoding::smart_decode(&row.get::<_, String>(18)?),
                        transfer_level: crate::encoding::smart_decode(&row.get::<_, String>(19)?)
                            .parse()
                            .unwrap_or(0),
                        former_occupation: crate::encoding::smart_decode(&row.get::<_, String>(20)?),
                        belong: crate::encoding::smart_decode(&row.get::<_, String>(21)?),
                        attack_effect: crate::encoding::smart_decode(&row.get::<_, String>(22)?),
                        attack_tips: crate::encoding::smart_decode(&row.get::<_, String>(23)?),
                        magic_resistance: crate::encoding::smart_decode(&row.get::<_, String>(24)?)
                            .parse()
                            .unwrap_or(0),
                        ignore_shield: row.get::<_, String>(25)? == "TRUE",
                    },
                ))
            })
            .unwrap();
        for (raw, occ) in rows.flatten() {
            if raw == name || occ.name == name {
                return Some(occ);
            }
        }
        None
    }

    /// 获取所有职业名称列表
    pub fn occupation_list(&self) -> Vec<String> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT Name FROM Config_Occupation").unwrap();
        let rows = stmt
            .query_map([], |row| {
                let raw: String = row.get(0)?;
                Ok(crate::encoding::smart_decode(&raw))
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// 获取排行榜数据
    pub fn rank_by_attribute(&self, node: &str, item: &str, limit: i32) -> Vec<RankEntry> {
        let conn = self.lock_conn();
        let sql = "SELECT ID, Data FROM Basic_User WHERE Node=?1 AND Item=?2 ORDER BY CAST(Data AS REAL) DESC LIMIT ?3";
        let mut stmt = conn.prepare(sql).unwrap();
        let rows = stmt
            .query_map(params![node, item, limit], |row| {
                Ok(RankEntry {
                    rank: 0,
                    user_id: row.get(0)?,
                    name: String::new(),
                    value: row.get(1)?,
                })
            })
            .unwrap();
        let mut result: Vec<RankEntry> = rows.filter_map(|r| r.ok()).collect();
        for (i, entry) in result.iter_mut().enumerate() {
            entry.rank = (i + 1) as i32;
        }
        result
    }

    // ==================== 任务系统操作 ====================

    /// 读取任务定义 (Config_Task 表)
    pub fn task_get(&self, title: &str) -> Option<TaskDef> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT Title, LV, Type, ResetTime, ResetType, CompleteTask, Occupation, \
                 Target, Data, Reward_Gold, Reward_Diamonds, Reward_EXP, Reward_Goods, Info \
                 FROM Config_Task WHERE Title=?1",
            )
            .unwrap();
        stmt.query_row(params![title], |row| {
            let lv_str: String = row.get(1)?;
            let lv_parts: Vec<&str> = lv_str.split('-').collect();
            let level_min: i32 = lv_parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            let level_max: i32 = lv_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

            let reset_time: i32 = row.get::<_, String>(3)?.parse().unwrap_or(-1);
            let reset_type: i32 = row.get::<_, String>(4)?.parse().unwrap_or(0);
            let reward_gold: i64 = row.get::<_, String>(9)?.parse().unwrap_or(0);
            let reward_diamond: i64 = row.get::<_, String>(10)?.parse().unwrap_or(0);
            let reward_exp: i64 = row.get::<_, String>(11)?.parse().unwrap_or(0);

            let reward_goods_str: String = row.get(12)?;
            let reward_goods = parse_reward_goods(&reward_goods_str);

            Ok(TaskDef {
                title: row.get(0)?,
                level_min,
                level_max,
                task_type: row.get(2)?,
                reset_time,
                reset_type,
                complete_task: row.get(5)?,
                occupation: row.get(6)?,
                target_type: row.get(7)?,
                data: row.get(8)?,
                reward_gold,
                reward_diamond,
                reward_exp,
                reward_goods,
                info: row.get(13)?,
            })
        })
        .ok()
    }

    /// 任务是否存在
    pub fn task_exists(&self, title: &str) -> bool {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM Config_Task WHERE Title=?1").unwrap();
        let count: i32 = stmt.query_row(params![title], |row| row.get(0)).unwrap_or(0);
        count > 0
    }

    /// 获取用户可接任务列表 (考虑等级、职业、已完成任务)
    pub fn task_available(&self, user_id: &str) -> Vec<String> {
        let occupation = self.read_basic(user_id, ITEM_OCCUPATION);
        let _level: i32 = self.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);

        let conn = self.lock_conn();

        // 先获取用户已完成/进行中的任务
        let mut completed_tasks: Vec<String> = Vec::new();
        {
            let mut stmt = conn
                .prepare("SELECT TaskName FROM Task_Register WHERE User=?1 AND Complete=?2")
                .unwrap();
            let rows = stmt.query_map(params![user_id, "TRUE"], |row| row.get(0)).unwrap();
            for name in rows.flatten() {
                completed_tasks.push(name);
            }
        }

        // 获取所有任务
        let mut stmt = conn
            .prepare("SELECT Title, Occupation, CompleteTask FROM Config_Task")
            .unwrap();
        let mut available: Vec<String> = Vec::new();
        let rows = stmt
            .query_map([], |row| {
                let title: String = row.get(0)?;
                let occ: String = row.get(1)?;
                let complete_task: String = row.get(2)?;
                Ok((title, occ, complete_task))
            })
            .unwrap();

        for (title, occ, complete_task) in rows.flatten() {
            // 检查职业限制
            if !occ.is_empty() && occ != "[NULL]" && occ != occupation {
                continue;
            }
            // 检查前置任务
            if !complete_task.is_empty() && complete_task != "[NULL]" && !completed_tasks.contains(&complete_task) {
                continue;
            }
            available.push(title);
        }
        available
    }

    /// 获取所有任务标题（不过滤）
    pub fn task_get_all_titles(&self) -> Vec<String> {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare("SELECT Title FROM Config_Task ORDER BY LV ASC").unwrap();
        let rows = stmt.query_map([], |row| row.get(0)).unwrap();
        rows.flatten().collect()
    }

    /// 获取用户进行中的任务记录
    pub fn task_get_record(&self, user_id: &str, task_name: &str) -> Option<TaskRecord> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT User, TaskName, Date, Target, Data, Complete FROM Task_Register WHERE User=?1 AND TaskName=?2",
            )
            .unwrap();
        stmt.query_row(params![user_id, task_name], |row| {
            Ok(TaskRecord {
                user: row.get(0)?,
                task_name: row.get(1)?,
                date: row.get(2)?,
                target: row.get(3)?,
                data: row.get(4)?,
                complete: row.get::<_, String>(5)?.as_str() == "TRUE",
            })
        })
        .ok()
    }

    /// 登记任务 (领取/更新)
    pub fn task_register(
        &self,
        user_id: &str,
        task_name: &str,
        date: &str,
        target: &str,
        data: &str,
        complete: bool,
    ) -> bool {
        let conn = self.lock_conn();
        let complete_str = if complete { "TRUE" } else { "FALSE" };

        // 检查是否已存在
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM Task_Register WHERE User=?1 AND TaskName=?2")
            .unwrap();
        let count: i32 = stmt
            .query_row(params![user_id, task_name], |row| row.get(0))
            .unwrap_or(0);

        if count > 0 {
            // 更新
            let mut updates = Vec::new();
            if !date.is_empty() {
                updates.push(format!("Date='{}'", date));
            }
            if !target.is_empty() {
                updates.push(format!("Target='{}'", target));
            }
            if !data.is_empty() {
                updates.push(format!("Data='{}'", data));
            }
            updates.push(format!("Complete='{}'", complete_str));

            let sql = format!(
                "UPDATE Task_Register SET {} WHERE User=?1 AND TaskName=?2",
                updates.join(", ")
            );
            conn.execute(&sql, params![user_id, task_name]).is_ok()
        } else {
            // 插入
            conn.execute(
                "INSERT INTO Task_Register (User, TaskName, Date, Target, Data, Complete) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![user_id, task_name, date, target, data, complete_str],
            )
            .is_ok()
        }
    }

    /// 删除任务记录
    pub fn task_delete_record(&self, user_id: &str, task_name: &str) -> bool {
        let conn = self.lock_conn();
        conn.execute(
            "DELETE FROM Task_Register WHERE User=?1 AND TaskName=?2",
            params![user_id, task_name],
        )
        .is_ok()
    }

    // ==================== 私人商店操作 ====================

    /// 获取私人商店列表（已开启的）
    pub fn private_shop_list(&self) -> Vec<PrivateShop> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ID, Name, GoodsData, Open FROM Config_PrivateShops WHERE Open=?1")
            .unwrap();
        let rows = stmt
            .query_map(params!["TRUE"], |row| {
                Ok(PrivateShop {
                    owner: row.get(0)?,
                    name: row.get(1)?,
                    goods_data: row.get(2)?,
                    open: true,
                })
            })
            .unwrap();
        rows.filter_map(|r| r.ok()).collect()
    }

    /// 获取用户私人商店
    pub fn private_shop_get(&self, user_id: &str) -> Option<PrivateShop> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ID, Name, GoodsData, Open FROM Config_PrivateShops WHERE ID=?1")
            .unwrap();
        stmt.query_row(params![user_id], |row| {
            let open_str: String = row.get(3)?;
            Ok(PrivateShop {
                owner: row.get(0)?,
                name: row.get(1)?,
                goods_data: row.get(2)?,
                open: open_str == "TRUE",
            })
        })
        .ok()
    }

    /// 保存私人商店
    pub fn private_shop_save(&self, shop: &PrivateShop) -> bool {
        let conn = self.lock_conn();
        let open_str = if shop.open { "TRUE" } else { "FALSE" };

        // 检查是否已存在
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM Config_PrivateShops WHERE ID=?1")
            .unwrap();
        let count: i32 = stmt.query_row(params![shop.owner], |row| row.get(0)).unwrap_or(0);

        if count > 0 {
            conn.execute(
                "UPDATE Config_PrivateShops SET Name=?1, GoodsData=?2, Open=?3 WHERE ID=?4",
                params![shop.name, shop.goods_data, open_str, shop.owner],
            )
            .is_ok()
        } else {
            conn.execute(
                "INSERT INTO Config_PrivateShops (ID, Name, GoodsData, Open) VALUES (?1, ?2, ?3, ?4)",
                params![shop.owner, shop.name, shop.goods_data, open_str],
            )
            .is_ok()
        }
    }

    // ==================== 通用查询方法 ====================

    /// 通用单行查询，返回闭包处理结果
    pub fn query_row<T, F>(&self, sql: &str, params: &[&str], f: F) -> Result<T, String>
    where
        F: FnOnce(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> = params
            .iter()
            .map(|s| Box::new(s.to_string()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(param_refs.as_slice(), f).map_err(|e| e.to_string())
    }

    /// 通用多行查询，返回 Vec<T>
    pub fn query_rows<T, F>(&self, sql: &str, params: &[&str], f: F) -> Vec<T>
    where
        F: FnMut(&rusqlite::Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let conn = self.lock_conn();
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> = params
            .iter()
            .map(|s| Box::new(s.to_string()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), f);
        match rows {
            Ok(r) => r.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        }
    }

    /// 通用单列查询，返回 Vec<String>
    pub fn query_list(&self, sql: &str, params: &[&str]) -> Vec<String> {
        self.query_rows(sql, params, |row| row.get::<_, String>(0))
    }

    /// 获取物品数量（别名）
    pub fn get_item_count(&self, user_id: &str, item_name: &str) -> i32 {
        self.knapsack_quantity(user_id, item_name)
    }

    /// 添加物品（别名）
    pub fn add_item(&self, user_id: &str, item_name: &str, count: i32) -> bool {
        self.knapsack_add(user_id, item_name, count)
    }

    /// 移除物品（别名）
    pub fn remove_item(&self, user_id: &str, item_name: &str, count: i32) -> bool {
        self.knapsack_remove(user_id, item_name, count)
    }

    /// 获取物品定义数据（类型, 名称, JSON数据）
    pub fn get_item_data(&self, name: &str) -> Option<(String, String, String)> {
        self.item_get(name).map(|item| {
            let data_json = serde_json::to_string(&item.data).unwrap_or_default();
            (item.item_type.clone(), item.name.clone(), data_json)
        })
    }

    // ==================== 护盾系统 ====================

    /// 查询用户的护盾记录
    /// 返回 (值, 获取时间, 持续天数, 模式)
    pub fn get_user_shield(&self, user_id: &str) -> Option<(i32, String, i32, String)> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Value, GetTime, Duration, Pattern FROM Shield_Register WHERE ID=?1 AND ID_Type='USER'")
            .ok()?;
        stmt.query_row(rusqlite::params![user_id], |row| {
            Ok((
                row.get::<_, String>(0)?.parse().unwrap_or(0),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2)?.parse().unwrap_or(0),
                row.get::<_, String>(3).unwrap_or_default(),
            ))
        })
        .ok()
    }

    /// 查询所有护盾记录数量
    pub fn get_shield_count(&self) -> i32 {
        let conn = self.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Shield_Register", [], |row| row.get(0))
            .unwrap_or(0)
    }

    /// 写入护盾记录
    #[allow(dead_code)]
    pub fn set_user_shield(&self, user_id: &str, value: i32, duration: i32, pattern: &str) {
        let now = chrono::Local::now().format("%Y年%m月%d日%H时%M分%S秒").to_string();
        let conn = self.lock_conn();
        // 先删除旧记录
        let _ = conn.execute(
            "DELETE FROM Shield_Register WHERE ID=?1 AND ID_Type='USER'",
            rusqlite::params![user_id],
        );
        // 插入新记录
        let _ = conn.execute(
            "INSERT INTO Shield_Register (ID, ID_Type, Value, GetTime, Duration, Pattern) VALUES (?1, 'USER', ?2, ?3, ?4, ?5)",
            rusqlite::params![user_id, value.to_string(), now, duration.to_string(), pattern],
        );
    }

    /// 扣除护盾值（被攻击时消耗）
    pub fn consume_shield(&self, user_id: &str, amount: i32) -> i32 {
        if let Some((value, _, _, _)) = self.get_user_shield(user_id) {
            let absorbed = value.min(amount);
            let remaining = value - absorbed;
            if remaining <= 0 {
                // 护盾耗尽，删除记录
                let conn = self.lock_conn();
                let _ = conn.execute(
                    "DELETE FROM Shield_Register WHERE ID=?1 AND ID_Type='USER'",
                    rusqlite::params![user_id],
                );
            } else {
                // 更新护盾值
                let conn = self.lock_conn();
                let _ = conn.execute(
                    "UPDATE Shield_Register SET Value=?1 WHERE ID=?2 AND ID_Type='USER'",
                    rusqlite::params![remaining.to_string(), user_id],
                );
            }
            absorbed
        } else {
            0
        }
    }

    // ==================== 烹饪系统操作 ====================

    /// 获取全部烹饪配方
    pub fn get_all_cooking_recipes(&self) -> Vec<CookingRecipe> {
        let conn = self.lock_conn();
        let mut stmt = match conn.prepare("SELECT Name, time, foodstuff FROM ext_cook_info") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([], |row| {
            Ok(CookingRecipe {
                name: row.get(0)?,
                time: row.get::<_, String>(1)?.parse().unwrap_or(0),
                foodstuff: row.get(2)?,
            })
        }) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// 获取单个烹饪配方
    pub fn get_cooking_recipe(&self, name: &str) -> Option<CookingRecipe> {
        let conn = self.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Name, time, foodstuff FROM ext_cook_info WHERE Name=?1")
            .ok()?;
        stmt.query_row(rusqlite::params![name], |row| {
            Ok(CookingRecipe {
                name: row.get(0)?,
                time: row.get::<_, String>(1)?.parse().unwrap_or(0),
                foodstuff: row.get(2)?,
            })
        })
        .ok()
    }

    /// 读取 system_uAttributes 表中的系统基础属性
    /// 返回 (HP, MP, AD, AP, Defense, Hit, Dodge, Crit, MagicResistance, AbsorbHP, ImmuneDamage, ADPTV, ADPTR, APPTV, APPTR)
    #[allow(clippy::type_complexity)]
    pub fn get_system_base_attrs(
        &self,
    ) -> (
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
    ) {
        let conn = self.lock_conn();
        let mut result = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let mut stmt = match conn.prepare("SELECT AttrName, aValue FROM system_uAttributes") {
            Ok(s) => s,
            Err(_) => return result,
        };
        if let Ok(rows) = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let val: i32 = row.get(1).unwrap_or(0);
            Ok((name, val))
        }) {
            for r in rows.flatten() {
                match r.0.as_str() {
                    "HP" => result.0 = r.1,
                    "MP" => result.1 = r.1,
                    "AD" => result.2 = r.1,
                    "AP" => result.3 = r.1,
                    "Defense" => result.4 = r.1,
                    "Hit" => result.5 = r.1,
                    "Dodge" => result.6 = r.1,
                    "Crit" => result.7 = r.1,
                    "MagicResistance" => result.8 = r.1,
                    "AbsorbHP" => result.9 = r.1,
                    "ImmuneDamage" => result.10 = r.1,
                    "ADPTV" => result.11 = r.1,
                    "ADPTR" => result.12 = r.1,
                    "APPTV" => result.13 = r.1,
                    "APPTR" => result.14 = r.1,
                    _ => {}
                }
            }
        }
        result
    }
}

/// 解析奖励物品字符串
fn parse_reward_goods(s: &str) -> Vec<RewardItem> {
    if s.is_empty() {
        return vec![];
    }
    s.split(',')
        .filter_map(|part| {
            let parts: Vec<&str> = part.trim().split('*').collect();
            if parts.len() >= 2 {
                Some(RewardItem {
                    name: parts[0].to_string(),
                    count: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1),
                    rate: parts[1].parse().unwrap_or(100.0),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reward_goods_empty() {
        assert!(parse_reward_goods("").is_empty());
    }

    #[test]
    fn test_parse_reward_goods_single() {
        let items = parse_reward_goods("强化石*10.0*5");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "强化石");
        assert!((items[0].rate - 10.0).abs() < 1e-10);
        assert_eq!(items[0].count, 5);
    }

    #[test]
    fn test_parse_reward_goods_single_no_count() {
        let items = parse_reward_goods("生命药水*50.0");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "生命药水");
        assert!((items[0].rate - 50.0).abs() < 1e-10);
        assert_eq!(items[0].count, 1);
    }

    #[test]
    fn test_parse_reward_goods_multiple() {
        let items = parse_reward_goods("强化石*10.0*5,生命药水*50.0*3");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "强化石");
        assert_eq!(items[1].name, "生命药水");
    }

    #[test]
    fn test_parse_reward_goods_invalid_entry() {
        let items = parse_reward_goods("invalid_entry");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_reward_goods_mixed() {
        let items = parse_reward_goods("强化石*10.0*5,invalid,生命药水*50.0");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_parse_reward_goods_rate_default() {
        let items = parse_reward_goods("物品*abc");
        assert_eq!(items.len(), 1);
        assert!((items[0].rate - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_reward_goods_with_spaces() {
        let items = parse_reward_goods(" 强化石 * 10.0 * 5 ");
        assert_eq!(items.len(), 1);
        // trim() removes leading/trailing spaces from the comma-separated segment
        // but split('*') preserves internal spaces
        assert_eq!(items[0].name, "强化石 ");
    }
}
