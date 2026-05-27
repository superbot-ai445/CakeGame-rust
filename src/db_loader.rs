/// CakeGame 数据库加载器 - 完整版
/// 解析原版 gamedata.sdb 数据库的所有表

use rusqlite::Connection;
use std::collections::HashMap;
use encoding::{DecoderTrap, Encoding};
use encoding::all::GBK;

use crate::core::*;

/// 解析十六进制编码的GBK字符串
pub fn decode_hex_gbk(hex_str: &str) -> String {
    if hex_str.is_empty() || hex_str == "[NULL]" {
        return String::new();
    }

    let bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .filter_map(|i| {
            hex_str.get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok())
        })
        .collect();

    if bytes.is_empty() {
        return hex_str.to_string();
    }

    GBK.decode(&bytes, DecoderTrap::Replace).unwrap_or_default()
}

/// 解析JSON格式的物品数据
pub fn parse_item_data(json_str: &str) -> ItemData {
    let decoded = decode_hex_gbk(json_str);
    let mut data = ItemData::default();

    if let Ok(map) = serde_json::from_str::<serde_json::Value>(&decoded) {
        data.slot_name = map["SlotName"].as_str().unwrap_or("").to_string();
        data.occupation = map["Occupation"].as_str().unwrap_or("").to_string();
        data.use_lv = map["UseLV"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_hp = map["Add_HP"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_mp = map["Add_MP"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_defense = map["Add_Defense"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_magic = map["Add_Magic"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_ad = map["Add_AD"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_ap = map["Add_AP"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_hit = map["Add_Hit"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_dodge = map["Add_Dodge"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_crit = map["Add_Crit"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.special_value = map["Special_Value"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.special_type = map["Special_Type"].as_str().unwrap_or("").to_string();
        data.add_absorb_hp = map["Add_AbsorbHP"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_immune_damage = map["Add_ImmuneDamage"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_adptr = map["Add_ADPTR"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_apptr = map["Add_APPTR"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_adptv = map["Add_ADPTV"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.add_apptv = map["Add_APPTV"].as_str().unwrap_or("0").parse().unwrap_or(0);

        data.s_type = map["s_type"].as_str().unwrap_or("").to_string();
        data.effect = map["effect"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.role = map["role"].as_str().unwrap_or("").to_string();
        data.cd = map["CD"].as_str().unwrap_or("0").parse().unwrap_or(0);
        data.b_continued = map["B_continued"].as_str().unwrap_or("0").parse().unwrap_or(0);
    }

    data
}

/// 加载物品配置 Config_Goods
pub fn load_items(conn: &Connection) -> HashMap<String, ItemDef> {
    let mut items = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT ID, Name, Type, Basic, Lock, LtemData, Introduce FROM Config_Goods"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((id, name, item_type, basic, locked, data_hex, introduce)) = row {
            let item_type = match decode_hex_gbk(&item_type).as_str() {
                "Equip" => ItemType::Equip,
                "potion" => ItemType::Potion,
                "Material" => ItemType::Material,
                "Quest" => ItemType::Quest,
                _ => ItemType::Other,
            };

            let item = ItemDef {
                id: id.clone(),
                name: decode_hex_gbk(&name),
                item_type,
                basic: decode_hex_gbk(&basic) == "TRUE",
                locked: decode_hex_gbk(&locked) == "TRUE",
                introduce: decode_hex_gbk(&introduce),
                data: parse_item_data(&data_hex),
            };

            items.insert(id, item);
        }
    }

    items
}

/// 加载怪物配置 Config_Monster
pub fn load_monsters(conn: &Connection) -> HashMap<String, MonsterDef> {
    let mut monsters = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Monster_Name, Monster_Type, Monster_AD, Monster_AP, Monster_HP, \
         Monster_Defense, Monster_AbsorbHP, Monster_ADPTV, Monster_ADPTR, \
         Monster_APPTR, Monster_APPTV, Monster_ImmuneDamage, Skills, \
         Reward_Goods, Reward_Exp, Reward_Gold, Introduce, AttackEffect, \
         AttackTips, MagicResistance, Hit, Dodge, IgnoreShield FROM Config_Monster"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, String>(17)?,
            row.get::<_, String>(18)?,
            row.get::<_, String>(19)?,
            row.get::<_, String>(20)?,
            row.get::<_, String>(21)?,
            row.get::<_, String>(22)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, mtype, ad, ap, hp, def, absorb, adptv, adptr, apptr, apptv,
                   immune, _skills_hex, rewards_hex, exp, gold, intro, atk_effect,
                   atk_tips, magic_res, hit, dodge, ignore_shield)) = row {

            let monster_type = match decode_hex_gbk(&mtype).as_str() {
                "Elite" => MonsterType::Elite,
                "Boss" => MonsterType::Boss,
                _ => MonsterType::Ordinary,
            };

            let rewards_str = decode_hex_gbk(&rewards_hex);
            let reward_goods = parse_reward_items(&rewards_str);

            let monster = MonsterDef {
                name: decode_hex_gbk(&name),
                monster_type,
                ad: decode_hex_gbk(&ad).parse().unwrap_or(0),
                ap: decode_hex_gbk(&ap).parse().unwrap_or(0),
                hp: decode_hex_gbk(&hp).parse().unwrap_or(0),
                defense: decode_hex_gbk(&def).parse().unwrap_or(0),
                absorb_hp: decode_hex_gbk(&absorb).parse().unwrap_or(0),
                adptv: decode_hex_gbk(&adptv).parse().unwrap_or(0),
                adptr: decode_hex_gbk(&adptr).parse().unwrap_or(0),
                apptr: decode_hex_gbk(&apptr).parse().unwrap_or(0),
                apptv: decode_hex_gbk(&apptv).parse().unwrap_or(0),
                immune_damage: decode_hex_gbk(&immune).parse().unwrap_or(0),
                skills: Vec::new(),
                reward_goods,
                reward_exp: decode_hex_gbk(&exp).parse().unwrap_or(0),
                reward_gold: decode_hex_gbk(&gold).parse().unwrap_or(0),
                introduce: decode_hex_gbk(&intro),
                attack_effect: decode_hex_gbk(&atk_effect),
                attack_tips: decode_hex_gbk(&atk_tips),
                magic_resistance: decode_hex_gbk(&magic_res).parse().unwrap_or(0),
                hit: decode_hex_gbk(&hit).parse().unwrap_or(0),
                dodge: decode_hex_gbk(&dodge).parse().unwrap_or(0),
                ignore_shield: decode_hex_gbk(&ignore_shield) == "TRUE",
            };

            monsters.insert(decode_hex_gbk(&name), monster);
        }
    }

    monsters
}

fn parse_reward_items(s: &str) -> Vec<RewardItem> {
    let mut items = Vec::new();

    for part in s.split(',') {
        let fields: Vec<&str> = part.split('*').collect();
        if fields.len() >= 3 {
            items.push(RewardItem {
                item_id: fields[0].to_string(),
                count: fields[1].parse().unwrap_or(1),
                rate: fields[2].parse().unwrap_or(100.0),
            });
        }
    }

    items
}

/// 加载地图配置 Config_Map
pub fn load_maps(conn: &Connection) -> HashMap<String, MapDef> {
    let mut maps = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, LV, Introduce, Security, Hid, Basis, Monster, \
         UP, Down, Left, Right, Consume, LV_UP FROM Config_Map"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, lv, intro, security, hid, basis, monster_hex,
                   up, down, left, right, consume_hex, lv_up)) = row {

            let map = MapDef {
                name: decode_hex_gbk(&name),
                lv: decode_hex_gbk(&lv).parse().unwrap_or(1),
                introduce: decode_hex_gbk(&intro),
                security: decode_hex_gbk(&security) == "TRUE",
                hid: decode_hex_gbk(&hid) == "TRUE",
                basis: decode_hex_gbk(&basis) == "TRUE",
                monsters: Vec::new(),
                up: decode_hex_gbk(&up),
                down: decode_hex_gbk(&down),
                left: decode_hex_gbk(&left),
                right: decode_hex_gbk(&right),
                consume: Vec::new(),
                lv_up: decode_hex_gbk(&lv_up).parse().unwrap_or(0),
            };

            maps.insert(decode_hex_gbk(&name), map);
        }
    }

    maps
}

/// 加载任务配置 Config_Task
pub fn load_tasks(conn: &Connection) -> HashMap<String, TaskDef> {
    let mut tasks = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Title, LV, Type, ResetTime, ResetType, CompleteTask, \
         Occupation, Target, Data, Reward_Gold, Reward_Diamonds, Reward_EXP, \
         Reward_Goods, Info FROM Config_Task"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((title, lv, ttype, reset_time, reset_type, complete,
                   occup, target_hex, _data, gold, diamonds, exp, goods_hex, info)) = row {

            let task_type = match decode_hex_gbk(&ttype).as_str() {
                "主线" => TaskType::Main,
                "支线" => TaskType::Branch,
                "日常" => TaskType::Daily,
                _ => TaskType::Other,
            };

            let title_decoded = decode_hex_gbk(&title);

            // 解析Target字段
            let target_str = decode_hex_gbk(&target_hex);
            let target = parse_task_target(&target_str);

            let task = TaskDef {
                title: title_decoded.clone(),
                lv: decode_hex_gbk(&lv).parse().unwrap_or(1),
                task_type,
                reset_time: decode_hex_gbk(&reset_time).parse().unwrap_or(-1),
                reset_type: decode_hex_gbk(&reset_type).parse().unwrap_or(5),
                complete_task: decode_hex_gbk(&complete),
                occupation: decode_hex_gbk(&occup),
                target,
                reward_gold: decode_hex_gbk(&gold).parse().unwrap_or(0),
                reward_diamonds: decode_hex_gbk(&diamonds).parse().unwrap_or(0),
                reward_exp: decode_hex_gbk(&exp).parse().unwrap_or(0),
                reward_goods: parse_reward_items(&decode_hex_gbk(&goods_hex)),
                info: decode_hex_gbk(&info),
            };

            tasks.insert(title_decoded, task);
        }
    }

    tasks
}

fn parse_task_target(s: &str) -> TaskTarget {
    // 解析 [Target]\n怪物名=数量 格式
    let mut target = TaskTarget {
        target_type: "Monster".to_string(),
        target_id: String::new(),
        count: 1,
    };

    for line in s.lines() {
        if line.starts_with('[') || line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            target.target_id = key.to_string();
            target.count = value.parse().unwrap_or(1);
        }
    }

    target
}

/// 加载合成配方 Config_Composite
pub fn load_composites(conn: &Connection) -> HashMap<String, CompositeRecipe> {
    let mut recipes = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Produce, ConsumeGoods, ConsumeGold, ConsumeDiamond, Success FROM Config_Composite"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((produce, goods_hex, gold, diamond, success)) = row {
            let produce_name = decode_hex_gbk(&produce);

            let goods_str = decode_hex_gbk(&goods_hex);
            let mut consume_goods = HashMap::new();
            if let Ok(map) = serde_json::from_str::<serde_json::Value>(&goods_str) {
                if let Some(obj) = map.as_object() {
                    for (k, v) in obj {
                        if let Some(count) = v.as_str().and_then(|s| s.parse::<i32>().ok()) {
                            consume_goods.insert(k.clone(), count);
                        }
                    }
                }
            }

            let recipe = CompositeRecipe {
                produce: produce_name.clone(),
                consume_goods,
                consume_gold: decode_hex_gbk(&gold).parse().unwrap_or(0),
                consume_diamond: decode_hex_gbk(&diamond).parse().unwrap_or(0),
                success_rate: decode_hex_gbk(&success).parse().unwrap_or(100),
            };

            recipes.insert(produce_name, recipe);
        }
    }

    recipes
}

/// 加载商店配置 Config_Shop
pub fn load_shops(conn: &Connection) -> HashMap<String, Vec<ShopItem>> {
    let mut shops = HashMap::new();
    let mut all_items = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Currency, Price, LimitNumber, LimitType FROM Config_Shop"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, currency, price, limit_num, limit_type)) = row {
            let item = ShopItem {
                name: decode_hex_gbk(&name),
                currency: decode_hex_gbk(&currency),
                price: decode_hex_gbk(&price).parse().unwrap_or(0),
                limit_number: decode_hex_gbk(&limit_num).parse().unwrap_or(0),
                limit_type: decode_hex_gbk(&limit_type),
            };
            all_items.push(item);
        }
    }

    shops.insert("default".to_string(), all_items);
    shops
}

/// 加载套装配置 Config_Suit
pub fn load_suits(conn: &Connection) -> HashMap<String, SuitDef> {
    let mut suits = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Add_HP, Add_MP, Add_Defense, Add_Magic, Add_AD, Add_AP, \
         Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, Add_ADPTV, Add_ADPTR, \
         Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value \
         FROM Config_Suit"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, String>(17)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, hp, mp, def, mag, ad, ap, hit, dodge, crit, absorb,
                   adptv, adptr, apptr, apptv, immune, stype, sval)) = row {

            let suit = SuitDef {
                name: decode_hex_gbk(&name),
                add_hp: decode_hex_gbk(&hp).parse().unwrap_or(0),
                add_mp: decode_hex_gbk(&mp).parse().unwrap_or(0),
                add_defense: decode_hex_gbk(&def).parse().unwrap_or(0),
                add_magic: decode_hex_gbk(&mag).parse().unwrap_or(0),
                add_ad: decode_hex_gbk(&ad).parse().unwrap_or(0),
                add_ap: decode_hex_gbk(&ap).parse().unwrap_or(0),
                add_hit: decode_hex_gbk(&hit).parse().unwrap_or(0),
                add_dodge: decode_hex_gbk(&dodge).parse().unwrap_or(0),
                add_crit: decode_hex_gbk(&crit).parse().unwrap_or(0),
                add_absorb_hp: decode_hex_gbk(&absorb).parse().unwrap_or(0),
                add_adptv: decode_hex_gbk(&adptv).parse().unwrap_or(0),
                add_adptr: decode_hex_gbk(&adptr).parse().unwrap_or(0),
                add_apptr: decode_hex_gbk(&apptr).parse().unwrap_or(0),
                add_apptv: decode_hex_gbk(&apptv).parse().unwrap_or(0),
                add_immune_damage: decode_hex_gbk(&immune).parse().unwrap_or(0),
                special_type: decode_hex_gbk(&stype),
                special_value: decode_hex_gbk(&sval).parse().unwrap_or(0),
            };

            suits.insert(decode_hex_gbk(&name), suit);
        }
    }

    suits
}

/// 加载技能配置 Config_Skills
pub fn load_skills(conn: &Connection) -> HashMap<String, SkillDef> {
    let mut skills = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Type, Consume, Effect, EO, LV, ConsumeType, Cooling, \
         Accurate, AttackTips, Introduce, ACS, Shield, IgnoreShield, IgnoreIM, \
         IgnoreRE, BanAbsorb, BanMultipleShot, ProhibitUO, ConsumableGoods, \
         Continued_Round, Continued_Type, Continued_Effect FROM Config_Skills"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, String>(17)?,
            row.get::<_, String>(18)?,
            row.get::<_, String>(19)?,
            row.get::<_, String>(20)?,
            row.get::<_, String>(21)?,
            row.get::<_, String>(22)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, _type, consume, effect, _eo, lv, _consume_type, cooling,
                   _accurate, _atk_tips, introduce, _acs, _shield, _ignore_shield,
                   _ignore_im, _ignore_re, _ban_absorb, _ban_multi, _prohibit_uo,
                   _consumable, _cont_round, _cont_type, _cont_effect)) = row {

            let skill = SkillDef {
                name: decode_hex_gbk(&name),
                hp: 0,
                mp: decode_hex_gbk(&consume).parse().unwrap_or(0),
                ad: 0,
                ap: 0,
                def: 0,
                mdf: 0,
                hit: decode_hex_gbk(&_accurate).parse().unwrap_or(100),
                sb: 0,
                xx: 0,
                bj: 0,
                ms: 0,
                wz: String::new(),
                cdr: decode_hex_gbk(&cooling).parse().unwrap_or(0),
                combo: 0,
                combo_time: 0,
                combo_need: 0,
            };

            skills.insert(decode_hex_gbk(&name), skill);
        }
    }

    skills
}

/// 加载职业配置 Config_Occupation
pub fn load_occupations(conn: &Connection) -> HashMap<String, OccupationDef> {
    let mut occupations = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Basics, HP, MP, AD, AP, Defense, Hit, Dodge, Crit, \
         AbsorbHP, ADPTV, ADPTR, APPTR, APPTV, ImmuneDamage, Intro, \
         ExclusiveSkills, TransferDemand, TransferLevel, FormerOccupation, \
         Belong, AttackEffect, AttackTips, MagicResistance, IgnoreShield \
         FROM Config_Occupation"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            row.get::<_, String>(16)?,
            row.get::<_, String>(17)?,
            row.get::<_, String>(18)?,
            row.get::<_, String>(19)?,
            row.get::<_, String>(20)?,
            row.get::<_, String>(21)?,
            row.get::<_, String>(22)?,
            row.get::<_, String>(23)?,
            row.get::<_, String>(24)?,
            row.get::<_, String>(25)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, _basics, hp, mp, ad, ap, defense, hit, dodge, crit,
                   absorb_hp, adptv, adptr, apptr, apptv, immune_damage,
                   _intro, _exclusive, _transfer_demand, _transfer_lv,
                   _former, _belong, _atk_effect, _atk_tips, _magic_res,
                   _ignore_shield)) = row {

            let occupation = OccupationDef {
                name: decode_hex_gbk(&name),
                base_hp: decode_hex_gbk(&hp).parse().unwrap_or(100),
                base_mp: decode_hex_gbk(&mp).parse().unwrap_or(50),
                base_ad: decode_hex_gbk(&ad).parse().unwrap_or(10),
                base_ap: decode_hex_gbk(&ap).parse().unwrap_or(10),
                base_defense: decode_hex_gbk(&defense).parse().unwrap_or(5),
                base_magic: 5,
                growth_hp: 10,
                growth_mp: 5,
                growth_ad: 2,
                growth_ap: 2,
                growth_defense: 1,
                growth_magic: 1,
            };

            occupations.insert(decode_hex_gbk(&name), occupation);
        }
    }

    occupations
}

/// 加载NPC信息 Ext_NPC_Info
pub fn load_npcs(conn: &Connection) -> HashMap<String, NpcDef> {
    let mut npcs = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Location, Function, Dialog, Introduce FROM Ext_NPC_Info"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, location, function, dialog, introduce)) = row {
            let npc = NpcDef {
                name: decode_hex_gbk(&name),
                location: decode_hex_gbk(&location),
                function: decode_hex_gbk(&function),
                dialog: decode_hex_gbk(&dialog),
                introduce: decode_hex_gbk(&introduce),
            };

            npcs.insert(decode_hex_gbk(&name), npc);
        }
    }

    npcs
}

/// 加载分解配置 Config_Decomposition
pub fn load_decompositions(conn: &Connection) -> HashMap<String, DecompositionDef> {
    let mut decompositions = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Goods, NeedG, NeedD, GetGoods, Success FROM Config_Decomposition"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((goods, need_g, need_d, get_goods, success)) = row {
            let decomposition = DecompositionDef {
                goods: decode_hex_gbk(&goods),
                need_gold: decode_hex_gbk(&need_g).parse().unwrap_or(0),
                need_diamond: decode_hex_gbk(&need_d).parse().unwrap_or(0),
                get_goods: decode_hex_gbk(&get_goods),
                success_rate: decode_hex_gbk(&success).parse().unwrap_or(100),
            };

            decompositions.insert(decode_hex_gbk(&goods), decomposition);
        }
    }

    decompositions
}

/// 加载帮助配置 Config_Help
pub fn load_helps(conn: &Connection) -> HashMap<String, String> {
    let mut helps = HashMap::new();

    let mut stmt = conn.prepare("SELECT Help, HelpData FROM Config_Help").unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((help, data)) = row {
            helps.insert(decode_hex_gbk(&help), decode_hex_gbk(&data));
        }
    }

    helps
}

/// 加载私人商店 Config_PrivateShops
pub fn load_private_shops(conn: &Connection) -> HashMap<String, PrivateShopDef> {
    let mut shops = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT ID, Name, GoodsData, Open FROM Config_PrivateShops"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((id, name, goods_data, open)) = row {
            let shop = PrivateShopDef {
                id: decode_hex_gbk(&id),
                name: decode_hex_gbk(&name),
                goods_data: decode_hex_gbk(&goods_data),
                open: decode_hex_gbk(&open) == "TRUE",
            };

            shops.insert(decode_hex_gbk(&id), shop);
        }
    }

    shops
}

/// 加载食物饥饿值 Foods_Hungervalue
pub fn load_foods(conn: &Connection) -> HashMap<String, FoodDef> {
    let mut foods = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT Name, Value, Price FROM Foods_Hungervalue"
    ).unwrap();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }).unwrap();

    for row in rows {
        if let Ok((name, value, price)) = row {
            let food = FoodDef {
                name: decode_hex_gbk(&name),
                value: decode_hex_gbk(&value).parse().unwrap_or(0),
                price: decode_hex_gbk(&price).parse().unwrap_or(0),
            };

            foods.insert(decode_hex_gbk(&name), food);
        }
    }

    foods
}

/// 加载整个数据库
pub fn load_database(db_path: &str) -> Result<GameEngine, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut engine = GameEngine::new();

    // 加载所有配置表
    engine.items = load_items(&conn);
    engine.monsters = load_monsters(&conn);
    engine.maps = load_maps(&conn);
    engine.tasks = load_tasks(&conn);
    engine.composites = load_composites(&conn);
    engine.shops = load_shops(&conn);
    engine.suits = load_suits(&conn);
    engine.skills = load_skills(&conn);
    engine.occupations = load_occupations(&conn);
    engine.npcs = load_npcs(&conn);
    engine.decompositions = load_decompositions(&conn);
    engine.helps = load_helps(&conn);
    engine.private_shops = load_private_shops(&conn);
    engine.foods = load_foods(&conn);

    Ok(engine)
}
