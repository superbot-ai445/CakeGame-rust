/// 队伍讨伐目标系统
/// 激活 Config_Ranks 表 (22条队伍讨伐目标数据)
/// 数据结构: ID(队伍标识), Name(队伍名), Member(成员列表), Goal(讨伐目标)
/// 玩家可查看队伍的讨伐目标，追踪各地图怪物击杀进度
use crate::db::Database;
use crate::user;

/// 队伍讨伐目标
#[derive(Debug)]
#[allow(dead_code)]
struct TeamGoal {
    team_id: String,
    team_name: String,
    members: Vec<TeamMember>,
    goals: Vec<MapGoal>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct TeamMember {
    user_id: String,
    position: String, // "Captain" or "Member"
}

#[derive(Debug)]
struct MapGoal {
    map_name: String,
    monsters: Vec<MonsterGoal>,
}

#[derive(Debug, Clone)]
struct MonsterGoal {
    name: String,
    target_count: i32,
}

/// 解析成员列表 (格式: [UserID]\r\nPosition=Captain\r\n[UserID2]\r\nPosition=Member)
fn parse_members(member_str: &str) -> Vec<TeamMember> {
    let mut members = Vec::new();
    let mut current_id = String::new();

    for line in member_str.lines() {
        let line = line.trim().replace('\r', "");
        if line.starts_with('[') && line.ends_with(']') {
            current_id = line[1..line.len() - 1].to_string();
        } else if let Some(pos) = line.strip_prefix("Position=") {
            let pos = pos.to_string();
            if !current_id.is_empty() {
                members.push(TeamMember {
                    user_id: current_id.clone(),
                    position: pos,
                });
                current_id.clear();
            }
        }
    }
    members
}

/// 解析讨伐目标 (格式: [地图名]\r\n怪物名=数量)
fn parse_goals(goal_str: &str) -> Vec<MapGoal> {
    let mut goals = Vec::new();
    let mut current_map = String::new();
    let mut current_monsters = Vec::new();

    for line in goal_str.lines() {
        let line = line.trim().replace('\r', "");
        if line.starts_with('[') && line.ends_with(']') {
            // Save previous map
            if !current_map.is_empty() && !current_monsters.is_empty() {
                goals.push(MapGoal {
                    map_name: current_map.clone(),
                    monsters: current_monsters.clone(),
                });
            }
            current_map = line[1..line.len() - 1].to_string();
            current_monsters.clear();
        } else if let Some((name, count_str)) = line.split_once('=') {
            let count: i32 = count_str.trim().parse().unwrap_or(0);
            if count > 0 {
                current_monsters.push(MonsterGoal {
                    name: name.trim().to_string(),
                    target_count: count,
                });
            }
        }
    }
    // Save last map
    if !current_map.is_empty() && !current_monsters.is_empty() {
        goals.push(MapGoal {
            map_name: current_map,
            monsters: current_monsters,
        });
    }
    goals
}

/// 加载所有队伍讨伐目标
fn load_all_team_goals(db: &Database) -> Vec<TeamGoal> {
    db.query_rows("SELECT ID, Name, Member, Goal FROM Config_Ranks", &[], |row| {
        let team_id: String = row.get(0).unwrap_or_default();
        let team_name: String = row.get(1).unwrap_or_default();
        let member_str: String = row.get(2).unwrap_or_default();
        let goal_str: String = row.get(3).unwrap_or_default();

        let members = parse_members(&member_str);
        let goals = parse_goals(&goal_str);

        Ok(TeamGoal {
            team_id,
            team_name,
            members,
            goals,
        })
    })
}

/// 查看讨伐目标列表
pub fn cmd_view_team_goals(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let teams = load_all_team_goals(db);

    if teams.is_empty() {
        return format!("{}\n═══ 队伍讨伐目标 ═══\n暂无队伍讨伐目标数据。", prefix);
    }

    let args = args.trim();

    // 查看特定队伍详情
    if !args.is_empty() {
        for team in &teams {
            if team.team_name.contains(args) || team.team_id == args {
                return format_team_detail(&prefix, team, db, user_id);
            }
        }
        // Try matching by member ID
        for team in &teams {
            if team.members.iter().any(|m| m.user_id == user_id) {
                return format_team_detail(&prefix, team, db, user_id);
            }
        }
        return format!("{}\n找不到讨伐目标 [{}]。", prefix, args);
    }

    // 列出所有队伍讨伐目标概览
    let mut r = format!("{}\n═══ 🏆 队伍讨伐目标 ═══", prefix);
    for (i, team) in teams.iter().enumerate() {
        let total_goals: i32 = team
            .goals
            .iter()
            .map(|g| g.monsters.iter().map(|m| m.target_count).sum::<i32>())
            .sum();
        let captain = team
            .members
            .iter()
            .find(|m| m.position == "Captain")
            .map(|m| m.user_id.as_str())
            .unwrap_or("未知");
        let is_my_team = team.members.iter().any(|m| m.user_id == user_id);
        let tag = if is_my_team { " ⭐" } else { "" };
        r.push_str(&format!(
            "\n{}. [{}]{} 队长:{} | 成员:{} | 目标总计:{}",
            i + 1,
            team.team_name,
            tag,
            captain,
            team.members.len(),
            total_goals
        ));
    }
    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    r.push_str("\n\n发送 '讨伐目标+队伍名' 查看队伍详情");
    r.push_str("\n发送 '我的讨伐目标' 查看所在队伍目标");
    r
}

/// 查看我的讨伐目标
pub fn cmd_my_team_goals(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let teams = load_all_team_goals(db);

    // Find teams where user is a member
    let my_teams: Vec<&TeamGoal> = teams
        .iter()
        .filter(|t| t.members.iter().any(|m| m.user_id == user_id))
        .collect();

    if my_teams.is_empty() {
        return format!("{}\n您还没有加入任何讨伐队伍。\n\n发送 '讨伐目标' 查看所有队伍", prefix);
    }

    let mut r = format!("{}\n═══ ⭐ 我的讨伐目标 ═══", prefix);
    for team in &my_teams {
        r.push_str(&format!("\n\n▸ 队伍: [{}]", team.team_name));
        let role = team
            .members
            .iter()
            .find(|m| m.user_id == user_id)
            .map(|m| if m.position == "Captain" { "队长" } else { "队员" })
            .unwrap_or("未知");
        r.push_str(&format!(" ({})", role));

        if team.goals.is_empty() {
            r.push_str("\n  暂无讨伐目标");
        } else {
            for map_goal in &team.goals {
                r.push_str(&format!("\n  📍 {}", map_goal.map_name));
                for monster in &map_goal.monsters {
                    // Check progress from Global table
                    let progress = get_monster_kill_progress(db, &team.team_id, &map_goal.map_name, &monster.name);
                    let status = if progress >= monster.target_count {
                        "✅".to_string()
                    } else {
                        format!("{}/{}", progress, monster.target_count)
                    };
                    r.push_str(&format!("\n    · {} ×{}", monster.name, status));
                }
            }
        }
    }
    r
}

/// 查看讨伐进度
pub fn cmd_team_goal_progress(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let teams = load_all_team_goals(db);

    let args = args.trim();

    // Find team by name or user membership
    let team = if args.is_empty() {
        teams.iter().find(|t| t.members.iter().any(|m| m.user_id == user_id))
    } else {
        teams.iter().find(|t| t.team_name.contains(args) || t.team_id == args)
    };

    let team = match team {
        Some(t) => t,
        None => return format!("{}\n找不到队伍。发送 '讨伐目标' 查看所有队伍", prefix),
    };

    let mut r = format!("{}\n═══ 📊 讨伐进度: {} ═══", prefix, team.team_name);
    let mut total_target = 0i32;
    let mut total_done = 0i32;

    if team.goals.is_empty() {
        r.push_str("\n暂无讨伐目标");
        return r;
    }

    for map_goal in &team.goals {
        r.push_str(&format!("\n\n📍 {}", map_goal.map_name));
        for monster in &map_goal.monsters {
            let progress = get_monster_kill_progress(db, &team.team_id, &map_goal.map_name, &monster.name);
            total_target += monster.target_count;
            total_done += progress.min(monster.target_count);

            let pct = if monster.target_count > 0 {
                (progress as f64 / monster.target_count as f64 * 100.0).min(100.0)
            } else {
                100.0
            };

            let bar_len = 10;
            let filled = ((pct / 100.0) * bar_len as f64) as usize;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

            let status = if progress >= monster.target_count { "✅" } else { "" };
            r.push_str(&format!(
                "\n  · {} {} {}/{} ({}%){}",
                monster.name, bar, progress, monster.target_count, pct as i32, status
            ));
        }
    }

    let total_pct = if total_target > 0 {
        total_done as f64 / total_target as f64 * 100.0
    } else {
        0.0
    };

    r.push_str(&format!(
        "\n\n━━━━━━━━━━━━━━━━━━━━\n总进度: {}/{} ({}%)",
        total_done, total_target, total_pct as i32
    ));

    if total_done >= total_target {
        r.push_str("\n🎉 恭喜！所有讨伐目标已完成！");
    }

    r
}

/// 获取怪物击杀进度 (从 Global 表读取)
fn get_monster_kill_progress(db: &Database, team_id: &str, map_name: &str, monster_name: &str) -> i32 {
    let key = format!("{}_{}_{}", team_id, map_name, monster_name);
    db.global_get("team_goal_progress", &key).parse().unwrap_or(0)
}

/// 记录怪物击杀进度 (供战斗系统调用)
pub fn record_monster_kill(db: &Database, user_id: &str, monster_name: &str, map_name: &str) {
    let teams = load_all_team_goals(db);
    for team in &teams {
        // Check if user is in this team
        if !team.members.iter().any(|m| m.user_id == user_id) {
            continue;
        }
        // Check if this monster is a goal
        for map_goal in &team.goals {
            if map_goal.map_name != map_name {
                continue;
            }
            for monster in &map_goal.monsters {
                if monster.name == monster_name {
                    let key = format!("{}_{}_{}", team.team_id, map_name, monster_name);
                    let current: i32 = db.global_get("team_goal_progress", &key).parse().unwrap_or(0);
                    let new_count = current + 1;
                    db.global_set("team_goal_progress", &key, &new_count.to_string());
                }
            }
        }
    }
}

/// 格式化队伍详情
fn format_team_detail(prefix: &str, team: &TeamGoal, _db: &Database, user_id: &str) -> String {
    let mut r = format!("{}\n═══ 🏆 讨伐目标: {} ═══", prefix, team.team_name);

    // Members
    r.push_str("\n\n👥 成员:");
    for m in &team.members {
        let role = if m.position == "Captain" {
            "👑队长"
        } else {
            "队员"
        };
        let tag = if m.user_id == user_id { " ← 您" } else { "" };
        r.push_str(&format!("\n  · {} ({}){}", m.user_id, role, tag));
    }

    // Goals
    if team.goals.is_empty() {
        r.push_str("\n\n📋 暂无讨伐目标");
    } else {
        r.push_str("\n\n📋 讨伐目标:");
        for map_goal in &team.goals {
            r.push_str(&format!("\n📍 {}", map_goal.map_name));
            for monster in &map_goal.monsters {
                let progress = get_monster_kill_progress(_db, &team.team_id, &map_goal.map_name, &monster.name);
                let status = if progress >= monster.target_count {
                    "✅".to_string()
                } else {
                    format!("{}/{}", progress, monster.target_count)
                };
                r.push_str(&format!("\n  · {} ×{}", monster.name, status));
            }
        }
    }

    r.push_str("\n\n发送 '讨伐进度+队伍名' 查看详细进度");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_members() {
        let input = "[123456]\r\nPosition=Captain\r\n[789012]\r\nPosition=Member\r\n[345678]\r\nPosition=Member";
        let members = parse_members(input);
        assert_eq!(members.len(), 3);
        assert_eq!(members[0].user_id, "123456");
        assert_eq!(members[0].position, "Captain");
        assert_eq!(members[1].user_id, "789012");
        assert_eq!(members[1].position, "Member");
    }

    #[test]
    fn test_parse_goals() {
        let input = "[格兰森林]\r\n史莱姆=100\r\n哥布林=300\r\n[莱茵高原]\r\n牛头人=1184";
        let goals = parse_goals(input);
        assert_eq!(goals.len(), 2);
        assert_eq!(goals[0].map_name, "格兰森林");
        assert_eq!(goals[0].monsters.len(), 2);
        assert_eq!(goals[0].monsters[0].name, "史莱姆");
        assert_eq!(goals[0].monsters[0].target_count, 100);
        assert_eq!(goals[1].map_name, "莱茵高原");
        assert_eq!(goals[1].monsters[0].name, "牛头人");
        assert_eq!(goals[1].monsters[0].target_count, 1184);
    }

    #[test]
    fn test_parse_goals_empty() {
        let goals = parse_goals("");
        assert!(goals.is_empty());
    }

    #[test]
    fn test_parse_members_empty() {
        let members = parse_members("");
        assert!(members.is_empty());
    }

    #[test]
    fn test_parse_goals_null_value() {
        let input = "[格兰森林]\r\n哥布林=300\r\n[NULL]";
        let goals = parse_goals(input);
        assert_eq!(goals.len(), 1); // [NULL] has no monsters so not added
        assert_eq!(goals[0].map_name, "格兰森林");
    }
}
