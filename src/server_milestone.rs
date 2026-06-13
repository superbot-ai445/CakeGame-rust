/// CakeGame 全服里程碑成就系统
///
/// 追踪服务器级别的里程碑目标（注册人数、总击杀数、总金币产出等），
/// 当全服达成里程碑时，所有玩家可领取里程碑奖励。
///
/// 功能:
/// - 里程碑: 查看所有里程碑及当前进度
/// - 里程碑进度: 查看具体里程碑的详细进度
/// - 领取里程碑奖励: 领取已达成里程碑的奖励
/// - 里程碑排行: 查看里程碑贡献排名
///
/// 数据存储: Global 表 SECTION='server_milestone'
use crate::core::*;
use crate::db::Database;
use crate::user;

const SECTION: &str = "server_milestone";

/// 里程碑定义
struct MilestoneDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    track_type: &'static str,
    target: i64,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
    reward_item_count: i32,
    emoji: &'static str,
}

const MILESTONES: &[MilestoneDef] = &[
    MilestoneDef {
        id: "ms_reg_100",
        name: "百人集结",
        desc: "全服注册玩家达到100人",
        track_type: "player_count",
        target: 100,
        reward_gold: 5000,
        reward_diamond: 50,
        reward_item: "生命药水",
        reward_item_count: 10,
        emoji: "👥",
    },
    MilestoneDef {
        id: "ms_reg_500",
        name: "群雄汇聚",
        desc: "全服注册玩家达到500人",
        track_type: "player_count",
        target: 500,
        reward_gold: 15000,
        reward_diamond: 150,
        reward_item: "高级生命药水",
        reward_item_count: 5,
        emoji: "🏟️",
    },
    MilestoneDef {
        id: "ms_reg_1000",
        name: "千人盛世",
        desc: "全服注册玩家达到1000人",
        track_type: "player_count",
        target: 1000,
        reward_gold: 50000,
        reward_diamond: 500,
        reward_item: "复活卷轴",
        reward_item_count: 3,
        emoji: "🎆",
    },
    MilestoneDef {
        id: "ms_kill_10000",
        name: "万怪斩首",
        desc: "全服怪物击杀总数达到10,000",
        track_type: "total_kills",
        target: 10_000,
        reward_gold: 10000,
        reward_diamond: 100,
        reward_item: "强化石",
        reward_item_count: 5,
        emoji: "⚔️",
    },
    MilestoneDef {
        id: "ms_kill_100000",
        name: "十万伏尸",
        desc: "全服怪物击杀总数达到100,000",
        track_type: "total_kills",
        target: 100_000,
        reward_gold: 30000,
        reward_diamond: 300,
        reward_item: "高级强化石",
        reward_item_count: 3,
        emoji: "💀",
    },
    MilestoneDef {
        id: "ms_kill_1000000",
        name: "百万屠戮",
        desc: "全服怪物击杀总数达到1,000,000",
        track_type: "total_kills",
        target: 1_000_000,
        reward_gold: 100000,
        reward_diamond: 1000,
        reward_item: "凤凰之羽",
        reward_item_count: 1,
        emoji: "🔥",
    },
    MilestoneDef {
        id: "ms_gold_1000000",
        name: "百万富翁",
        desc: "全服金币总产出达到1,000,000",
        track_type: "total_gold",
        target: 1_000_000,
        reward_gold: 20000,
        reward_diamond: 200,
        reward_item: "金币宝箱",
        reward_item_count: 3,
        emoji: "💰",
    },
    MilestoneDef {
        id: "ms_gold_10000000",
        name: "千万金币",
        desc: "全服金币总产出达到10,000,000",
        track_type: "total_gold",
        target: 10_000_000,
        reward_gold: 50000,
        reward_diamond: 500,
        reward_item: "钻石宝箱",
        reward_item_count: 2,
        emoji: "💎",
    },
    MilestoneDef {
        id: "ms_diamond_100000",
        name: "十万钻石",
        desc: "全服钻石总产出达到100,000",
        track_type: "total_diamond",
        target: 100_000,
        reward_gold: 30000,
        reward_diamond: 300,
        reward_item: "强化石",
        reward_item_count: 10,
        emoji: "💍",
    },
    MilestoneDef {
        id: "ms_level_500",
        name: "等级总和500",
        desc: "全服玩家等级总和达到500",
        track_type: "total_levels",
        target: 500,
        reward_gold: 8000,
        reward_diamond: 80,
        reward_item: "经验药水",
        reward_item_count: 5,
        emoji: "📈",
    },
    MilestoneDef {
        id: "ms_level_5000",
        name: "等级总和5000",
        desc: "全服玩家等级总和达到5000",
        track_type: "total_levels",
        target: 5000,
        reward_gold: 30000,
        reward_diamond: 300,
        reward_item: "高级经验药水",
        reward_item_count: 3,
        emoji: "🚀",
    },
    MilestoneDef {
        id: "ms_guild_10",
        name: "十大公会",
        desc: "全服公会数量达到10个",
        track_type: "guild_count",
        target: 10,
        reward_gold: 10000,
        reward_diamond: 100,
        reward_item: "生命药水",
        reward_item_count: 20,
        emoji: "🏰",
    },
    MilestoneDef {
        id: "ms_guild_50",
        name: "百家争鸣",
        desc: "全服公会数量达到50个",
        track_type: "guild_count",
        target: 50,
        reward_gold: 50000,
        reward_diamond: 500,
        reward_item: "高级生命药水",
        reward_item_count: 10,
        emoji: "🏛️",
    },
    MilestoneDef {
        id: "ms_trade_100",
        name: "交易繁荣",
        desc: "全服交易总次数达到100",
        track_type: "trade_count",
        target: 100,
        reward_gold: 15000,
        reward_diamond: 150,
        reward_item: "金币宝箱",
        reward_item_count: 2,
        emoji: "🤝",
    },
    MilestoneDef {
        id: "ms_trade_1000",
        name: "贸易帝国",
        desc: "全服交易总次数达到1,000",
        track_type: "trade_count",
        target: 1000,
        reward_gold: 50000,
        reward_diamond: 500,
        reward_item: "钻石宝箱",
        reward_item_count: 3,
        emoji: "🏪",
    },
];

/// 从 Global 表读取全服进度
fn load_progress(db: &Database) -> serde_json::Value {
    let raw = db.global_get(SECTION, "progress");
    if raw.is_empty() {
        default_progress()
    } else {
        serde_json::from_str(&raw).unwrap_or_else(|_| default_progress())
    }
}

fn default_progress() -> serde_json::Value {
    serde_json::json!({
        "player_count": 0, "total_kills": 0, "total_gold": 0,
        "total_diamond": 0, "total_levels": 0, "guild_count": 0, "trade_count": 0,
    })
}

fn save_progress(db: &Database, progress: &serde_json::Value) {
    db.global_set(
        SECTION,
        "progress",
        &serde_json::to_string(progress).unwrap_or_default(),
    );
}

fn load_claimed(db: &Database, user_id: &str) -> Vec<String> {
    let key = format!("rewards_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    }
}

fn save_claimed(db: &Database, user_id: &str, claimed: &[String]) {
    let key = format!("rewards_{}", user_id);
    db.global_set(SECTION, &key, &serde_json::to_string(claimed).unwrap_or_default());
}

fn get_track(progress: &serde_json::Value, tt: &str) -> i64 {
    progress.get(tt).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn pct(cur: i64, target: i64) -> f64 {
    if target <= 0 {
        100.0
    } else {
        ((cur as f64 / target as f64) * 100.0).min(100.0)
    }
}

fn bar(cur: i64, target: i64, w: usize) -> String {
    let filled = ((pct(cur, target) / 100.0) * w as f64) as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(w - filled))
}

fn fmt_num(n: i64) -> String {
    let s = n.to_string();
    let mut r = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            r.push(',');
        }
        r.push(*c);
    }
    r
}

/// 刷新全服实时统计
pub fn refresh_server_stats(db: &Database) {
    let conn = db.lock_conn();
    let mut progress = load_progress(db);
    if let Ok(count) = conn.query_row("SELECT COUNT(*) FROM Basic_User", [], |r| r.get::<_, i64>(0)) {
        progress["player_count"] = serde_json::json!(count);
    }
    if let Ok(sum) = conn.query_row(
        "SELECT COALESCE(SUM(CAST(Level AS INTEGER)), 0) FROM Basic_User",
        [],
        |r| r.get::<_, i64>(0),
    ) {
        progress["total_levels"] = serde_json::json!(sum);
    }
    if let Ok(mut stmt) = conn.prepare("SELECT COUNT(DISTINCT ID) FROM Global WHERE Section = 'guild_data'") {
        if let Ok(count) = stmt.query_row([], |r| r.get::<_, i64>(0)) {
            progress["guild_count"] = serde_json::json!(count);
        }
    }
    save_progress(db, &progress);
}

/// 查看全服里程碑
pub fn cmd_view_milestones(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    refresh_server_stats(db);
    let progress = load_progress(db);
    let claimed = load_claimed(db, user_id);

    let mut out = format!("{}\n🎯 ═══ 全服里程碑 ═══\n", prefix);
    out.push_str("全服玩家共同努力，达成里程碑解锁丰厚奖励！\n\n");

    let mut achieved = 0usize;
    let mut pending = 0usize;

    for ms in MILESTONES {
        let cur = get_track(&progress, ms.track_type);
        let p = pct(cur, ms.target);
        let done = cur >= ms.target;
        let got = claimed.contains(&ms.id.to_string());
        let status = if got {
            "✅ 已领取"
        } else if done {
            "🎁 可领取"
        } else {
            "⏳ 进行中"
        };
        if done {
            achieved += 1;
        } else {
            pending += 1;
        }

        out.push_str(&format!(
            "{} {} [{}]\n   进度: {}/{} ({:.1}%)\n   {} {}\n   奖励: {}金 + {}💎",
            ms.emoji,
            ms.name,
            status,
            fmt_num(cur),
            fmt_num(ms.target),
            p,
            bar(cur, ms.target, 16),
            ms.desc,
            fmt_num(ms.reward_gold),
            ms.reward_diamond,
        ));
        if !ms.reward_item.is_empty() {
            out.push_str(&format!(" + {}×{}", ms.reward_item, ms.reward_item_count));
        }
        out.push('\n');
        if done && !got {
            out.push_str("   💡 输入 '领取里程碑+里程碑名' 领取奖励\n");
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "📊 达成: {}/{} | 进行中: {}\n",
        achieved,
        MILESTONES.len(),
        pending
    ));
    out.push_str("💡 输入 '里程碑进度' 查看详细进度\n");
    out.push_str("💡 输入 '里程碑排行' 查看贡献排名\n");
    out
}

/// 里程碑详细进度
pub fn cmd_milestone_progress(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let progress = load_progress(db);

    if args.is_empty() {
        let mut out = format!("{}\n📊 ═══ 里程碑进度总览 ═══\n\n", prefix);
        let types = [
            ("player_count", "👥 注册玩家"),
            ("total_kills", "⚔️ 总击杀数"),
            ("total_gold", "💰 总金币产出"),
            ("total_diamond", "💎 总钻石产出"),
            ("total_levels", "📈 等级总和"),
            ("guild_count", "🏰 公会数量"),
            ("trade_count", "🤝 交易次数"),
        ];
        for (key, label) in &types {
            let val = get_track(&progress, key);
            let related: Vec<_> = MILESTONES.iter().filter(|m| m.track_type == *key).collect();
            let ach = related.iter().filter(|m| val >= m.target).count();
            out.push_str(&format!(
                "{}: {} (达成 {}/{})\n",
                label,
                fmt_num(val),
                ach,
                related.len()
            ));
        }
        return out;
    }

    let tt = match args {
        "玩家" | "注册" | "人数" => "player_count",
        "击杀" | "杀怪" | "怪物" => "total_kills",
        "金币" | "金" => "total_gold",
        "钻石" | "钻" => "total_diamond",
        "等级" => "total_levels",
        "公会" => "guild_count",
        "交易" => "trade_count",
        _ => {
            return format!(
                "{}\n⚠️ 未知追踪类型: {}\n可选: 玩家/击杀/金币/钻石/等级/公会/交易",
                prefix, args
            )
        }
    };
    let label = match tt {
        "player_count" => "注册玩家",
        "total_kills" => "总击杀数",
        "total_gold" => "总金币产出",
        "total_diamond" => "总钻石产出",
        "total_levels" => "等级总和",
        "guild_count" => "公会数量",
        "trade_count" => "交易次数",
        _ => "未知",
    };
    let cur = get_track(&progress, tt);
    let mut out = format!(
        "{}\n📊 ═══ {} 里程碑详情 ═══\n\n当前数值: {}\n\n",
        prefix,
        label,
        fmt_num(cur)
    );
    let related: Vec<_> = MILESTONES.iter().filter(|m| m.track_type == tt).collect();
    for ms in &related {
        let p = pct(cur, ms.target);
        let st = if cur >= ms.target { "✅" } else { "⏳" };
        out.push_str(&format!(
            "{} {} {} → {}/{} ({:.1}%)\n   {}\n",
            st,
            ms.emoji,
            ms.name,
            fmt_num(cur),
            fmt_num(ms.target),
            p,
            bar(cur, ms.target, 20)
        ));
    }
    if let Some(next) = related.iter().find(|m| cur < m.target) {
        out.push_str(&format!(
            "\n🎯 下一个目标: {} (还需 {})\n",
            next.name,
            fmt_num(next.target - cur)
        ));
    } else {
        out.push_str("\n🏆 恭喜！该类别所有里程碑已达成！\n");
    }
    out
}

/// 领取里程碑奖励
pub fn cmd_claim_milestone(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if args.is_empty() {
        let progress = load_progress(db);
        let claimed = load_claimed(db, user_id);
        let mut out = format!("{}\n🎁 可领取的里程碑奖励:\n\n", prefix);
        let mut has = false;
        for ms in MILESTONES {
            let cur = get_track(&progress, ms.track_type);
            if cur >= ms.target && !claimed.contains(&ms.id.to_string()) {
                out.push_str(&format!(
                    "  {} {} ({}金 + {}💎)\n",
                    ms.emoji,
                    ms.name,
                    fmt_num(ms.reward_gold),
                    ms.reward_diamond
                ));
                has = true;
            }
        }
        if !has {
            out.push_str("  暂无可领取的里程碑奖励\n  💡 继续努力达成更多里程碑吧！\n");
        } else {
            out.push_str("\n💡 输入 '领取里程碑+里程碑名' 领取奖励\n");
        }
        return out;
    }

    let progress = load_progress(db);
    let mut claimed = load_claimed(db, user_id);
    let ms = match MILESTONES.iter().find(|m| m.name.contains(args) || m.id.contains(args)) {
        Some(m) => m,
        None => return format!("{}\n⚠️ 未找到里程碑: {}\n💡 输入 '里程碑' 查看所有里程碑", prefix, args),
    };
    let cur = get_track(&progress, ms.track_type);
    if cur < ms.target {
        return format!(
            "{}\n⚠️ 里程碑 \"{}\" 尚未达成\n当前进度: {}/{} ({:.1}%)",
            prefix,
            ms.name,
            fmt_num(cur),
            fmt_num(ms.target),
            pct(cur, ms.target)
        );
    }
    if claimed.contains(&ms.id.to_string()) {
        return format!("{}\n⚠️ 你已经领取过 \"{}\" 的奖励了", prefix, ms.name);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, ms.reward_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, ms.reward_diamond);
    if !ms.reward_item.is_empty() && ms.reward_item_count > 0 {
        db.knapsack_add(user_id, ms.reward_item, ms.reward_item_count);
    }
    claimed.push(ms.id.to_string());
    save_claimed(db, user_id, &claimed);

    let mut out = format!(
        "{}\n🎉 ═══ 里程碑达成！ ═══\n{} {} ✅\n{}\n\n🎁 获得奖励:\n",
        prefix, ms.emoji, ms.name, ms.desc
    );
    out.push_str(&format!(
        "   💰 金币: +{}\n   💎 钻石: +{}\n",
        fmt_num(ms.reward_gold),
        ms.reward_diamond
    ));
    if !ms.reward_item.is_empty() && ms.reward_item_count > 0 {
        out.push_str(&format!("   🎁 {} ×{}\n", ms.reward_item, ms.reward_item_count));
    }
    out.push_str("\n感谢你为服务器做出的贡献！\n");
    out
}

/// 里程碑贡献排行
pub fn cmd_milestone_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let tt = match args {
        "" | "击杀" | "杀怪" => "total_kills",
        "金币" | "金" => "total_gold",
        "钻石" | "钻" => "total_diamond",
        "交易" => "trade_count",
        _ => return format!("{}\n⚠️ 未知排行类型: {}\n可选: 击杀/金币/钻石/交易", prefix, args),
    };
    let label = match tt {
        "total_kills" => "击杀贡献",
        "total_gold" => "金币产出",
        "total_diamond" => "钻石产出",
        "trade_count" => "交易贡献",
        _ => "贡献",
    };
    let key = format!("contrib_{}", tt);
    let raw = db.global_get(SECTION, &key);
    let contribs: serde_json::Map<String, serde_json::Value> = if raw.is_empty() {
        serde_json::Map::new()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let mut entries: Vec<(String, i64)> = contribs
        .iter()
        .filter_map(|(k, v)| v.as_i64().map(|val| (k.clone(), val)))
        .collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n🏆 ═══ {} 排行榜 ═══\n\n", prefix, label);
    if entries.is_empty() {
        out.push_str("  暂无贡献数据\n  💡 服务器正在收集数据中...\n");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (name, val)) in entries.iter().take(15).enumerate() {
            let medal = if i < 3 { medals[i] } else { "  " };
            out.push_str(&format!("{} {}. {}: {}\n", medal, i + 1, name, fmt_num(*val)));
        }
        let nick = db.read_basic(user_id, "Nickname");
        if let Some(pos) = entries.iter().position(|(n, _)| n == &nick) {
            out.push_str(&format!(
                "\n📍 你的排名: 第 {} 名 ({})\n",
                pos + 1,
                fmt_num(entries[pos].1)
            ));
        }
    }
    out
}

/// 记录贡献（供其他系统调用）
#[allow(dead_code)]
pub fn record_contribution(db: &Database, user_id: &str, contrib_type: &str, amount: i64) {
    if amount <= 0 {
        return;
    }
    let mut progress = load_progress(db);
    let cur = get_track(&progress, contrib_type);
    progress[contrib_type] = serde_json::json!(cur + amount);
    save_progress(db, &progress);

    let key = format!("contrib_{}", contrib_type);
    let raw = db.global_get(SECTION, &key);
    let mut contribs: serde_json::Map<String, serde_json::Value> = if raw.is_empty() {
        serde_json::Map::new()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let nick = db.read_basic(user_id, "Nickname");
    let entry = contribs.entry(nick).or_insert(serde_json::json!(0));
    if let Some(val) = entry.as_i64() {
        *entry = serde_json::json!(val + amount);
    }
    db.global_set(
        SECTION,
        &key,
        &serde_json::to_string(&serde_json::Value::Object(contribs)).unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestone_count() {
        assert_eq!(MILESTONES.len(), 15);
    }

    #[test]
    fn test_milestone_ids_unique() {
        let mut ids: Vec<&str> = MILESTONES.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), MILESTONES.len());
    }

    #[test]
    fn test_milestone_track_types() {
        let valid = [
            "player_count",
            "total_kills",
            "total_gold",
            "total_diamond",
            "total_levels",
            "guild_count",
            "trade_count",
        ];
        for ms in MILESTONES {
            assert!(valid.contains(&ms.track_type), "Invalid track type: {}", ms.track_type);
        }
    }

    #[test]
    fn test_milestone_targets_positive() {
        for ms in MILESTONES {
            assert!(ms.target > 0, "Target must be positive for {}", ms.name);
        }
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for ms in MILESTONES {
            assert!(ms.reward_gold > 0, "Gold reward must be positive for {}", ms.name);
            assert!(ms.reward_diamond > 0, "Diamond reward must be positive for {}", ms.name);
        }
    }

    #[test]
    fn test_milestone_targets_escalate() {
        let mut by_type: std::collections::HashMap<&str, Vec<i64>> = std::collections::HashMap::new();
        for ms in MILESTONES {
            by_type.entry(ms.track_type).or_default().push(ms.target);
        }
        for (tt, targets) in &by_type {
            if targets.len() > 1 {
                for i in 1..targets.len() {
                    assert!(targets[i] > targets[i - 1], "Targets should escalate for {}", tt);
                }
            }
        }
    }

    #[test]
    fn test_pct() {
        assert!((pct(0, 100) - 0.0).abs() < 0.01);
        assert!((pct(50, 100) - 50.0).abs() < 0.01);
        assert!((pct(100, 100) - 100.0).abs() < 0.01);
        assert!((pct(150, 100) - 100.0).abs() < 0.01);
        assert!((pct(0, 0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_bar() {
        assert_eq!(bar(50, 100, 10), "█████░░░░░");
        assert_eq!(bar(100, 100, 10), "██████████");
        assert_eq!(bar(0, 100, 10), "░░░░░░░░░░");
    }

    #[test]
    fn test_fmt_num() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(123), "123");
        assert_eq!(fmt_num(1234), "1,234");
        assert_eq!(fmt_num(1234567), "1,234,567");
        assert_eq!(fmt_num(-1234), "-1,234");
    }

    #[test]
    fn test_milestone_names_non_empty() {
        for ms in MILESTONES {
            assert!(!ms.name.is_empty());
            assert!(!ms.desc.is_empty());
            assert!(!ms.emoji.is_empty());
        }
    }

    #[test]
    fn test_milestone_gold_escalates() {
        let mut by_type: std::collections::HashMap<&str, Vec<i64>> = std::collections::HashMap::new();
        for ms in MILESTONES {
            by_type.entry(ms.track_type).or_default().push(ms.reward_gold);
        }
        for (tt, rewards) in &by_type {
            if rewards.len() > 1 {
                for i in 1..rewards.len() {
                    assert!(rewards[i] >= rewards[i - 1], "Gold should escalate for {}", tt);
                }
            }
        }
    }

    #[test]
    fn test_default_progress() {
        let p = default_progress();
        assert_eq!(get_track(&p, "player_count"), 0);
        assert_eq!(get_track(&p, "total_kills"), 0);
        assert_eq!(get_track(&p, "unknown"), 0);
    }
}
