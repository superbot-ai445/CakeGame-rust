//! CakeGame 小游戏系统
//!
//! 经典 QQ 机器人小游戏：
//! - 猜拳 (石头/剪刀/布): 下注金币对战
//! - 掷骰子: 下注猜大小/点数
//! - 猜数字: 猜 1-100，提示高/低
//!
//! 数据存储: Global 表 (minigame section)
//! 来源: 新增功能，增强休闲玩法

use crate::core::{CURRENCY_GOLD, OP_ADD, OP_SUB};
use crate::db::Database;
use std::time::{SystemTime, UNIX_EPOCH};

// ==================== 猜拳 ====================

/// 猜拳选项
#[derive(Debug, Clone, Copy, PartialEq)]
enum RpsChoice {
    Rock,     // 石头
    Scissors, // 剪刀
    Paper,    // 布
}

impl RpsChoice {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "石头" | "拳" | "石" | "rock" | "r" => Some(Self::Rock),
            "剪刀" | "剪" | "scissors" | "s" => Some(Self::Scissors),
            "布" | "bao" | "paper" | "p" => Some(Self::Paper),
            _ => None,
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            Self::Rock => "\u{270a}",
            Self::Scissors => "\u{270c}\u{fe0f}",
            Self::Paper => "\u{270b}",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Rock => "石头",
            Self::Scissors => "剪刀",
            Self::Paper => "布",
        }
    }

    /// 胜负判定: 1=赢, 0=平, -1=输
    fn vs(self, other: Self) -> i32 {
        match (self, other) {
            (a, b) if a == b => 0,
            (Self::Rock, Self::Scissors) | (Self::Scissors, Self::Paper) | (Self::Paper, Self::Rock) => 1,
            _ => -1,
        }
    }
}

/// 生成伪随机数 (基于时间+用户ID的确定性随机)
fn pseudo_random(seed: &str, max: u32) -> u32 {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let hash: u64 = seed
        .bytes()
        .fold(5381u64, |h, b| h.wrapping_mul(33).wrapping_add(b as u64));
    let mixed = hash.wrapping_add(ts as u64);
    ((mixed ^ (mixed >> 33)).wrapping_mul(0xff51afd7ed558ccd) >> 33) as u32 % max
}

/// 更新用户小游戏统计
fn update_user_stat(db: &Database, user_id: &str, key: &str, delta: i64) {
    let full_key = format!("minigame_{}_{}", user_id, key);
    let current: i64 = db.global_get("minigame", &full_key).parse().unwrap_or(0);
    db.global_set("minigame", &full_key, &(current + delta).to_string());
}

/// 读取用户小游戏统计
fn get_user_stat(db: &Database, user_id: &str, key: &str) -> i64 {
    let full_key = format!("minigame_{}_{}", user_id, key);
    db.global_get("minigame", &full_key).parse().unwrap_or(0)
}

/// 增减用户金币
fn adjust_gold(db: &Database, user_id: &str, amount: i64) -> i64 {
    let op = if amount >= 0 { OP_ADD } else { OP_SUB };
    db.modify_currency(user_id, CURRENCY_GOLD, op, amount.abs())
}

/// 查询用户金币
fn get_gold(db: &Database, user_id: &str) -> i64 {
    db.read_currency(user_id, CURRENCY_GOLD)
}

// ==================== 猜拳命令 ====================

pub fn cmd_rps(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 格式: 猜拳+石头+100 (猜拳+选项+下注金额)
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return "\u{1f3ae} 猜拳游戏\n\n\
            用法: 猜拳+石头/剪刀/布+下注金额\n\
            示例: 猜拳+石头+100\n\n\
            \u{270a} 石头 克 \u{270c}\u{fe0f} 剪刀\n\
            \u{270c}\u{fe0f} 剪刀 克 \u{270b} 布\n\
            \u{270b} 布 克 \u{270a} 石头\n\n\
            赢: 2倍下注 | 平: 退还 | 输: 损失下注\n\
            默认下注 10 金币"
            .to_string();
    }

    let player = match RpsChoice::from_str(parts[0]) {
        Some(c) => c,
        None => return "❌ 无效选项！请选择: 石头 / 剪刀 / 布".to_string(),
    };

    let bet: i64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10).max(1);

    let gold = get_gold(db, user_id);
    if gold < bet {
        return format!("❌ 金币不足！你有 {} 金币，需要 {} 金币", gold, bet);
    }

    // 系统出拳
    let seed = format!("rps_{}_{}", user_id, bet);
    let npc_idx = pseudo_random(&seed, 3);
    let npc = match npc_idx {
        0 => RpsChoice::Rock,
        1 => RpsChoice::Scissors,
        _ => RpsChoice::Paper,
    };

    let result = player.vs(npc);
    let mut msg = format!(
        "\u{1f3ae} 猜拳结果\n\n{}  {} vs {}  {}\n",
        player.emoji(),
        player.name(),
        npc.emoji(),
        npc.name()
    );

    match result {
        1 => {
            let reward = bet * 2;
            adjust_gold(db, user_id, reward);
            msg.push_str(&format!(
                "\u{1f389} 你赢了！获得 {} 金币\n下注: {} \u{2192} 获得: {}",
                reward, bet, reward
            ));
        }
        0 => {
            msg.push_str(&format!("\u{1f91d} 平局！退还 {} 金币", bet));
        }
        _ => {
            adjust_gold(db, user_id, -bet);
            msg.push_str(&format!("\u{1f622} 你输了！损失 {} 金币", bet));
        }
    }

    update_user_stat(db, user_id, "rps_total", 1);
    msg
}

// ==================== 掷骰子命令 ====================

pub fn cmd_dice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 格式: 掷骰子+大+100  (下注大小)
    // 或: 掷骰子+点数4+100 (猜具体点数)
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return "\u{1f3b2} 掷骰子游戏\n\n\
            用法:\n\
            掷骰子+大+下注 (点数 4-6 为大)\n\
            掷骰子+小+下注 (点数 1-3 为小)\n\
            掷骰子+点数N+下注 (猜具体点数 1-6)\n\n\
            大小赢: 2倍 | 猜中点数: 6倍\n\
            默认下注 10 金币"
            .to_string();
    }

    // 解析下注金额（最后一个数字部分）
    let (choice, bet) = if parts.len() >= 2 {
        if let Ok(b) = parts.last().unwrap().parse::<i64>() {
            (&parts[0..parts.len() - 1].join("+"), b.max(1))
        } else {
            (&parts[0..1].join("+"), 10i64)
        }
    } else {
        (&parts[0..1].join("+"), 10i64)
    };

    let gold = get_gold(db, user_id);
    if gold < bet {
        return format!("❌ 金币不足！你有 {} 金币，需要 {} 金币", gold, bet);
    }

    // 掷骰子
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seed = format!("dice_{}_{}", user_id, ts);
    let dice = pseudo_random(&seed, 6) + 1; // 1-6

    let dice_faces = ["\u{2680}", "\u{2681}", "\u{2682}", "\u{2683}", "\u{2684}", "\u{2685}"];
    let face = dice_faces.get((dice - 1) as usize).unwrap_or(&"\u{2680}");

    let mut msg = format!("\u{1f3b2} 掷骰子结果: {} [{}点]\n\n", face, dice);

    match choice.as_str() {
        "大" | "big" => {
            if dice >= 4 {
                let reward = bet * 2;
                adjust_gold(db, user_id, reward);
                msg.push_str(&format!("\u{1f389} 你赢了！{}点为「大」\n获得 {} 金币", dice, reward));
            } else {
                adjust_gold(db, user_id, -bet);
                msg.push_str(&format!("\u{1f622} 你输了！{}点为「小」\n损失 {} 金币", dice, bet));
            }
        }
        "小" | "small" => {
            if dice <= 3 {
                let reward = bet * 2;
                adjust_gold(db, user_id, reward);
                msg.push_str(&format!("\u{1f389} 你赢了！{}点为「小」\n获得 {} 金币", dice, reward));
            } else {
                adjust_gold(db, user_id, -bet);
                msg.push_str(&format!("\u{1f622} 你输了！{}点为「大」\n损失 {} 金币", dice, bet));
            }
        }
        other => {
            // 猜点数
            let guess_str = other.trim_start_matches("点数");
            let guess: u32 = match guess_str.parse() {
                Ok(n) if (1..=6).contains(&n) => n,
                _ => return "❌ 无效选项！请选择: 大 / 小 / 点数1~6".to_string(),
            };
            if dice == guess {
                let reward = bet * 6;
                adjust_gold(db, user_id, reward);
                msg.push_str(&format!(
                    "\u{1f389}\u{1f389} 猜中了！{}点！\n获得 {} 金币 (6倍)",
                    dice, reward
                ));
            } else {
                adjust_gold(db, user_id, -bet);
                msg.push_str(&format!(
                    "\u{1f622} 没猜中！是{}点，你猜了{}点\n损失 {} 金币",
                    dice, guess, bet
                ));
            }
        }
    }

    update_user_stat(db, user_id, "dice_total", 1);
    msg
}

// ==================== 猜数字命令 ====================

pub fn cmd_guess(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 格式: 猜数字+50+100 (猜数字+猜测值+下注)
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return "\u{1f522} 猜数字游戏\n\n\
            系统随机生成 1~100 的数字\n\
            用法: 猜数字+你的猜测+下注金额\n\
            示例: 猜数字+50+100\n\n\
            \u{1f3af} 猜中: 10倍下注\n\
            \u{1f525} 差1~2: 3倍下注\n\
            \u{1f321}\u{fe0f} 差3~5: 退还下注\n\
            \u{2744}\u{fe0f} 差6+: 损失下注\n\
            默认下注 10 金币"
            .to_string();
    }

    let guess: u32 = match parts[0].parse() {
        Ok(n) if (1..=100).contains(&n) => n,
        _ => return "❌ 请输入 1~100 之间的数字".to_string(),
    };

    let bet: i64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10).max(1);

    let gold = get_gold(db, user_id);
    if gold < bet {
        return format!("❌ 金币不足！你有 {} 金币，需要 {} 金币", gold, bet);
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let seed = format!("guess_{}_{}", user_id, ts);
    let answer = pseudo_random(&seed, 100) + 1; // 1-100

    let diff = guess.abs_diff(answer);

    let mut msg = format!(
        "\u{1f522} 猜数字结果: 答案是 [{}]\n你的猜测: [{}]\n差距: {}\n\n",
        answer, guess, diff
    );

    match diff {
        0 => {
            let reward = bet * 10;
            adjust_gold(db, user_id, reward);
            msg.push_str(&format!(
                "\u{1f3af}\u{1f3af}\u{1f3af} 完美猜中！获得 {} 金币 (10倍)",
                reward
            ));
            update_user_stat(db, user_id, "guess_perfect", 1);
        }
        1..=2 => {
            let reward = bet * 3;
            adjust_gold(db, user_id, reward);
            msg.push_str(&format!("\u{1f525} 非常接近！获得 {} 金币 (3倍)", reward));
        }
        3..=5 => {
            msg.push_str(&format!("\u{1f321}\u{fe0f} 还算接近，退还 {} 金币", bet));
        }
        _ => {
            adjust_gold(db, user_id, -bet);
            msg.push_str(&format!("\u{2744}\u{fe0f} 差太远了！损失 {} 金币", bet));
        }
    }

    update_user_stat(db, user_id, "guess_total", 1);
    msg
}

// ==================== 小游戏统计命令 ====================

pub fn cmd_minigame_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let rps_total = get_user_stat(db, user_id, "rps_total");
    let dice_total = get_user_stat(db, user_id, "dice_total");
    let guess_total = get_user_stat(db, user_id, "guess_total");
    let guess_perfect = get_user_stat(db, user_id, "guess_perfect");
    let total = rps_total + dice_total + guess_total;

    format!(
        "\u{1f3ae} 小游戏统计\n\
         \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n\
         \u{1f4ca} 总游戏次数: {}\n\n\
         \u{270a} 猜拳: {} 次\n\
         \u{1f3b2} 骰子: {} 次\n\
         \u{1f522} 猜数字: {} 次\n\
         \u{1f3af} 完美猜中: {} 次\n\n\
         \u{1f4a1} 游戏列表:\n\
         \u{2022} 猜拳+石头/剪刀/布+下注\n\
         \u{2022} 掷骰子+大/小/点数N+下注\n\
         \u{2022} 猜数字+数字+下注\n\n\
         \u{26a0}\u{fe0f} 适度游戏，理性消费",
        total, rps_total, dice_total, guess_total, guess_perfect
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rps_choice_parsing() {
        assert_eq!(RpsChoice::from_str("石头"), Some(RpsChoice::Rock));
        assert_eq!(RpsChoice::from_str("剪刀"), Some(RpsChoice::Scissors));
        assert_eq!(RpsChoice::from_str("布"), Some(RpsChoice::Paper));
        assert_eq!(RpsChoice::from_str("rock"), Some(RpsChoice::Rock));
        assert_eq!(RpsChoice::from_str("r"), Some(RpsChoice::Rock));
        assert_eq!(RpsChoice::from_str("s"), Some(RpsChoice::Scissors));
        assert_eq!(RpsChoice::from_str("p"), Some(RpsChoice::Paper));
        assert_eq!(RpsChoice::from_str("无效"), None);
    }

    #[test]
    fn test_rps_outcome() {
        assert_eq!(RpsChoice::Rock.vs(RpsChoice::Scissors), 1);
        assert_eq!(RpsChoice::Scissors.vs(RpsChoice::Paper), 1);
        assert_eq!(RpsChoice::Paper.vs(RpsChoice::Rock), 1);
        assert_eq!(RpsChoice::Rock.vs(RpsChoice::Rock), 0);
        assert_eq!(RpsChoice::Scissors.vs(RpsChoice::Scissors), 0);
        assert_eq!(RpsChoice::Paper.vs(RpsChoice::Paper), 0);
        assert_eq!(RpsChoice::Rock.vs(RpsChoice::Paper), -1);
        assert_eq!(RpsChoice::Scissors.vs(RpsChoice::Rock), -1);
        assert_eq!(RpsChoice::Paper.vs(RpsChoice::Scissors), -1);
    }

    #[test]
    fn test_pseudo_random_range() {
        for i in 0..100 {
            let seed = format!("test_{}", i);
            let r = pseudo_random(&seed, 6);
            assert!(r < 6, "random {} should be < 6", r);
        }
    }

    #[test]
    fn test_dice_range() {
        for i in 0..100 {
            let seed = format!("dice_test_{}", i);
            let dice = pseudo_random(&seed, 6) + 1;
            assert!((1..=6).contains(&dice), "dice {} should be 1-6", dice);
        }
    }

    #[test]
    fn test_guess_range() {
        for i in 0..100 {
            let seed = format!("guess_test_{}", i);
            let num = pseudo_random(&seed, 100) + 1;
            assert!((1..=100).contains(&num), "guess {} should be 1-100", num);
        }
    }

    #[test]
    fn test_rps_all_outcomes() {
        let choices = [RpsChoice::Rock, RpsChoice::Scissors, RpsChoice::Paper];
        for &a in &choices {
            for &b in &choices {
                let result = a.vs(b);
                assert!(result >= -1 && result <= 1);
                if a == b {
                    assert_eq!(result, 0);
                }
            }
        }
    }

    #[test]
    fn test_rps_symmetry() {
        // 如果 A 赢 B，则 B 必须输给 A
        let pairs = [
            (RpsChoice::Rock, RpsChoice::Scissors),
            (RpsChoice::Scissors, RpsChoice::Paper),
            (RpsChoice::Paper, RpsChoice::Rock),
        ];
        for (a, b) in pairs {
            assert_eq!(a.vs(b), 1, "{} should beat {}", a.name(), b.name());
            assert_eq!(b.vs(a), -1, "{} should lose to {}", b.name(), a.name());
        }
    }
}
