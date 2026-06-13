/// CakeGame 怪物猎杀日志系统
///
/// 追踪每个玩家的怪物击杀记录，提供详细统计分析和猎杀里程碑奖励。
/// 数据存储在 Global 表:
///   SECTION='monster_kills', ID='{user_id}' → JSON: {"稻草人": 100, "史莱姆": 50, ...}
///   SECTION='kill_milestones', ID='{user_id}' → JSON: {"100": "claimed", "500": "claimed"}
///   SECTION='kill_totals', ID='{user_id}' → JSON: {"total": 150, "last_kill_time": "..."}
///
/// 新增指令: 猎杀日志, 猎杀详情, 猎杀排行, 猎杀里程碑, 掉落分析
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::collections::HashMap;

// ==================== 里程碑定义 ====================

/// 猎杀里程碑
struct KillMilestone {
    count: i32,
    name: &'static str,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
}

/// 里程碑列表 (按击杀数递增)
const MILESTONES: &[KillMilestone] = &[
    KillMilestone {
        count: 10,
        name: "初级猎手",
        reward_gold: 500,
        reward_diamond: 5,
        reward_item: "【普通】生命药水*5",
    },
    KillMilestone {
        count: 50,
        name: "怪物克星",
        reward_gold: 2000,
        reward_diamond: 15,
        reward_item: "【稀有】强化石*2",
    },
    KillMilestone {
        count: 100,
        name: "百战勇士",
        reward_gold: 5000,
        reward_diamond: 30,
        reward_item: "【稀有】高级生命药水*10",
    },
    KillMilestone {
        count: 200,
        name: "屠杀者",
        reward_gold: 10000,
        reward_diamond: 50,
        reward_item: "【稀有】复活卷轴*2",
    },
    KillMilestone {
        count: 500,
        name: "战场收割者",
        reward_gold: 25000,
        reward_diamond: 100,
        reward_item: "【史诗】强化石*5",
    },
    KillMilestone {
        count: 1000,
        name: "千杀传奇",
        reward_gold: 50000,
        reward_diamond: 200,
        reward_item: "【传说】高级强化石*3",
    },
    KillMilestone {
        count: 2000,
        name: "万军屠戮者",
        reward_gold: 100000,
        reward_diamond: 500,
        reward_item: "【传说】神圣强化石*1",
    },
    KillMilestone {
        count: 5000,
        name: "灭世之主",
        reward_gold: 250000,
        reward_diamond: 1000,
        reward_item: "【超界】传说宝箱*1",
    },
];

// ==================== 怪物数据 ====================

/// 从 Config_Monster 获取怪物列表
fn get_all_monsters(db: &Database) -> Vec<(String, String, i32, i32, i64, i64)> {
    // (name, type, hp, level_estimate, reward_exp, reward_gold)
    db.query_rows(
        "SELECT Monster_Name, Monster_Type, Monster_HP, Reward_Exp, Reward_Gold FROM Config_Monster",
        &[],
        |row| {
            let name: String = row.get(0)?;
            let mtype: String = row.get(1).unwrap_or_default();
            let hp_str: String = row.get(2).unwrap_or_default();
            let exp_str: String = row.get(3).unwrap_or_default();
            let gold_str: String = row.get(4).unwrap_or_default();
            let hp: i32 = hp_str.parse().unwrap_or(100);
            // 估算等级: HP / 100
            let level_est = (hp / 100).max(1);
            let exp: i64 = exp_str.parse().unwrap_or(0);
            let gold: i64 = gold_str.parse().unwrap_or(0);
            Ok((name, mtype, hp, level_est, exp, gold))
        },
    )
}

/// 获取玩家的击杀记录 (JSON)
fn get_kills_json(db: &Database, user_id: &str) -> String {
    db.global_get("monster_kills", user_id)
}

/// 解析击杀 JSON
fn parse_kills(json: &str) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    if json.is_empty() {
        return map;
    }
    // 格式: {"稻草人":100,"史莱姆":50}
    let cleaned = json.trim_matches(|c| c == '{' || c == '}');
    for part in cleaned.split(',') {
        let kv: Vec<&str> = part.splitn(2, ':').collect();
        if kv.len() == 2 {
            let key = kv[0].trim().trim_matches('"');
            let val: i32 = kv[1].trim().trim_matches('"').parse().unwrap_or(0);
            if val > 0 {
                map.insert(key.to_string(), val);
            }
        }
    }
    map
}

/// 序列化击杀 JSON
fn serialize_kills(kills: &HashMap<String, i32>) -> String {
    let mut parts: Vec<String> = kills
        .iter()
        .filter(|(_, &v)| v > 0)
        .map(|(k, v)| format!("\"{}\":{}", k, v))
        .collect();
    parts.sort();
    format!("{{{}}}", parts.join(","))
}

#[allow(dead_code)]
/// 获取总击杀数
fn get_total_kills(db: &Database, user_id: &str) -> i32 {
    let json = db.global_get("kill_totals", user_id);
    if json.is_empty() {
        return 0;
    }
    // 格式: {"total":150}
    if let Some(idx) = json.find("\"total\":") {
        let rest = &json[idx + 8..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num_str.parse().unwrap_or(0);
    }
    0
}

#[allow(dead_code)]
/// 设置总击杀数
fn set_total_kills(db: &Database, user_id: &str, total: i32) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.global_set(
        "kill_totals",
        user_id,
        &format!("{{\"total\":{},\"last_kill\":\"{}\"}}", total, now),
    );
}

// ==================== 公共 API ====================

#[allow(dead_code)]
/// 记录一次击杀 (战斗系统调用)
pub fn record_kill(db: &Database, user_id: &str, monster_name: &str) {
    let json = get_kills_json(db, user_id);
    let mut kills = parse_kills(&json);
    *kills.entry(monster_name.to_string()).or_insert(0) += 1;
    db.global_set("monster_kills", user_id, &serialize_kills(&kills));

    // 更新总击杀
    let total = get_total_kills(db, user_id) + 1;
    set_total_kills(db, user_id, total);
}

#[allow(dead_code)]
/// 检查并获取里程碑奖励 (返回 Some(message) 如果达到新里程碑)
pub fn check_milestone(db: &Database, user_id: &str) -> Option<String> {
    let total = get_total_kills(db, user_id);
    let claimed_json = db.global_get("kill_milestones", user_id);
    let mut claimed: Vec<String> = Vec::new();
    if !claimed_json.is_empty() {
        let cleaned = claimed_json.trim_matches(|c| c == '[' || c == ']');
        for s in cleaned.split(',') {
            claimed.push(s.trim().trim_matches('"').to_string());
        }
    }

    for ms in MILESTONES {
        let key = ms.count.to_string();
        if total >= ms.count && !claimed.contains(&key) {
            claimed.push(key);
            // 保存已领取
            let claimed_str: Vec<String> = claimed.iter().map(|s| format!("\"{}\"", s)).collect();
            db.global_set("kill_milestones", user_id, &format!("[{}]", claimed_str.join(",")));

            // 发放奖励
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, ms.reward_gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, ms.reward_diamond);

            return Some(format!(
                "🎉 恭喜达成【{}】里程碑！累计击杀 {} 只怪物！\n💰 奖励: {}金币 + {}钻石\n📦 额外奖励: {}",
                ms.name, ms.count, ms.reward_gold, ms.reward_diamond, ms.reward_item
            ));
        }
    }
    None
}

// ==================== 指令实现 ====================

/// 猎杀日志 - 查看个人怪物击杀统计
pub fn cmd_kill_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let kills_json = get_kills_json(db, user_id);
    let kills = parse_kills(&kills_json);
    let total = get_total_kills(db, user_id);

    if total == 0 {
        return format!(
            "{}\n📜 猎杀日志\n你还没有击杀过任何怪物。\n💡 去搜索怪物并攻击吧！",
            prefix
        );
    }

    let mut result = format!("{}\n═══ 📜 猎杀日志 ═══\n", prefix);
    result.push_str(&format!("🎯 总击杀数: {}\n", total));
    result.push_str(&format!("📊 击杀种类: {}种\n\n", kills.len()));

    // 按击杀数排序
    let mut sorted: Vec<(&String, &i32)> = kills.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    result.push_str("🏆 击杀排行:\n");
    for (i, (name, count)) in sorted.iter().enumerate().take(10) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let max_val = sorted.iter().map(|(_, c)| **c).max().unwrap_or(1).max(1);
        let bar = progress_bar(**count, max_val);
        result.push_str(&format!("{} {} ×{} {}\n", medal, name, count, bar));
    }

    if kills.len() > 10 {
        result.push_str(&format!("\n  ...还有 {} 种怪物\n", kills.len() - 10));
    }

    // 里程碑进度
    result.push_str("\n🎯 里程碑进度:\n");
    for ms in MILESTONES {
        let claimed_json = db.global_get("kill_milestones", user_id);
        let claimed = claimed_json.contains(&ms.count.to_string());
        let status = if claimed {
            "✅"
        } else if total >= ms.count {
            "🎁"
        } else {
            "⬜"
        };
        result.push_str(&format!(
            "  {} {}击杀: {} {}\n",
            status,
            ms.name,
            ms.count,
            if claimed { "(已领取)" } else { "" }
        ));
    }

    result
}

/// 猎杀详情 - 查看特定怪物的击杀详情
pub fn cmd_kill_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let monster_name = args.trim();

    if monster_name.is_empty() {
        return format!("{}\n格式: 猎杀详情+怪物名称\n例如: 猎杀详情+稻草人", prefix);
    }

    let kills_json = get_kills_json(db, user_id);
    let kills = parse_kills(&kills_json);

    // 模糊匹配
    let matched: Vec<(&String, &i32)> = kills.iter().filter(|(k, _)| k.contains(monster_name)).collect();

    if matched.is_empty() {
        return format!(
            "{}\n⚠️ 你还没有击杀过「{}」\n💡 去搜索怪物并攻击吧！",
            prefix, monster_name
        );
    }

    // 获取怪物配置数据
    let all_monsters = get_all_monsters(db);

    let mut result = format!("{}\n═══ 🔍 怪物猎杀详情 ═══\n", prefix);

    for (name, count) in &matched {
        result.push_str(&format!("\n🎯 {} ×{}\n", name, count));

        // 从 Config_Monster 找怪物数据
        if let Some((_, mtype, hp, level_est, exp, gold)) = all_monsters.iter().find(|(n, _, _, _, _, _)| n == *name) {
            result.push_str(&format!("  📊 类型: {} | 推荐等级: Lv.{}\n", mtype, level_est));
            result.push_str(&format!("  ❤️ HP: {} | 🌟 经验: {} | 💰 金币: {}\n", hp, exp, gold));
            result.push_str(&format!("  💀 你已击杀 {} 次\n", count));
            let total_exp = exp * (**count as i64);
            let total_gold = gold * (**count as i64);
            result.push_str(&format!("  📈 累计获得: {}经验 + {}金币\n", total_exp, total_gold));
        } else {
            result.push_str(&format!("  💀 击杀次数: {}\n", count));
        }
    }

    result
}

/// 猎杀排行 - 全服怪物击杀排行
pub fn cmd_kill_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 收集所有玩家的总击杀
    let mut rankings: Vec<(String, i32)> = Vec::new();
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION='kill_totals'") {
        if let Ok(rows) = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((id, data))
        }) {
            for row in rows.flatten() {
                let total = if let Some(idx) = row.1.find("\"total\":") {
                    let rest = &row.1[idx + 8..];
                    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    num_str.parse().unwrap_or(0)
                } else {
                    0
                };
                if total > 0 {
                    // 获取昵称
                    let nick = db.read_basic(&row.0, ITEM_NAME);
                    let display = if nick.is_empty() { row.0.clone() } else { nick };
                    rankings.push((display, total));
                }
            }
        }
    }
    drop(conn);

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    if rankings.is_empty() {
        return format!("{}\n📊 猎杀排行\n暂无击杀数据。", prefix);
    }

    let target = args.trim();
    let max_total = rankings.iter().map(|(_, t)| *t).max().unwrap_or(1).max(1);
    let mut result = format!("{}\n═══ 📊 全服猎杀排行 ═══\n", prefix);

    for (i, (name, total)) in rankings.iter().enumerate().take(10) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => &format!("{:>2}.", i + 1),
        };
        let bar = progress_bar(*total, max_total);
        result.push_str(&format!("{} {} ×{} {}\n", medal, name, total, bar));
    }

    // 显示当前用户排名
    let my_name = db.read_basic(user_id, ITEM_NAME);
    if let Some((pos, (_, total))) = rankings
        .iter()
        .enumerate()
        .find(|(_, (n, _))| *n == my_name || n == user_id)
    {
        result.push_str(&format!("\n📍 你的排名: 第{}名 (×{})\n", pos + 1, total));
    }

    if !target.is_empty() {
        // 按怪物名筛选
        result.push_str(&format!("\n🔍 筛选「{}」的击杀排行:\n", target));
        let mut monster_rank: Vec<(String, i32)> = Vec::new();
        let _conn2 = db.lock_conn();
        if let Ok(mut stmt) = _conn2.prepare("SELECT ID, DATA FROM Global WHERE SECTION='monster_kills'") {
            if let Ok(rows) = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let data: String = row.get(1)?;
                Ok((id, data))
            }) {
                for row in rows.flatten() {
                    let kills = parse_kills(&row.1);
                    for (mname, count) in &kills {
                        if mname.contains(target) {
                            let nick = db.read_basic(&row.0, ITEM_NAME);
                            let display = if nick.is_empty() { row.0.clone() } else { nick };
                            monster_rank.push((display, *count));
                        }
                    }
                }
            }
        }
        monster_rank.sort_by_key(|b| std::cmp::Reverse(b.1));
        for (i, (name, count)) in monster_rank.iter().enumerate().take(5) {
            result.push_str(&format!("  {}. {} ×{}\n", i + 1, name, count));
        }
    }

    result
}

/// 猎杀里程碑 - 领取里程碑奖励
pub fn cmd_claim_milestone(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let total = get_total_kills(db, user_id);

    // 查看里程碑状态
    let claimed_json = db.global_get("kill_milestones", user_id);
    let mut claimed: Vec<String> = Vec::new();
    if !claimed_json.is_empty() {
        let cleaned = claimed_json.trim_matches(|c| c == '[' || c == ']');
        for s in cleaned.split(',') {
            claimed.push(s.trim().trim_matches('"').to_string());
        }
    }

    let mut result = format!("{}\n═══ 🎯 猎杀里程碑 ═══\n", prefix);
    result.push_str(&format!("📊 当前总击杀: {}\n\n", total));

    let mut unclaimed: Vec<&KillMilestone> = Vec::new();

    for ms in MILESTONES {
        let key = ms.count.to_string();
        let is_claimed = claimed.contains(&key);
        let is_ready = total >= ms.count && !is_claimed;

        let status = if is_claimed {
            "✅"
        } else if is_ready {
            "🎁"
        } else {
            "⬜"
        };

        let progress = if total >= ms.count {
            format!("{}/{}", ms.count, ms.count)
        } else {
            format!("{}/{}", total, ms.count)
        };

        let pct = if ms.count > 0 {
            ((total as f64 / ms.count as f64) * 100.0).min(100.0) as i32
        } else {
            100
        };

        result.push_str(&format!(
            "{} 【{}】{}击杀 — {} ({}%)\n   💰{}金 + {}💎 + {}\n",
            status, ms.name, ms.count, progress, pct, ms.reward_gold, ms.reward_diamond, ms.reward_item
        ));

        if is_ready {
            unclaimed.push(ms);
        }
    }

    if !unclaimed.is_empty() {
        // 自动领取
        for ms in &unclaimed {
            let key = ms.count.to_string();
            claimed.push(key);
        }
        let claimed_str: Vec<String> = claimed.iter().map(|s| format!("\"{}\"", s)).collect();
        db.global_set("kill_milestones", user_id, &format!("[{}]", claimed_str.join(",")));

        let mut total_gold: i64 = 0;
        let mut total_diamond: i64 = 0;
        for ms in &unclaimed {
            total_gold += ms.reward_gold;
            total_diamond += ms.reward_diamond;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, ms.reward_gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, ms.reward_diamond);
        }

        result.push_str(&format!(
            "\n🎉 自动领取了 {} 个里程碑奖励！\n💰 共获得: {}金币 + {}钻石",
            unclaimed.len(),
            total_gold,
            total_diamond
        ));
    } else if total == 0 {
        result.push_str("\n💡 去击杀怪物解锁里程碑奖励吧！");
    } else {
        // 找下一个未达成的里程碑
        if let Some(next) = MILESTONES.iter().find(|ms| total < ms.count) {
            result.push_str(&format!(
                "\n⏳ 下一个里程碑: 还需击杀 {} 只怪物达到【{}】",
                next.count - total,
                next.name
            ));
        } else {
            result.push_str("\n🏆 恭喜！你已达成所有猎杀里程碑！");
        }
    }

    result
}

/// 掉落分析 - 查看怪物掉落配置
pub fn cmd_drop_analysis(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();

    let conn = db.lock_conn();

    if query.is_empty() {
        // 显示所有怪物的掉落概览
        let mut result = format!("{}\n═══ 📦 掉落分析 ═══\n", prefix);
        result.push_str("格式: 掉落分析+怪物名称\n\n");

        if let Ok(mut stmt) =
            conn.prepare("SELECT Monster_Name, Reward_Exp, Reward_Gold, Reward_Goods FROM Config_Monster")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                let name: String = row.get(0)?;
                let exp: String = row.get(1).unwrap_or_default();
                let gold: String = row.get(2).unwrap_or_default();
                let goods: String = row.get(3).unwrap_or_default();
                Ok((name, exp, gold, goods))
            }) {
                for row in rows.flatten() {
                    let has_drops = !row.3.is_empty() && row.3 != "[NULL]";
                    let drop_icon = if has_drops { "📦" } else { "❌" };
                    result.push_str(&format!("  {} {} (EXP:{} G:{})\n", drop_icon, row.0, row.1, row.2));
                }
            }
        }
        return result;
    }

    // 搜索特定怪物
    let mut result = format!("{}\n═══ 📦 掉落分析: {} ═══\n", prefix, query);

    if let Ok(mut stmt) = conn.prepare(
        "SELECT Monster_Name, Monster_Type, Monster_HP, Reward_Exp, Reward_Gold, Reward_Goods, Monster_AD, Monster_Defense FROM Config_Monster WHERE Monster_Name LIKE ?1"
    ) {
        let pattern = format!("%{}%", query);
        if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
            let name: String = row.get(0)?;
            let mtype: String = row.get(1).unwrap_or_default();
            let hp: String = row.get(2).unwrap_or_default();
            let exp: String = row.get(3).unwrap_or_default();
            let gold: String = row.get(4).unwrap_or_default();
            let goods: String = row.get(5).unwrap_or_default();
            let ad: String = row.get(6).unwrap_or_default();
            let def: String = row.get(7).unwrap_or_default();
            Ok((name, mtype, hp, exp, gold, goods, ad, def))
        }) {
            let mut found = false;
            for row in rows.flatten() {
                found = true;
                result.push_str(&format!("\n🎯 {} [{}]\n", row.0, row.1));
                result.push_str(&format!("  ❤️ HP: {} | ⚔️ 物攻: {} | 🛡️ 防御: {}\n", row.2, row.6, row.7));
                result.push_str(&format!("  🌟 经验: {} | 💰 金币: {}\n", row.3, row.4));

                if !row.5.is_empty() && row.5 != "[NULL]" {
                    result.push_str("  📦 掉落物品:\n");
                    for item in row.5.split(',') {
                        let parts: Vec<&str> = item.splitn(3, '*').collect();
                        if parts.len() >= 2 {
                            let item_name = parts[0];
                            let rate_str = parts.last().unwrap_or(&"100");
                            let rate: i32 = rate_str.parse().unwrap_or(100);
                            let prob = format!("{}%", rate);
                            let bar = progress_bar(rate, 100);
                            result.push_str(&format!("    • {} ({}) {}\n", item_name, prob, bar));
                        } else {
                            result.push_str(&format!("    • {}\n", item));
                        }
                    }
                } else {
                    result.push_str("  📦 无掉落物品\n");
                }
            }
            if !found {
                result.push_str(&format!("\n⚠️ 未找到怪物「{}」", query));
            }
        }
    }

    result
}

/// 进度条辅助函数
fn progress_bar(current: i32, max: i32) -> String {
    let max = max.max(1);
    let filled = ((current as f64 / max as f64) * 10.0).min(10.0) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kills_empty() {
        let kills = parse_kills("");
        assert!(kills.is_empty());
    }

    #[test]
    fn test_parse_kills_single() {
        let kills = parse_kills("{\"稻草人\":100}");
        assert_eq!(kills.get("稻草人"), Some(&100));
    }

    #[test]
    fn test_parse_kills_multiple() {
        let kills = parse_kills("{\"史莱姆\":50,\"稻草人\":100}");
        assert_eq!(kills.len(), 2);
        assert_eq!(kills.get("稻草人"), Some(&100));
        assert_eq!(kills.get("史莱姆"), Some(&50));
    }

    #[test]
    fn test_serialize_kills_roundtrip() {
        let mut map = HashMap::new();
        map.insert("稻草人".to_string(), 100);
        map.insert("史莱姆".to_string(), 50);
        let json = serialize_kills(&map);
        let parsed = parse_kills(&json);
        assert_eq!(parsed.get("稻草人"), Some(&100));
        assert_eq!(parsed.get("史莱姆"), Some(&50));
    }

    #[test]
    fn test_serialize_kills_empty() {
        let map = HashMap::new();
        let json = serialize_kills(&map);
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_parse_kills_zero_filtered() {
        let kills = parse_kills("{\"稻草人\":0,\"史莱姆\":5}");
        assert_eq!(kills.len(), 1);
        assert_eq!(kills.get("史莱姆"), Some(&5));
    }

    #[test]
    fn test_milestones_sorted() {
        for i in 1..MILESTONES.len() {
            assert!(MILESTONES[i].count > MILESTONES[i - 1].count);
        }
    }

    #[test]
    fn test_milestones_rewards_positive() {
        for ms in MILESTONES {
            assert!(ms.reward_gold > 0);
            assert!(ms.reward_diamond > 0);
            assert!(!ms.reward_item.is_empty());
        }
    }

    #[test]
    fn test_milestone_names_unique() {
        let mut names: Vec<&str> = MILESTONES.iter().map(|m| m.name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len);
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100);
        assert!(bar.contains("██████████"));
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100);
        assert!(bar.contains("░░░░░░░░░░"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100);
        assert!(bar.contains("█████"));
        assert!(bar.contains("░░░░░"));
    }

    #[test]
    fn test_parse_kills_with_spaces() {
        let kills = parse_kills("{ \"稻草人\" : 100 }");
        assert_eq!(kills.get("稻草人"), Some(&100));
    }

    #[test]
    fn test_milestone_count_range() {
        assert_eq!(MILESTONES[0].count, 10);
        assert!(MILESTONES.last().unwrap().count >= 1000);
    }

    #[test]
    fn test_serialize_single() {
        let mut map = HashMap::new();
        map.insert("测试".to_string(), 1);
        let json = serialize_kills(&map);
        assert_eq!(json, "{\"测试\":1}");
    }
}
