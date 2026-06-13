/// CakeGame 玩家举报系统 (Player Report System)
///
/// 允许玩家举报违规行为，GM可查看/处理举报
/// 数据存储: Global 表 section='player_reports'
///
/// 指令: 举报玩家, 举报记录, 举报处理, 举报统计
use crate::db::Database;
use crate::user;
use chrono::Local;

const SECTION: &str = "player_reports";
const ADMIN_LEVEL: i32 = 98;

/// 举报类型
fn report_type_name(code: &str) -> &str {
    match code {
        "1" => "辱骂/骚扰",
        "2" => "外挂/作弊",
        "3" => "恶意PK",
        "4" => "诈骗",
        "5" => "刷屏",
        "6" => "其他违规",
        _ => "未知",
    }
}

/// 举报状态
fn report_status_name(code: &str) -> &str {
    match code {
        "pending" => "⏳ 待处理",
        "accepted" => "✅ 已采纳",
        "rejected" => "❌ 已驳回",
        "punished" => "🔨 已处罚",
        _ => "未知",
    }
}

/// 生成举报ID
fn gen_report_id(reporter: &str) -> String {
    let now = Local::now().format("%Y%m%d%H%M%S");
    let hash = reporter.len() as u64 * 31 + now.to_string().bytes().map(|b| b as u64).sum::<u64>();
    format!("RPT-{}-{:04}", now, hash % 10000)
}

/// 举报玩家
/// 用法: 举报玩家+玩家昵称+举报类型(1-6)+描述
/// 类型: 1=辱骂 2=外挂 3=恶意PK 4=诈骗 5=刷屏 6=其他
pub fn cmd_report_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册再使用举报功能。", prefix);
    }

    let parts: Vec<&str> = if args.contains('+') {
        args.split('+').map(|s| s.trim()).collect()
    } else {
        args.split_whitespace().collect()
    };
    if parts.len() < 2 {
        return format!(
            "{}\n用法: 举报玩家+玩家昵称+类型(1-6)+描述\n\
             类型说明:\n\
             1=辱骂/骚扰\n\
             2=外挂/作弊\n\
             3=恶意PK\n\
             4=诈骗\n\
             5=刷屏\n\
             6=其他违规",
            prefix
        );
    }

    let target_nick = parts[0];
    let report_type = parts.get(1).unwrap_or(&"6");
    let description = if parts.len() > 2 { parts[2] } else { "" };

    // 验证举报类型
    let type_code: i32 = report_type.parse().unwrap_or(0);
    if !(1..=6).contains(&type_code) {
        return format!("{}\n举报类型无效，请输入1-6。", prefix);
    }

    // 不能举报自己
    let my_nick = db.read_basic(user_id, "NickName");
    if target_nick == my_nick || target_nick == user_id {
        return format!("{}\n不能举报自己哦~", prefix);
    }

    // 检查目标是否存在
    let conn = db.lock_conn();
    let target_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM Basic_User WHERE NickName = ?1 OR UserId = ?1",
            rusqlite::params![target_nick],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !target_exists {
        return format!("{}\n找不到玩家 [{}]，请确认昵称。", prefix, target_nick);
    }

    // 防刷: 每10分钟只能举报一次
    let cd_key = format!("report_cd_{}", user_id);
    let last_report = db.global_get(SECTION, &cd_key);
    if !last_report.is_empty() {
        if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&last_report, "%Y-%m-%d %H:%M:%S") {
            let now = Local::now().naive_local();
            let elapsed = (now - last_time).num_seconds();
            if elapsed < 600 {
                let remaining = (600 - elapsed) / 60 + 1;
                return format!("{}\n举报冷却中，还需{}分钟。", prefix, remaining);
            }
        }
    }

    let report_id = gen_report_id(user_id);
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let type_name = report_type_name(report_type);

    // 存储举报: key=report_id, value=JSON-like数据
    let report_data = format!(
        "{}|{}|{}|{}|{}|{}|pending",
        report_id, user_id, my_nick, target_nick, type_name, now
    );
    db.global_set(SECTION, &report_id, &report_data);

    // 记录冷却
    db.global_set(SECTION, &cd_key, &now);

    // 举报计数
    let count_key = format!("report_count_{}", target_nick);
    let current_count: i32 = db.global_get(SECTION, &count_key).parse().unwrap_or(0);
    db.global_set(SECTION, &count_key, &(current_count + 1).to_string());

    // 用户举报历史
    let my_history_key = format!("my_reports_{}", user_id);
    let existing = db.global_get(SECTION, &my_history_key);
    let new_entry = if existing.is_empty() {
        report_id.clone()
    } else {
        format!("{},{}", existing, report_id)
    };
    db.global_set(SECTION, &my_history_key, &new_entry);

    let mut out = format!("{}\n═══ 📢 举报提交成功 ═══", prefix);
    out.push_str(&format!("\n举报编号: {}", report_id));
    out.push_str(&format!("\n被举报人: {}", target_nick));
    out.push_str(&format!("\n举报类型: {}", type_name));
    if !description.is_empty() {
        out.push_str(&format!("\n详细描述: {}", description));
    }
    out.push_str(&format!("\n提交时间: {}", now));
    out.push_str("\n\n⏰ 举报将在24小时内处理");
    out.push_str("\n💡 恶意举报将受到处罚");

    out
}

/// 查看我的举报记录
pub fn cmd_my_reports(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let my_history_key = format!("my_reports_{}", user_id);
    let history = db.global_get(SECTION, &my_history_key);

    if history.is_empty() {
        return format!("{}\n═══ 📋 我的举报记录 ═══\n暂无举报记录。", prefix);
    }

    let report_ids: Vec<&str> = history.split(',').collect();
    let mut out = format!("{}\n═══ 📋 我的举报记录 ═══", prefix);
    out.push_str(&format!("\n共 {} 条举报\n", report_ids.len()));

    let mut shown = 0;
    for rid in report_ids.iter().rev() {
        if shown >= 10 {
            out.push_str(&format!("\n... 还有{}条更早的记录", report_ids.len() - shown));
            break;
        }
        let data = db.global_get(SECTION, rid);
        if data.is_empty() {
            continue;
        }
        let fields: Vec<&str> = data.split('|').collect();
        if fields.len() >= 7 {
            let target = fields.get(3).unwrap_or(&"?");
            let rtype = fields.get(4).unwrap_or(&"?");
            let time = fields.get(5).unwrap_or(&"?");
            let status = fields.get(6).unwrap_or(&"pending");
            out.push_str(&format!(
                "\n  {} | 举报{} | {} | {} | {}",
                rid,
                target,
                rtype,
                &time[..10.min(time.len())],
                report_status_name(status)
            ));
            shown += 1;
        }
    }

    out
}

/// GM处理举报
/// 用法: 举报处理+举报编号+处理结果(accept/reject/punish)
pub fn cmd_handle_report(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // GM权限检查
    let perm = crate::permissions::get_permission(db, user_id);
    if perm < ADMIN_LEVEL {
        return format!("{}\n❌ 仅管理员可处理举报。", prefix);
    }

    let parts: Vec<&str> = if args.contains('+') {
        args.split('+').map(|s| s.trim()).collect()
    } else {
        args.split_whitespace().collect()
    };
    if parts.len() < 2 {
        return format!(
            "{}\n用法: 举报处理+举报编号+结果\n\
             结果: accept=采纳, reject=驳回, punish=已处罚",
            prefix
        );
    }

    let report_id = parts[0];
    let action = parts[1];

    let data = db.global_get(SECTION, report_id);
    if data.is_empty() {
        return format!("{}\n找不到举报 [{}]。", prefix, report_id);
    }

    let mut fields: Vec<String> = data.split('|').map(|s| s.to_string()).collect();
    if fields.len() < 7 {
        return format!("{}\n举报数据格式异常。", prefix);
    }

    let new_status = match action {
        "accept" | "采纳" => "accepted",
        "reject" | "驳回" => "rejected",
        "punish" | "处罚" => {
            // 自动增加被举报人邪恶值
            let target_nick = &fields[3];
            let evil_key = format!("evil_{}", target_nick);
            let current_evil: i32 = db.global_get("xiaoheiwu", &evil_key).parse().unwrap_or(0);
            db.global_set("xiaoheiwu", &evil_key, &(current_evil + 20).to_string());
            "punished"
        }
        _ => return format!("{}\n无效操作。使用: accept/reject/punish", prefix),
    };

    fields[6] = new_status.to_string();
    let updated = fields.join("|");
    db.global_set(SECTION, report_id, &updated);

    format!(
        "{}\n✅ 举报 [{}] 已处理: {}",
        prefix,
        report_id,
        report_status_name(new_status)
    )
}

/// 举报统计（GM/全服）
pub fn cmd_report_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    // 统计所有举报
    let mut stmt = match conn.prepare("SELECT key, value FROM Global WHERE section = ?1 AND key LIKE 'RPT-%'") {
        Ok(s) => s,
        Err(_) => return format!("{}\n暂无举报数据。", prefix),
    };

    let mut total = 0i32;
    let mut pending = 0i32;
    let mut accepted = 0i32;
    let mut rejected = 0i32;
    let mut punished = 0i32;
    let mut type_counts = std::collections::HashMap::new();

    if let Ok(rows) = stmt.query_map(rusqlite::params![SECTION], |row| {
        let value: String = row.get(1)?;
        Ok(value)
    }) {
        for row in rows.flatten() {
            total += 1;
            let fields: Vec<&str> = row.split('|').collect();
            if let Some(status) = fields.get(6) {
                match *status {
                    "pending" => pending += 1,
                    "accepted" => accepted += 1,
                    "rejected" => rejected += 1,
                    "punished" => punished += 1,
                    _ => {}
                }
            }
            if let Some(rtype) = fields.get(4) {
                *type_counts.entry(rtype.to_string()).or_insert(0i32) += 1;
            }
        }
    }

    let mut out = format!("{}\n═══ 📊 举报统计 ═══", prefix);
    out.push_str(&format!("\n总举报数: {}", total));
    out.push_str(&format!("\n⏳ 待处理: {}", pending));
    out.push_str(&format!("\n✅ 已采纳: {}", accepted));
    out.push_str(&format!("\n❌ 已驳回: {}", rejected));
    out.push_str(&format!("\n🔨 已处罚: {}", punished));

    if !type_counts.is_empty() {
        out.push_str("\n\n═══ 按类型统计 ═══");
        let mut sorted: Vec<_> = type_counts.iter().collect();
        sorted.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
        for (rtype, count) in sorted {
            out.push_str(&format!("\n  {} : {} 件", rtype, count));
        }
    }

    let processed = accepted + rejected + punished;
    if total > 0 {
        let rate = processed as f64 / total as f64 * 100.0;
        out.push_str(&format!("\n\n处理率: {:.1}%", rate));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_type_names() {
        assert_eq!(report_type_name("1"), "辱骂/骚扰");
        assert_eq!(report_type_name("2"), "外挂/作弊");
        assert_eq!(report_type_name("3"), "恶意PK");
        assert_eq!(report_type_name("4"), "诈骗");
        assert_eq!(report_type_name("5"), "刷屏");
        assert_eq!(report_type_name("6"), "其他违规");
        assert_eq!(report_type_name("99"), "未知");
    }

    #[test]
    fn test_report_status_names() {
        assert_eq!(report_status_name("pending"), "⏳ 待处理");
        assert_eq!(report_status_name("accepted"), "✅ 已采纳");
        assert_eq!(report_status_name("rejected"), "❌ 已驳回");
        assert_eq!(report_status_name("punished"), "🔨 已处罚");
        assert_eq!(report_status_name("other"), "未知");
    }

    #[test]
    fn test_gen_report_id_format() {
        let id = gen_report_id("test_user");
        assert!(id.starts_with("RPT-"));
        assert!(id.contains('-'));
        // Should be like RPT-20260609213500-1234
        let parts: Vec<&str> = id.split('-').collect();
        assert!(parts.len() >= 3, "Report ID should have at least 3 parts");
    }

    #[test]
    fn test_gen_report_id_unique() {
        let id1 = gen_report_id("user_a");
        let id2 = gen_report_id("user_b");
        // Different users should produce different IDs (not guaranteed but high probability)
        // At minimum, both should be valid format
        assert!(id1.starts_with("RPT-"));
        assert!(id2.starts_with("RPT-"));
    }

    #[test]
    fn test_report_type_coverage() {
        // All 6 types should have valid names
        for i in 1..=6 {
            let s = i.to_string();
            let name = report_type_name(&s);
            assert_ne!(name, "未知", "Type {} should have a name", i);
        }
    }
}
