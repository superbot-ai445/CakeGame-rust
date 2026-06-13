/// 地图资源总览系统
/// 显示地图中的怪物、NPC、植物等资源信息
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 从 Config_Map 的 Monster 字段解析怪物名称列表
fn parse_map_monsters(monster_field: &str) -> Vec<String> {
    let mut monsters = Vec::new();
    for line in monster_field.lines() {
        let line = line.trim().replace('\r', "");
        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            if !name.is_empty() {
                monsters.push(name.to_string());
            }
        }
    }
    monsters
}

/// 解析地图通道（连接的其他地图）
fn parse_connections(up: &str, down: &str, left: &str, right: &str) -> Vec<(&'static str, String)> {
    let mut conns = Vec::new();
    if up != "[NULL]" && !up.is_empty() {
        conns.push(("⬆️ 上方", up.to_string()));
    }
    if down != "[NULL]" && !down.is_empty() {
        conns.push(("⬇️ 下方", down.to_string()));
    }
    if left != "[NULL]" && !left.is_empty() {
        conns.push(("⬅️ 左侧", left.to_string()));
    }
    if right != "[NULL]" && !right.is_empty() {
        conns.push(("➡️ 右侧", right.to_string()));
    }
    conns
}

/// 地图资源信息结构
struct MapResourceInfo {
    name: String,
    level_range: String,
    intro: String,
    is_safe: bool,
    monsters: Vec<String>,
    npcs: Vec<String>,
    connections: Vec<(&'static str, String)>,
}

/// 获取地图资源信息
fn get_map_info(db: &Database, map_name: &str) -> Option<MapResourceInfo> {
    let mut result = None;
    let _ = db.query_row(
        "SELECT Name, LV, Introduce, Security, Monster, UP, Down, Left, Right, LV_UP \
         FROM Config_Map WHERE Name = ?1",
        &[map_name],
        |row| {
            let name: String = row.get(0).unwrap_or_default();
            let lv: String = row.get(1).unwrap_or_default();
            let intro: String = row.get(2).unwrap_or_default();
            let security: String = row.get(3).unwrap_or_default();
            let monster: String = row.get(4).unwrap_or_default();
            let up: String = row.get(5).unwrap_or_default();
            let down: String = row.get(6).unwrap_or_default();
            let left: String = row.get(7).unwrap_or_default();
            let right: String = row.get(8).unwrap_or_default();
            let lv_up: String = row.get(9).unwrap_or_default();

            let level_range = if lv_up.is_empty() || lv_up == lv {
                format!("Lv.{}", lv)
            } else {
                format!("Lv.{}-{}", lv, lv_up)
            };

            result = Some(MapResourceInfo {
                name,
                level_range,
                intro,
                is_safe: security.to_uppercase() == "TRUE",
                monsters: parse_map_monsters(&monster),
                npcs: Vec::new(), // filled later
                connections: parse_connections(&up, &down, &left, &right),
            });
            Ok(())
        },
    );
    result
}

/// 获取地图中的 NPC 列表
fn get_map_npcs(db: &Database, map_name: &str) -> Vec<String> {
    db.query_rows(
        "SELECT Name FROM Ext_NPC_Info WHERE Location = ?1",
        &[map_name],
        |row| {
            let name: String = row.get(0).unwrap_or_default();
            Ok(name)
        },
    )
}

/// 查看地图资源总览
/// 用法: 地图资源 (当前地图) 或 地图资源+地图名 (指定地图)
pub fn cmd_map_resource(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 确定目标地图
    let target_map = if args.is_empty() {
        db.read_basic(user_id, ITEM_LOCATION)
    } else {
        args.trim().to_string()
    };

    if target_map.is_empty() || target_map == "无" {
        return format!("{}\n❌ 无法确定地图位置！请先进入地图或指定地图名。", prefix);
    }

    let mut info = match get_map_info(db, &target_map) {
        Some(info) => info,
        None => return format!("{}\n❌ 地图「{}」不存在！请检查地图名。", prefix, target_map),
    };

    // 获取 NPC 列表
    info.npcs = get_map_npcs(db, &info.name);

    // 构建输出
    let mut result = format!("═══ 🗺️ {} 资源总览 ═══\n", info.name);
    result.push_str(&format!("📊 等级范围: {}\n", info.level_range));
    result.push_str(&format!("📖 介绍: {}\n", info.intro));
    result.push_str(&format!("{} 安全区\n", if info.is_safe { "🛡️" } else { "⚔️" }));

    // 连接地图
    if !info.connections.is_empty() {
        result.push_str(&format!("\n📍 通往 ({}个):\n", info.connections.len()));
        for (dir, name) in &info.connections {
            result.push_str(&format!("  {} {}\n", dir, name));
        }
    }

    // 怪物信息
    if info.monsters.is_empty() {
        result.push_str("\n🐉 怪物: 无\n");
    } else {
        result.push_str(&format!("\n🐉 怪物 ({}种):\n", info.monsters.len()));
        for (i, name) in info.monsters.iter().enumerate() {
            // 尝试从 Config_Monster 获取怪物等级
            let monster_info = get_monster_brief(db, name);
            if monster_info.is_empty() {
                result.push_str(&format!("  {}. {}\n", i + 1, name));
            } else {
                result.push_str(&format!("  {}. {} {}\n", i + 1, name, monster_info));
            }
        }
    }

    // NPC 信息
    if info.npcs.is_empty() {
        result.push_str("\n👤 NPC: 无\n");
    } else {
        result.push_str(&format!("\n👤 NPC ({}个):\n", info.npcs.len()));
        for (i, name) in info.npcs.iter().enumerate() {
            result.push_str(&format!("  {}. {}\n", i + 1, name));
        }
        result.push_str("💡 发送「查看NPC」查看NPC详细信息\n");
    }

    result.push_str(&format!(
        "\n💡 发送「搜索怪物」查看怪物详情\n\
         💡 发送「进入+方向」前往其他地图\n\
         💡 当前位置: {}",
        info.name
    ));

    format!("{}\n{}", prefix, result)
}

/// 获取怪物简介（等级/HP/类型）
fn get_monster_brief(db: &Database, monster_name: &str) -> String {
    let mut brief = String::new();
    let _ = db.query_row(
        "SELECT Level, HP, MonsterType FROM Config_Monster WHERE Name = ?1",
        &[monster_name],
        |row| {
            let level: String = row.get(0).unwrap_or_default();
            let hp: String = row.get(1).unwrap_or_default();
            let mtype: String = row.get(2).unwrap_or_default();
            let type_emoji = match mtype.as_str() {
                "BOSS" => "👑",
                _ => "",
            };
            brief = format!("(Lv.{} HP:{} {})", level, hp, type_emoji);
            Ok(())
        },
    );
    brief
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_map_monsters_basic() {
        let input = "[哥布林]\n[史莱姆]\n[骷髅兵]";
        let monsters = parse_map_monsters(input);
        assert_eq!(monsters, vec!["哥布林", "史莱姆", "骷髅兵"]);
    }

    #[test]
    fn test_parse_map_monsters_empty() {
        let monsters = parse_map_monsters("");
        assert!(monsters.is_empty());
    }

    #[test]
    fn test_parse_map_monsters_empty_brackets() {
        let monsters = parse_map_monsters("[]\n[哥布林]");
        assert_eq!(monsters, vec!["哥布林"]);
    }

    #[test]
    fn test_parse_map_monsters_with_carriage_return() {
        let monsters = parse_map_monsters("[哥布林]\r\n[史莱姆]");
        assert_eq!(monsters, vec!["哥布林", "史莱姆"]);
    }

    #[test]
    fn test_parse_map_monsters_no_brackets() {
        let monsters = parse_map_monsters("哥布林\n史莱姆");
        assert!(monsters.is_empty());
    }

    #[test]
    fn test_parse_connections_all() {
        let conns = parse_connections("北方森林", "南方沙漠", "东方海洋", "西方山脉");
        assert_eq!(conns.len(), 4);
        assert_eq!(conns[0].1, "北方森林");
        assert_eq!(conns[1].1, "南方沙漠");
        assert_eq!(conns[2].1, "东方海洋");
        assert_eq!(conns[3].1, "西方山脉");
    }

    #[test]
    fn test_parse_connections_partial() {
        let conns = parse_connections("北方森林", "[NULL]", "", "西方山脉");
        assert_eq!(conns.len(), 2);
        assert_eq!(conns[0].1, "北方森林");
        assert_eq!(conns[1].1, "西方山脉");
    }

    #[test]
    fn test_parse_connections_none() {
        let conns = parse_connections("[NULL]", "[NULL]", "[NULL]", "[NULL]");
        assert!(conns.is_empty());
    }

    #[test]
    fn test_parse_connections_empty() {
        let conns = parse_connections("", "", "", "");
        assert!(conns.is_empty());
    }
}
