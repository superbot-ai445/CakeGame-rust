#![allow(dead_code)]

//! 血脉觉醒系统 (Bloodline Awakening System)
//!
//! 远古血脉赋予玩家独特力量，觉醒祖先之力获得永久属性加成。
//! - 10种远古血脉: 龙/凤凰/白虎/玄武/青龙/朱雀/麒麟/九尾/饕餮/烛龙
//! - 7级血脉觉醒等级: 未觉醒→初觉→微觉→中觉→强觉→极觉→神觉
//! - 血脉之力收集: 击败BOSS/PvP/深渊/竞技场/公会战/世界BOSS获得血脉之力
//! - 血脉共鸣: 同系血脉2/3件共鸣额外加成
//! - 血脉传承: 高级血脉解锁被动技能
//! - 全服血脉排行

use crate::core::ITEM_NAME;
use crate::db::Database;
use crate::user;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 血脉类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BloodlineType {
    Dragon,        // 龙 - 物攻
    Phoenix,       // 凤凰 - 魔攻
    WhiteTiger,    // 白虎 - 暴击
    BlackTortoise, // 玄武 - 防御
    AzureDragon,   // 青龙 - HP
    VermilionBird, // 朱雀 - 魔抗
    Qilin,         // 麒麟 - 全属性
    NineTails,     // 九尾 - 闪避
    Taotie,        // 饕餮 - 吸血
    Zhurong,       // 烛龙 - 穿透
}

impl BloodlineType {
    pub fn all() -> &'static [BloodlineType] {
        &[
            Self::Dragon,
            Self::Phoenix,
            Self::WhiteTiger,
            Self::BlackTortoise,
            Self::AzureDragon,
            Self::VermilionBird,
            Self::Qilin,
            Self::NineTails,
            Self::Taotie,
            Self::Zhurong,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dragon => "龙血",
            Self::Phoenix => "凤血",
            Self::WhiteTiger => "虎血",
            Self::BlackTortoise => "龟血",
            Self::AzureDragon => "青龙血",
            Self::VermilionBird => "朱雀血",
            Self::Qilin => "麒麟血",
            Self::NineTails => "狐血",
            Self::Taotie => "饕餮血",
            Self::Zhurong => "烛龙血",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Dragon => "🐉",
            Self::Phoenix => "🔥",
            Self::WhiteTiger => "🐯",
            Self::BlackTortoise => "🐢",
            Self::AzureDragon => "🐲",
            Self::VermilionBird => "🐦",
            Self::Qilin => "🦄",
            Self::NineTails => "🦊",
            Self::Taotie => "👹",
            Self::Zhurong => "☀️",
        }
    }

    pub fn desc(&self) -> &'static str {
        match self {
            Self::Dragon => "远古龙族之血，蕴含毁天灭地的物理力量",
            Self::Phoenix => "涅槃凤凰之血，浴火重生的魔法源泉",
            Self::WhiteTiger => "西方白虎之血，一击必杀的暴击之力",
            Self::BlackTortoise => "北方玄武之血，坚不可摧的防御屏障",
            Self::AzureDragon => "东方青龙之血，浩瀚无穷的生命之力",
            Self::VermilionBird => "南方朱雀之血，焚尽一切的魔抗之力",
            Self::Qilin => "祥瑞麒麟之血，均衡万物的全属性之力",
            Self::NineTails => "九尾妖狐之血，飘渺无形的闪避之力",
            Self::Taotie => "贪婪饕餮之血，吞噬万物的吸血之力",
            Self::Zhurong => "火神烛龙之血，洞穿一切的穿透之力",
        }
    }

    /// 主属性类型 (用于加成计算)
    pub fn primary_attr(&self) -> &'static str {
        match self {
            Self::Dragon => "物攻",
            Self::Phoenix => "魔攻",
            Self::WhiteTiger => "暴击",
            Self::BlackTortoise => "防御",
            Self::AzureDragon => "HP",
            Self::VermilionBird => "魔抗",
            Self::Qilin => "全属性",
            Self::NineTails => "闪避",
            Self::Taotie => "吸血",
            Self::Zhurong => "穿透",
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Dragon),
            1 => Some(Self::Phoenix),
            2 => Some(Self::WhiteTiger),
            3 => Some(Self::BlackTortoise),
            4 => Some(Self::AzureDragon),
            5 => Some(Self::VermilionBird),
            6 => Some(Self::Qilin),
            7 => Some(Self::NineTails),
            8 => Some(Self::Taotie),
            9 => Some(Self::Zhurong),
            _ => None,
        }
    }

    pub fn to_id(self) -> u8 {
        match self {
            Self::Dragon => 0,
            Self::Phoenix => 1,
            Self::WhiteTiger => 2,
            Self::BlackTortoise => 3,
            Self::AzureDragon => 4,
            Self::VermilionBird => 5,
            Self::Qilin => 6,
            Self::NineTails => 7,
            Self::Taotie => 8,
            Self::Zhurong => 9,
        }
    }
}

/// 觉醒等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AwakeningLevel {
    Dormant = 0, // 未觉醒
    Initial = 1, // 初觉
    Slight = 2,  // 微觉
    Medium = 3,  // 中觉
    Strong = 4,  // 强觉
    Extreme = 5, // 极觉
    Divine = 6,  // 神觉
}

impl AwakeningLevel {
    pub fn all() -> &'static [AwakeningLevel] {
        &[
            Self::Dormant,
            Self::Initial,
            Self::Slight,
            Self::Medium,
            Self::Strong,
            Self::Extreme,
            Self::Divine,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dormant => "未觉醒",
            Self::Initial => "初觉",
            Self::Slight => "微觉",
            Self::Medium => "中觉",
            Self::Strong => "强觉",
            Self::Extreme => "极觉",
            Self::Divine => "神觉",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Dormant => "⚪",
            Self::Initial => "🟢",
            Self::Slight => "🔵",
            Self::Medium => "🟣",
            Self::Strong => "🟠",
            Self::Extreme => "🔴",
            Self::Divine => "🟡",
        }
    }

    /// 觉醒所需的血脉之力
    pub fn required_power(&self) -> u64 {
        match self {
            Self::Dormant => 0,
            Self::Initial => 100,
            Self::Slight => 500,
            Self::Medium => 2000,
            Self::Strong => 8000,
            Self::Extreme => 30000,
            Self::Divine => 100000,
        }
    }

    /// 觉醒所需的玩家等级
    pub fn required_level(&self) -> u32 {
        match self {
            Self::Dormant => 0,
            Self::Initial => 10,
            Self::Slight => 25,
            Self::Medium => 40,
            Self::Strong => 55,
            Self::Extreme => 70,
            Self::Divine => 90,
        }
    }

    /// 觉醒所需的金币
    pub fn required_gold(&self) -> u64 {
        match self {
            Self::Dormant => 0,
            Self::Initial => 5000,
            Self::Slight => 20000,
            Self::Medium => 80000,
            Self::Strong => 300000,
            Self::Extreme => 1000000,
            Self::Divine => 5000000,
        }
    }

    pub fn next(&self) -> Option<AwakeningLevel> {
        match self {
            Self::Dormant => Some(Self::Initial),
            Self::Initial => Some(Self::Slight),
            Self::Slight => Some(Self::Medium),
            Self::Medium => Some(Self::Strong),
            Self::Strong => Some(Self::Extreme),
            Self::Extreme => Some(Self::Divine),
            Self::Divine => None,
        }
    }

    /// 前一级显示（用于觉醒提示）
    pub fn prev_display(&self) -> &str {
        match self {
            Self::Dormant => "未觉醒",
            Self::Initial => "未觉醒",
            Self::Slight => "初觉",
            Self::Medium => "微觉",
            Self::Strong => "中觉",
            Self::Extreme => "强觉",
            Self::Divine => "极觉",
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Dormant,
            1 => Self::Initial,
            2 => Self::Slight,
            3 => Self::Medium,
            4 => Self::Strong,
            5 => Self::Extreme,
            _ => Self::Divine,
        }
    }
}

/// 血脉被动技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodlineSkill {
    pub name: &'static str,
    pub desc: &'static str,
    pub unlock_level: AwakeningLevel,
    pub trigger_pct: f64,       // 触发概率%
    pub effect_multiplier: f64, // 效果倍率
}

/// 获取血脉被动技能列表
pub fn get_bloodline_skills(bt: BloodlineType) -> Vec<BloodlineSkill> {
    match bt {
        BloodlineType::Dragon => vec![
            BloodlineSkill {
                name: "龙息",
                desc: "攻击时15%概率触发龙息，造成1.5倍物攻伤害",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 15.0,
                effect_multiplier: 1.5,
            },
            BloodlineSkill {
                name: "逆鳞怒",
                desc: "HP低于30%时物攻+50%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.5,
            },
        ],
        BloodlineType::Phoenix => vec![
            BloodlineSkill {
                name: "涅槃焰",
                desc: "攻击时15%概率触发涅槃焰，造成1.5倍魔攻伤害",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 15.0,
                effect_multiplier: 1.5,
            },
            BloodlineSkill {
                name: "浴火重生",
                desc: "死亡时20%概率复活并恢复30%HP",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 20.0,
                effect_multiplier: 0.3,
            },
        ],
        BloodlineType::WhiteTiger => vec![
            BloodlineSkill {
                name: "虎啸",
                desc: "攻击时20%概率暴击率+30%",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 20.0,
                effect_multiplier: 1.3,
            },
            BloodlineSkill {
                name: "啸风斩",
                desc: "暴击时额外造成目标当前HP 8%伤害",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 0.08,
            },
        ],
        BloodlineType::BlackTortoise => vec![
            BloodlineSkill {
                name: "玄甲",
                desc: "受击时20%概率防御+50%持续3回合",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 20.0,
                effect_multiplier: 1.5,
            },
            BloodlineSkill {
                name: "不动如山",
                desc: "HP低于20%时免疫所有伤害1回合",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.0,
            },
        ],
        BloodlineType::AzureDragon => vec![
            BloodlineSkill {
                name: "青龙吐珠",
                desc: "每回合自动恢复5%最大HP",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 100.0,
                effect_multiplier: 0.05,
            },
            BloodlineSkill {
                name: "生命洪流",
                desc: "受治疗效果+80%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.8,
            },
        ],
        BloodlineType::VermilionBird => vec![
            BloodlineSkill {
                name: "焚天羽",
                desc: "受击时15%概率反弹30%伤害",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 15.0,
                effect_multiplier: 0.3,
            },
            BloodlineSkill {
                name: "朱雀之翼",
                desc: "魔抗+60%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.6,
            },
        ],
        BloodlineType::Qilin => vec![
            BloodlineSkill {
                name: "祥瑞降临",
                desc: "攻击时10%概率全属性+20%持续3回合",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 10.0,
                effect_multiplier: 1.2,
            },
            BloodlineSkill {
                name: "天降祥瑞",
                desc: "金币和经验获取+30%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.3,
            },
        ],
        BloodlineType::NineTails => vec![
            BloodlineSkill {
                name: "狐魅",
                desc: "受击时25%概率闪避本次攻击",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 25.0,
                effect_multiplier: 1.0,
            },
            BloodlineSkill {
                name: "幻狐千面",
                desc: "闪避后下次攻击伤害+100%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 2.0,
            },
        ],
        BloodlineType::Taotie => vec![
            BloodlineSkill {
                name: "吞噬",
                desc: "攻击时20%概率吸取造成伤害的15%为HP",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 20.0,
                effect_multiplier: 0.15,
            },
            BloodlineSkill {
                name: "饕餮盛宴",
                desc: "击败敌人恢复50%最大HP",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 0.5,
            },
        ],
        BloodlineType::Zhurong => vec![
            BloodlineSkill {
                name: "穿心",
                desc: "攻击时15%概率无视目标30%防御",
                unlock_level: AwakeningLevel::Medium,
                trigger_pct: 15.0,
                effect_multiplier: 0.7,
            },
            BloodlineSkill {
                name: "焚天烈焰",
                desc: "穿透伤害+50%",
                unlock_level: AwakeningLevel::Extreme,
                trigger_pct: 100.0,
                effect_multiplier: 1.5,
            },
        ],
    }
}

/// 血脉属性加成 (每级觉醒)
pub fn get_bloodline_bonus_per_level(bt: BloodlineType, level: AwakeningLevel) -> BloodlineBonus {
    let multiplier = level as u32 as f64;
    match bt {
        BloodlineType::Dragon => BloodlineBonus {
            hp: 0,
            ad: (50.0 * multiplier) as i64,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::Phoenix => BloodlineBonus {
            hp: 0,
            ad: 0,
            ap: (50.0 * multiplier) as i64,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::WhiteTiger => BloodlineBonus {
            hp: 0,
            ad: (20.0 * multiplier) as i64,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 2.0 * multiplier,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::BlackTortoise => BloodlineBonus {
            hp: (100.0 * multiplier) as i64,
            ad: 0,
            ap: 0,
            def: (30.0 * multiplier) as i64,
            mres: (20.0 * multiplier) as i64,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::AzureDragon => BloodlineBonus {
            hp: (200.0 * multiplier) as i64,
            ad: 0,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::VermilionBird => BloodlineBonus {
            hp: 0,
            ad: 0,
            ap: (20.0 * multiplier) as i64,
            def: 0,
            mres: (30.0 * multiplier) as i64,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::Qilin => BloodlineBonus {
            hp: (80.0 * multiplier) as i64,
            ad: (25.0 * multiplier) as i64,
            ap: (25.0 * multiplier) as i64,
            def: (15.0 * multiplier) as i64,
            mres: (15.0 * multiplier) as i64,
            crit: 1.0 * multiplier,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::NineTails => BloodlineBonus {
            hp: 0,
            ad: (15.0 * multiplier) as i64,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 3.0 * multiplier,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::Taotie => BloodlineBonus {
            hp: (50.0 * multiplier) as i64,
            ad: (30.0 * multiplier) as i64,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 2.0 * multiplier,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
        BloodlineType::Zhurong => BloodlineBonus {
            hp: 0,
            ad: (40.0 * multiplier) as i64,
            ap: (20.0 * multiplier) as i64,
            def: 0,
            mres: 0,
            crit: 0.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 3.0 * multiplier,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        },
    }
}

/// 血脉属性加成结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BloodlineBonus {
    pub hp: i64,
    pub ad: i64,
    pub ap: i64,
    pub def: i64,
    pub mres: i64,
    pub crit: f64,
    pub dodge: f64,
    pub lifesteal: f64,
    pub penetrate: f64,
    pub exp_bonus: f64,
    pub gold_bonus: f64,
}

impl BloodlineBonus {
    pub fn merge(&mut self, other: &BloodlineBonus) {
        self.hp += other.hp;
        self.ad += other.ad;
        self.ap += other.ap;
        self.def += other.def;
        self.mres += other.mres;
        self.crit += other.crit;
        self.dodge += other.dodge;
        self.lifesteal += other.lifesteal;
        self.penetrate += other.penetrate;
        self.exp_bonus += other.exp_bonus;
        self.gold_bonus += other.gold_bonus;
    }

    pub fn is_empty(&self) -> bool {
        self.hp == 0
            && self.ad == 0
            && self.ap == 0
            && self.def == 0
            && self.mres == 0
            && self.crit == 0.0
            && self.dodge == 0.0
            && self.lifesteal == 0.0
            && self.penetrate == 0.0
            && self.exp_bonus == 0.0
            && self.gold_bonus == 0.0
    }

    pub fn format_display(&self) -> String {
        let mut parts = Vec::new();
        if self.hp > 0 {
            parts.push(format!("HP+{}", self.hp));
        }
        if self.ad > 0 {
            parts.push(format!("物攻+{}", self.ad));
        }
        if self.ap > 0 {
            parts.push(format!("魔攻+{}", self.ap));
        }
        if self.def > 0 {
            parts.push(format!("防御+{}", self.def));
        }
        if self.mres > 0 {
            parts.push(format!("魔抗+{}", self.mres));
        }
        if self.crit > 0.0 {
            parts.push(format!("暴击+{:.1}%", self.crit));
        }
        if self.dodge > 0.0 {
            parts.push(format!("闪避+{:.1}%", self.dodge));
        }
        if self.lifesteal > 0.0 {
            parts.push(format!("吸血+{:.1}%", self.lifesteal));
        }
        if self.penetrate > 0.0 {
            parts.push(format!("穿透+{:.1}%", self.penetrate));
        }
        if self.exp_bonus > 0.0 {
            parts.push(format!("经验+{:.1}%", self.exp_bonus));
        }
        if self.gold_bonus > 0.0 {
            parts.push(format!("金币+{:.1}%", self.gold_bonus));
        }
        parts.join(" ")
    }
}

/// 血脉共鸣组 (同系3条血脉)
#[derive(Debug, Clone)]
pub struct BloodlineResonanceGroup {
    pub name: &'static str,
    pub members: [BloodlineType; 3],
    pub bonus_desc: &'static str,
    pub bonus_hp: i64,
    pub bonus_ad: i64,
    pub bonus_ap: i64,
    pub bonus_def: i64,
    pub bonus_mres: i64,
}

/// 获取所有共鸣组
pub fn get_resonance_groups() -> Vec<BloodlineResonanceGroup> {
    vec![
        BloodlineResonanceGroup {
            name: "四象之力",
            members: [
                BloodlineType::AzureDragon,
                BloodlineType::WhiteTiger,
                BloodlineType::BlackTortoise,
            ],
            bonus_desc: "三象觉醒共鸣: 全属性+5%",
            bonus_hp: 300,
            bonus_ad: 60,
            bonus_ap: 40,
            bonus_def: 50,
            bonus_mres: 40,
        },
        BloodlineResonanceGroup {
            name: "圣兽之血",
            members: [BloodlineType::Dragon, BloodlineType::Phoenix, BloodlineType::Qilin],
            bonus_desc: "圣兽觉醒共鸣: 全属性+8%",
            bonus_hp: 500,
            bonus_ad: 100,
            bonus_ap: 100,
            bonus_def: 80,
            bonus_mres: 80,
        },
        BloodlineResonanceGroup {
            name: "暗影之力",
            members: [BloodlineType::NineTails, BloodlineType::Taotie, BloodlineType::Zhurong],
            bonus_desc: "暗影觉醒共鸣: 暴击+5% 吸血+3% 穿透+3%",
            bonus_hp: 200,
            bonus_ad: 80,
            bonus_ap: 60,
            bonus_def: 30,
            bonus_mres: 30,
        },
    ]
}

/// 血脉之力来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSource {
    BossKill,    // 击败BOSS
    PvpWin,      // PvP胜利
    AbyssClear,  // 深渊通关
    ArenaWin,    // 竞技场胜利
    GuildWar,    // 公会战
    WorldBoss,   // 世界BOSS
    DailySignIn, // 每日签到
    Quest,       // 完成任务
}

impl PowerSource {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BossKill => "击败BOSS",
            Self::PvpWin => "PvP胜利",
            Self::AbyssClear => "深渊通关",
            Self::ArenaWin => "竞技场胜利",
            Self::GuildWar => "公会战",
            Self::WorldBoss => "世界BOSS",
            Self::DailySignIn => "每日签到",
            Self::Quest => "完成任务",
        }
    }

    /// 每次获取的血脉之力
    pub fn power_amount(&self) -> u64 {
        match self {
            Self::BossKill => 15,
            Self::PvpWin => 10,
            Self::AbyssClear => 12,
            Self::ArenaWin => 8,
            Self::GuildWar => 20,
            Self::WorldBoss => 25,
            Self::DailySignIn => 5,
            Self::Quest => 6,
        }
    }

    /// 每日上限
    pub fn daily_cap(&self) -> u64 {
        match self {
            Self::BossKill => 150,
            Self::PvpWin => 100,
            Self::AbyssClear => 120,
            Self::ArenaWin => 80,
            Self::GuildWar => 100,
            Self::WorldBoss => 75,
            Self::DailySignIn => 5,
            Self::Quest => 60,
        }
    }
}

/// 单条血脉数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodlineEntry {
    pub bloodline_type: BloodlineType,
    pub level: AwakeningLevel,
    pub power_accumulated: u64,
    pub awakened_at: Option<String>,
}

impl BloodlineEntry {
    pub fn new(bt: BloodlineType) -> Self {
        Self {
            bloodline_type: bt,
            level: AwakeningLevel::Dormant,
            power_accumulated: 0,
            awakened_at: None,
        }
    }

    /// 战力贡献
    pub fn combat_power(&self) -> u64 {
        let base = self.level as u64 * 100;
        let bonus = get_bloodline_bonus_per_level(self.bloodline_type, self.level);
        base + (bonus.hp / 10 + bonus.ad / 2 + bonus.ap / 2 + bonus.def / 3 + bonus.mres / 3) as u64
    }
}

/// 玩家血脉数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodlineData {
    pub entries: Vec<BloodlineEntry>,
    pub daily_power: HashMap<String, u64>, // "source" -> 今日获取
    pub last_reset_date: String,
}
#[allow(clippy::derivable_impls)]
impl Default for BloodlineData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            daily_power: HashMap::new(),
            last_reset_date: String::new(),
        }
    }
}

impl BloodlineData {
    pub fn get_or_create(&mut self, bt: BloodlineType) -> &mut BloodlineEntry {
        if !self.entries.iter().any(|e| e.bloodline_type == bt) {
            self.entries.push(BloodlineEntry::new(bt));
        }
        self.entries.iter_mut().find(|e| e.bloodline_type == bt).unwrap()
    }

    pub fn has_bloodline(&self, bt: BloodlineType) -> bool {
        self.entries.iter().any(|e| e.bloodline_type == bt)
    }

    /// 检查并重置每日计数
    pub fn check_daily_reset(&mut self, today: &str) {
        if self.last_reset_date != today {
            self.daily_power.clear();
            self.last_reset_date = today.to_string();
        }
    }

    /// 记录血脉之力获取 (返回实际获取量)
    pub fn record_power_gain(&mut self, source: PowerSource, today: &str) -> u64 {
        self.check_daily_reset(today);
        let key = format!("{:?}", source);
        let current = *self.daily_power.get(&key).unwrap_or(&0);
        let cap = source.daily_cap();
        if current >= cap {
            return 0;
        }
        let gain = source.power_amount().min(cap - current);
        *self.daily_power.entry(key).or_insert(0) += gain;
        gain
    }

    /// 获取今日某种来源已获取量
    pub fn get_daily_used(&self, source: PowerSource) -> u64 {
        let key = format!("{:?}", source);
        *self.daily_power.get(&key).unwrap_or(&0)
    }

    /// 计算总血脉战力
    pub fn total_combat_power(&self) -> u64 {
        self.entries.iter().map(|e| e.combat_power()).sum()
    }

    /// 计算所有血脉的总属性加成
    pub fn total_bonus(&self) -> BloodlineBonus {
        let mut total = BloodlineBonus::default();
        for entry in &self.entries {
            if entry.level > AwakeningLevel::Dormant {
                let bonus = get_bloodline_bonus_per_level(entry.bloodline_type, entry.level);
                total.merge(&bonus);
            }
        }

        // 检查共鸣组加成
        for group in get_resonance_groups() {
            let all_awakened = group.members.iter().all(|bt| {
                self.entries
                    .iter()
                    .any(|e| e.bloodline_type == *bt && e.level > AwakeningLevel::Dormant)
            });
            if all_awakened {
                total.hp += group.bonus_hp;
                total.ad += group.bonus_ad;
                total.ap += group.bonus_ap;
                total.def += group.bonus_def;
                total.mres += group.bonus_mres;
            }
        }

        total
    }

    /// 获取已觉醒的共鸣组
    pub fn get_active_resonances(&self) -> Vec<&'static str> {
        let mut result = Vec::new();
        for group in get_resonance_groups() {
            let all_awakened = group.members.iter().all(|bt| {
                self.entries
                    .iter()
                    .any(|e| e.bloodline_type == *bt && e.level > AwakeningLevel::Dormant)
            });
            if all_awakened {
                result.push(group.name);
            }
        }
        result
    }
}

/// 血脉战力排名条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodlineRanking {
    pub user_id: i64,
    pub nickname: String,
    pub total_power: u64,
    pub bloodline_count: usize,
    pub highest_level: AwakeningLevel,
}

// ===================== 公共API =====================

/// 获取血脉属性加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_bloodline_bonus(data: &BloodlineData) -> BloodlineBonus {
    data.total_bonus()
}

/// 记录血脉之力获取 (供外部系统调用)
#[allow(dead_code)]
pub fn record_bloodline_power(data: &mut BloodlineData, source: PowerSource, today: &str) -> u64 {
    data.record_power_gain(source, today)
}

/// 查看血脉 — 展示玩家所有血脉状态
pub fn cmd_bloodline_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("bloodline", user_id);
    let data: BloodlineData = if raw.is_empty() {
        BloodlineData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let mut out = format!("{}\n═══ 血脉觉醒 ═══\n", prefix);
    if data.entries.is_empty() {
        out.push_str("你尚未觉醒任何血脉。\n");
        out.push_str("使用「觉醒血脉+血脉名」开始觉醒之旅。\n");
        out.push_str("血脉之力: 0 | 总战力: 0\n");
        return out;
    }

    for entry in &data.entries {
        let bt = entry.bloodline_type;
        let marker = if entry.level > AwakeningLevel::Dormant {
            "✅"
        } else {
            "⬜"
        };
        out.push_str(&format!(
            "{} {}{} Lv.{} — 战力:{}\n",
            marker,
            bt.icon(),
            bt.name(),
            entry.level.name(),
            entry.combat_power()
        ));
    }

    let bonus = data.total_bonus();
    let resonances = data.get_active_resonances();
    out.push_str(&format!(
        "\n总战力: {} | 总属性: {}\n",
        data.total_combat_power(),
        bonus.format_display()
    ));
    if !resonances.is_empty() {
        out.push_str(&format!("激活共鸣: {}\n", resonances.join(", ")));
    }
    out
}

/// 觉醒血脉 — 觉醒指定血脉
pub fn cmd_bloodline_awaken(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();
    if name.is_empty() {
        let mut out = format!("{}\n格式: 觉醒血脉+血脉名\n可选血脉:\n", prefix);
        for bt in BloodlineType::all() {
            out.push_str(&format!("  {} {} ({})\n", bt.icon(), bt.name(), bt.desc()));
        }
        return out;
    }

    // Find bloodline by name
    let bt = BloodlineType::all().iter().find(|b| b.name() == name).copied();
    let bt = match bt {
        Some(b) => b,
        None => return format!("{}\n❌ 未找到血脉: {}", prefix, name),
    };

    let raw = db.global_get("bloodline", user_id);
    let mut data: BloodlineData = if raw.is_empty() {
        BloodlineData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    data.check_daily_reset(&today);

    let entry = data.get_or_create(bt);
    let next_level = match entry.level.next() {
        Some(l) => l,
        None => return format!("{}\n❌ {}血脉已达到最高觉醒等级: 神觉", prefix, bt.name()),
    };

    // Check requirements
    let required_power = next_level.required_power();
    let required_level = next_level.required_level();
    let required_gold = next_level.required_gold();

    if entry.power_accumulated < required_power {
        return format!(
            "{}\n❌ 血脉之力不足! 需要: {} 当前: {}\n通过击败BOSS/PvP/深渊等方式获取血脉之力。",
            prefix, required_power, entry.power_accumulated
        );
    }

    let user_level: u32 = db.read_basic(user_id, "等级").parse().unwrap_or(0);
    if user_level < required_level {
        return format!(
            "{}\n❌ 等级不足! 需要: {}级 当前: {}级",
            prefix, required_level, user_level
        );
    }

    let user_gold: i64 = db.read_basic(user_id, "金币").parse().unwrap_or(0);
    if user_gold < required_gold as i64 {
        return format!("{}\n❌ 金币不足! 需要: {} 当前: {}", prefix, required_gold, user_gold);
    }

    // Deduct gold
    db.write_basic(user_id, "金币", &(user_gold - required_gold as i64).to_string());

    // Awaken!
    let old_level_name = next_level.prev_display().to_string();
    entry.level = next_level;
    entry.awakened_at = Some(today.clone());
    let total_power = data.total_combat_power();
    db.global_set("bloodline", user_id, &serde_json::to_string(&data).unwrap());

    let bonus = get_bloodline_bonus_per_level(bt, next_level);
    let mut result = format!("{}\n🎉 {}血脉觉醒成功!\n", prefix, bt.name());
    result.push_str(&format!("觉醒等级: {} → {}\n", old_level_name, next_level.name()));
    result.push_str(&format!("属性加成: {}\n", bonus.format_display()));
    result.push_str(&format!("消耗: {}金币\n", required_gold));
    result.push_str(&format!("总战力: {}\n", total_power));

    // Check resonance
    let resonances = data.get_active_resonances();
    if !resonances.is_empty() {
        result.push_str(&format!("✨ 激活共鸣: {}\n", resonances.join(", ")));
    }
    result
}

/// 血脉详情 — 查看指定血脉的详细信息
pub fn cmd_bloodline_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();
    if name.is_empty() {
        return format!("{}\n格式: 血脉详情+血脉名", prefix);
    }

    let bt = BloodlineType::all().iter().find(|b| b.name() == name).copied();
    let bt = match bt {
        Some(b) => b,
        None => return format!("{}\n❌ 未找到血脉: {}", prefix, name),
    };

    let raw = db.global_get("bloodline", user_id);
    let data: BloodlineData = if raw.is_empty() {
        BloodlineData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let entry = data.entries.iter().find(|e| e.bloodline_type == bt);

    let mut out = format!("{}\n═══ {}{} {} ═══\n", prefix, bt.icon(), bt.name(), bt.desc());
    out.push_str(&format!("主属性: {}\n", bt.primary_attr()));

    let level = entry.map(|e| e.level).unwrap_or(AwakeningLevel::Dormant);
    let power = entry.map(|e| e.power_accumulated).unwrap_or(0);

    out.push_str(&format!("\n当前觉醒: {} (血脉之力: {})\n", level.name(), power));
    let bonus = get_bloodline_bonus_per_level(bt, level);
    if !bonus.is_empty() {
        out.push_str(&format!("属性加成: {}\n", bonus.format_display()));
    }

    // Show skills
    let skills = get_bloodline_skills(bt);
    if !skills.is_empty() {
        out.push_str("\n血脉特技:\n");
        for skill in &skills {
            let unlocked = match level {
                AwakeningLevel::Medium => skill.name.contains("中觉") || skill.name.contains("初觉"),
                AwakeningLevel::Strong | AwakeningLevel::Extreme | AwakeningLevel::Divine => true,
                AwakeningLevel::Initial => skill.name.contains("初觉"),
                _ => false,
            };
            let status = if unlocked { "🔓" } else { "🔒" };
            out.push_str(&format!(
                "  {} {} — 触发率:{:.0}% 伤害:{:.1}x\n",
                status, skill.name, skill.trigger_pct, skill.effect_multiplier
            ));
        }
    }

    // Show next level requirements
    if let Some(next) = level.next() {
        out.push_str(&format!("\n下一阶段: {}\n", next.name()));
        out.push_str(&format!(
            "  需要血脉之力: {} (当前: {})\n",
            next.required_power(),
            power
        ));
        out.push_str(&format!("  需要等级: {}\n", next.required_level()));
        out.push_str(&format!("  消耗金币: {}\n", next.required_gold()));
    } else {
        out.push_str("\n🏆 已达到最高觉醒等级!\n");
    }
    out
}

/// 血脉之力 — 查看今日血脉之力获取情况
pub fn cmd_bloodline_power(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("bloodline", user_id);
    let mut data: BloodlineData = if raw.is_empty() {
        BloodlineData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    data.check_daily_reset(&today);

    let mut out = format!("{}\n═══ 血脉之力 ═══\n", prefix);
    out.push_str(&format!("今日获取: {}\n\n", today));

    let sources = [
        PowerSource::BossKill,
        PowerSource::PvpWin,
        PowerSource::AbyssClear,
        PowerSource::ArenaWin,
        PowerSource::GuildWar,
        PowerSource::WorldBoss,
        PowerSource::DailySignIn,
        PowerSource::Quest,
    ];

    for src in &sources {
        let used = data.get_daily_used(*src);
        let cap = src.daily_cap();
        let pct = used.saturating_mul(100).checked_div(cap).unwrap_or(0).min(100);
        let bar = progress_bar(pct as u32, 10);
        out.push_str(&format!(
            "  {} {} {}/{} {}\n",
            src.name(),
            bar,
            used,
            cap,
            if used >= cap { "✅" } else { "" }
        ));
    }

    out.push_str(&format!(
        "\n总血脉之力: {}\n",
        data.entries.iter().map(|e| e.power_accumulated).sum::<u64>()
    ));
    out.push_str("提示: 通过战斗/签到/任务等方式获取血脉之力，用于觉醒血脉。\n");
    out
}

/// 血脉共鸣 — 查看共鸣组状态
pub fn cmd_bloodline_resonance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("bloodline", user_id);
    let data: BloodlineData = if raw.is_empty() {
        BloodlineData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let mut out = format!("{}\n═══ 血脉共鸣 ═══\n", prefix);
    for group in get_resonance_groups() {
        let all_awakened = group.members.iter().all(|bt| {
            data.entries
                .iter()
                .any(|e| e.bloodline_type == *bt && e.level > AwakeningLevel::Dormant)
        });
        let icon = if all_awakened { "✅" } else { "⬜" };
        out.push_str(&format!("\n{} {}:\n", icon, group.name));
        for bt in &group.members {
            let entry = data.entries.iter().find(|e| e.bloodline_type == *bt);
            let status = match entry {
                Some(e) if e.level > AwakeningLevel::Dormant => {
                    format!("  {} {} — {}", bt.icon(), bt.name(), e.level.name())
                }
                _ => format!("  {} {} — 未觉醒", bt.icon(), bt.name()),
            };
            out.push_str(&format!("{}\n", status));
        }
        out.push_str(&format!(
            "共鸣加成: HP+{} 物攻+{} 魔攻+{} 防御+{} 魔抗+{}\n",
            group.bonus_hp, group.bonus_ad, group.bonus_ap, group.bonus_def, group.bonus_mres
        ));
    }
    out
}

/// 血脉排行 — 全服血脉战力排行
pub fn cmd_bloodline_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare("SELECT Key, Value FROM Global WHERE Section='bloodline'") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    let mut rankings: Vec<(String, u64, usize)> = stmt
        .query_map([], |row| {
            let uid: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((uid, val))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|(uid, val)| {
            let data: BloodlineData = serde_json::from_str(&val).ok()?;
            let power = data.total_combat_power();
            let count = data
                .entries
                .iter()
                .filter(|e| e.level > AwakeningLevel::Dormant)
                .count();
            if power > 0 {
                Some((uid, power, count))
            } else {
                None
            }
        })
        .collect();

    rankings.sort_by_key(|x| std::cmp::Reverse(x.1));

    if rankings.is_empty() {
        return format!("{}\n暂无血脉排行数据。", prefix);
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = format!("{}\n═══ 血脉战力排行 ═══\n", prefix);
    for (i, (uid, power, count)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let display_name = if name.is_empty() { uid.clone() } else { name };
        out.push_str(&format!(
            "{}{}. {} — 战力:{} 觉醒:{}条\n",
            medal,
            i + 1,
            display_name,
            power,
            count
        ));
    }
    out
}

/// 血脉帮助
pub fn cmd_bloodline_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n        ═══ 血脉觉醒系统帮助 ═══\n\n        血脉系统让你觉醒远古血脉之力，获得强大属性加成。\n\n        📋 指令列表:\n        • 查看血脉 — 查看所有血脉状态\n        • 觉醒血脉+名称 — 觉醒/升级指定血脉\n        • 血脉详情+名称 — 查看血脉详细信息和技能\n        • 血脉之力 — 查看今日血脉之力获取情况\n        • 血脉共鸣 — 查看共鸣组加成\n        • 血脉排行 — 全服血脉战力排行\n        • 血脉帮助 — 本帮助信息\n\n        🐉 10种血脉: 龙/凤凰/白虎/玄武/青龙/朱雀/麒麟/九尾/饕餮/烛龙\n        ⬆️ 7级觉醒: 未觉醒→初觉→微觉→中觉→强觉→极觉→神觉\n        💪 血脉之力来源: BOSS(15/日)/PvP(10)/深渊(12)/竞技(8)/公会战(20)/世界BOSS(25)/签到(5)/任务(6)\n        🔗 共鸣组: 四象之力/圣兽之血/暗影之力 — 同组3血脉觉醒后额外加成\n",
        prefix
    )
}

/// Helper: progress bar
fn progress_bar(pct: u32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloodline_types_count() {
        assert_eq!(BloodlineType::all().len(), 10);
    }

    #[test]
    fn test_bloodline_type_names_unique() {
        let mut names: Vec<&str> = BloodlineType::all().iter().map(|b| b.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 10);
    }

    #[test]
    fn test_bloodline_type_ids_roundtrip() {
        for bt in BloodlineType::all() {
            let id = bt.to_id();
            let recovered = BloodlineType::from_id(id);
            assert_eq!(recovered, Some(*bt));
        }
    }

    #[test]
    fn test_awakening_levels_count() {
        assert_eq!(AwakeningLevel::all().len(), 7);
    }

    #[test]
    fn test_awakening_level_ordering() {
        for i in 0..AwakeningLevel::all().len() - 1 {
            assert!(AwakeningLevel::all()[i] < AwakeningLevel::all()[i + 1]);
        }
    }

    #[test]
    fn test_awakening_level_required_power_escalates() {
        let levels = AwakeningLevel::all();
        for i in 1..levels.len() {
            assert!(levels[i].required_power() > levels[i - 1].required_power());
        }
    }

    #[test]
    fn test_awakening_level_required_gold_escalates() {
        let levels = AwakeningLevel::all();
        for i in 1..levels.len() {
            assert!(levels[i].required_gold() > levels[i - 1].required_gold());
        }
    }

    #[test]
    fn test_awakening_level_required_level_escalates() {
        let levels = AwakeningLevel::all();
        for i in 1..levels.len() {
            assert!(levels[i].required_level() > levels[i - 1].required_level());
        }
    }

    #[test]
    fn test_awakening_next_chain() {
        let mut level = AwakeningLevel::Dormant;
        let mut count = 0;
        while let Some(next) = level.next() {
            level = next;
            count += 1;
        }
        assert_eq!(count, 6); // Dormant→Initial→...→Divine = 6 steps
        assert_eq!(level, AwakeningLevel::Divine);
        assert_eq!(level.next(), None);
    }

    #[test]
    fn test_awakening_from_u8() {
        for i in 0..=6 {
            let level = AwakeningLevel::from_u8(i);
            assert_eq!(level as u8, i);
        }
        // 7+ should be Divine
        assert_eq!(AwakeningLevel::from_u8(7), AwakeningLevel::Divine);
        assert_eq!(AwakeningLevel::from_u8(255), AwakeningLevel::Divine);
    }

    #[test]
    fn test_bloodline_skills_exist() {
        for bt in BloodlineType::all() {
            let skills = get_bloodline_skills(*bt);
            assert_eq!(skills.len(), 2, "{:?} should have 2 skills", bt);
            // Each skill should have valid trigger pct
            for skill in &skills {
                assert!(skill.trigger_pct > 0.0 && skill.trigger_pct <= 100.0);
                assert!(skill.effect_multiplier > 0.0);
            }
        }
    }

    #[test]
    fn test_bloodline_bonus_per_level_dormant_is_zero() {
        for bt in BloodlineType::all() {
            let bonus = get_bloodline_bonus_per_level(*bt, AwakeningLevel::Dormant);
            assert!(bonus.is_empty(), "{:?} dormant bonus should be zero", bt);
        }
    }

    #[test]
    fn test_bloodline_bonus_escalates() {
        for bt in BloodlineType::all() {
            let bonus_initial = get_bloodline_bonus_per_level(*bt, AwakeningLevel::Initial);
            let bonus_divine = get_bloodline_bonus_per_level(*bt, AwakeningLevel::Divine);
            let initial_sum =
                bonus_initial.hp + bonus_initial.ad + bonus_initial.ap + bonus_initial.def + bonus_initial.mres;
            let divine_sum = bonus_divine.hp + bonus_divine.ad + bonus_divine.ap + bonus_divine.def + bonus_divine.mres;
            assert!(
                divine_sum > initial_sum,
                "{:?} divine should be stronger than initial",
                bt
            );
        }
    }

    #[test]
    fn test_bloodline_bonus_merge() {
        let mut a = BloodlineBonus {
            hp: 100,
            ad: 50,
            ap: 30,
            def: 20,
            mres: 10,
            crit: 5.0,
            dodge: 3.0,
            lifesteal: 2.0,
            penetrate: 1.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        };
        let b = BloodlineBonus {
            hp: 200,
            ad: 100,
            ap: 60,
            def: 40,
            mres: 20,
            crit: 10.0,
            dodge: 6.0,
            lifesteal: 4.0,
            penetrate: 2.0,
            exp_bonus: 5.0,
            gold_bonus: 5.0,
        };
        a.merge(&b);
        assert_eq!(a.hp, 300);
        assert_eq!(a.ad, 150);
        assert_eq!(a.ap, 90);
        assert_eq!(a.crit, 15.0);
        assert_eq!(a.exp_bonus, 5.0);
    }

    #[test]
    fn test_bloodline_bonus_format_display() {
        let bonus = BloodlineBonus {
            hp: 100,
            ad: 50,
            ap: 0,
            def: 0,
            mres: 0,
            crit: 5.0,
            dodge: 0.0,
            lifesteal: 0.0,
            penetrate: 0.0,
            exp_bonus: 0.0,
            gold_bonus: 0.0,
        };
        let display = bonus.format_display();
        assert!(display.contains("HP+100"));
        assert!(display.contains("物攻+50"));
        assert!(display.contains("暴击+5.0%"));
        assert!(!display.contains("魔攻"));
    }

    #[test]
    fn test_bloodline_entry_new() {
        let entry = BloodlineEntry::new(BloodlineType::Dragon);
        assert_eq!(entry.bloodline_type, BloodlineType::Dragon);
        assert_eq!(entry.level, AwakeningLevel::Dormant);
        assert_eq!(entry.power_accumulated, 0);
        assert!(entry.awakened_at.is_none());
    }

    #[test]
    fn test_bloodline_entry_combat_power() {
        let mut entry = BloodlineEntry::new(BloodlineType::Dragon);
        let dormant_power = entry.combat_power();
        entry.level = AwakeningLevel::Divine;
        let divine_power = entry.combat_power();
        assert!(divine_power > dormant_power);
    }

    #[test]
    fn test_bloodline_data_get_or_create() {
        let mut data = BloodlineData::default();
        assert!(!data.has_bloodline(BloodlineType::Dragon));
        data.get_or_create(BloodlineType::Dragon);
        assert!(data.has_bloodline(BloodlineType::Dragon));
        assert_eq!(data.entries.len(), 1);
        // Second call should not create duplicate
        data.get_or_create(BloodlineType::Dragon);
        assert_eq!(data.entries.len(), 1);
    }

    #[test]
    fn test_bloodline_data_daily_reset() {
        let mut data = BloodlineData::default();
        data.record_power_gain(PowerSource::BossKill, "2026-01-01");
        assert_eq!(data.get_daily_used(PowerSource::BossKill), 15);
        data.record_power_gain(PowerSource::BossKill, "2026-01-01");
        assert_eq!(data.get_daily_used(PowerSource::BossKill), 30);
        // New day resets
        data.record_power_gain(PowerSource::BossKill, "2026-01-02");
        assert_eq!(data.get_daily_used(PowerSource::BossKill), 15);
    }

    #[test]
    fn test_bloodline_data_daily_cap() {
        let mut data = BloodlineData::default();
        let today = "2026-01-01";
        let cap = PowerSource::DailySignIn.daily_cap();
        let amount = PowerSource::DailySignIn.power_amount();
        let iterations = (cap / amount) + 5;
        let mut total = 0u64;
        for _ in 0..iterations {
            total += data.record_power_gain(PowerSource::DailySignIn, today);
        }
        assert_eq!(total, cap);
    }

    #[test]
    fn test_power_sources_all_valid() {
        let sources = [
            PowerSource::BossKill,
            PowerSource::PvpWin,
            PowerSource::AbyssClear,
            PowerSource::ArenaWin,
            PowerSource::GuildWar,
            PowerSource::WorldBoss,
            PowerSource::DailySignIn,
            PowerSource::Quest,
        ];
        for src in &sources {
            assert!(src.power_amount() > 0);
            assert!(src.daily_cap() >= src.power_amount());
            assert!(!src.name().is_empty());
        }
    }

    #[test]
    fn test_resonance_groups_count() {
        let groups = get_resonance_groups();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_resonance_groups_unique_members() {
        for group in get_resonance_groups() {
            assert_ne!(group.members[0], group.members[1]);
            assert_ne!(group.members[1], group.members[2]);
            assert_ne!(group.members[0], group.members[2]);
        }
    }

    #[test]
    fn test_total_bonus_with_resonance() {
        let mut data = BloodlineData::default();
        // Awaken all members of "四象之力" group
        for bt in &[
            BloodlineType::AzureDragon,
            BloodlineType::WhiteTiger,
            BloodlineType::BlackTortoise,
        ] {
            let entry = data.get_or_create(*bt);
            entry.level = AwakeningLevel::Initial;
        }
        let bonus = data.total_bonus();
        // Should include resonance bonus
        assert!(bonus.hp > 0);
        assert!(data.get_active_resonances().contains(&"四象之力"));
    }

    #[test]
    fn test_total_combat_power_empty() {
        let data = BloodlineData::default();
        assert_eq!(data.total_combat_power(), 0);
    }

    #[test]
    fn test_total_combat_power_multiple() {
        let mut data = BloodlineData::default();
        let entry1 = data.get_or_create(BloodlineType::Dragon);
        entry1.level = AwakeningLevel::Strong;
        let entry2 = data.get_or_create(BloodlineType::Phoenix);
        entry2.level = AwakeningLevel::Medium;
        assert!(data.total_combat_power() > 0);
    }

    #[test]
    fn test_get_bloodline_bonus_api() {
        let mut data = BloodlineData::default();
        let entry = data.get_or_create(BloodlineType::Dragon);
        entry.level = AwakeningLevel::Divine;
        let bonus = get_bloodline_bonus(&data);
        assert!(bonus.ad > 0);
    }

    #[test]
    fn test_record_bloodline_power_api() {
        let mut data = BloodlineData::default();
        let gain = record_bloodline_power(&mut data, PowerSource::BossKill, "2026-01-01");
        assert_eq!(gain, 15);
    }
}
