/// CakeGame 玩家设置系统
/// 允许玩家配置个性化游戏体验：自动战斗/通知/显示模式/隐私
/// 数据存储: Global 表 SECTION='player_settings'
/// 来源: 新增功能，原版无此系统
use crate::db::Database;
use crate::user;

// ==================== 设置定义 ====================

/// 单个设置项定义
struct SettingDef {
    key: &'static str,
    name: &'static str,
    emoji: &'static str,
    default: &'static str,
    options: &'static [&'static str],
    description: &'static str,
}

const SETTINGS: &[SettingDef] = &[
    SettingDef {
        key: "auto_attack",
        name: "自动攻击",
        emoji: "⚔️",
        default: "开启",
        options: &["开启", "关闭"],
        description: "战斗后自动继续攻击同一目标",
    },
    SettingDef {
        key: "auto_loot",
        name: "自动拾取",
        emoji: "🎒",
        default: "开启",
        options: &["开启", "关闭"],
        description: "击败怪物后自动拾取掉落物品",
    },
    SettingDef {
        key: "damage_display",
        name: "伤害显示",
        emoji: "💥",
        default: "详细",
        options: &["简略", "详细", "极简"],
        description: "战斗中伤害数字的显示模式",
    },
    SettingDef {
        key: "notify_mode",
        name: "通知模式",
        emoji: "🔔",
        default: "全部",
        options: &["全部", "重要", "关闭"],
        description: "接收系统通知的范围",
    },
    SettingDef {
        key: "privacy_mode",
        name: "隐私模式",
        emoji: "🔒",
        default: "公开",
        options: &["公开", "仅好友", "隐身"],
        description: "其他玩家是否能看到你的在线状态",
    },
    SettingDef {
        key: "auto_use_potion",
        name: "自动用药",
        emoji: "💊",
        default: "30%",
        options: &["10%", "20%", "30%", "50%", "关闭"],
        description: "生命低于阈值时自动使用药水",
    },
    SettingDef {
        key: "combat_style_lock",
        name: "风格锁定",
        emoji: "🔐",
        default: "关闭",
        options: &["开启", "关闭"],
        description: "锁定战斗风格防止误切换",
    },
    SettingDef {
        key: "font_size",
        name: "显示大小",
        emoji: "🔤",
        default: "标准",
        options: &["紧凑", "标准", "宽松"],
        description: "界面信息显示的紧凑程度",
    },
];

/// 获取设置值（带默认值回退）
fn get_setting(db: &Database, user_id: &str, key: &str) -> String {
    let section = format!("player_settings_{}", user_id);
    let val = db.global_get(&section, key);
    if val.is_empty() {
        SETTINGS
            .iter()
            .find(|s| s.key == key)
            .map(|s| s.default.to_string())
            .unwrap_or_default()
    } else {
        val
    }
}

/// 设置某个配置项
fn set_setting(db: &Database, user_id: &str, key: &str, value: &str) {
    let section = format!("player_settings_{}", user_id);
    db.global_set(&section, key, value);
}

/// 查找设置定义
fn find_setting(name: &str) -> Option<&'static SettingDef> {
    // 精确匹配 key
    if let Some(s) = SETTINGS.iter().find(|s| s.key == name) {
        return Some(s);
    }
    // 精确匹配中文名
    if let Some(s) = SETTINGS.iter().find(|s| s.name == name) {
        return Some(s);
    }
    // 模糊匹配
    SETTINGS.iter().find(|s| s.name.contains(name) || name.contains(s.name))
}

// ==================== 命令实现 ====================

/// 查看设置 — 显示所有个性化配置
pub fn cmd_view_settings(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let mut out = format!("{}\n═══ ⚙️ 玩家设置 ═══", prefix);
    out.push_str("\n个性化游戏体验配置\n");

    for s in SETTINGS {
        let val = get_setting(db, user_id, s.key);
        out.push_str(&format!("\n{} {}: {}", s.emoji, s.name, val));
    }

    out.push_str("\n\n💡 修改设置: 设置+名称+值");
    out.push_str("\n📋 示例: 设置+自动攻击+关闭");
    out.push_str("\n🔄 重置设置: 重置设置");

    out
}

/// 设置偏好 — 修改单个配置项
pub fn cmd_set_preference(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        let mut out = format!("{}\n═══ ⚙️ 可用设置列表 ═══", prefix);
        for s in SETTINGS {
            let val = get_setting(db, user_id, s.key);
            out.push_str(&format!("\n\n{} {} [当前: {}]", s.emoji, s.name, val));
            out.push_str(&format!("\n   {}", s.description));
            out.push_str(&format!("\n   可选: {}", s.options.join(" / ")));
        }
        out.push_str("\n\n💡 用法: 设置+名称+值");
        return out;
    }

    // 解析 "名称+值" 格式
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return format!("{}\n❌ 格式错误！用法: 设置+名称+值\n示例: 设置+自动攻击+关闭", prefix);
    }

    let setting_name = parts[0];
    let value = parts[1];

    let setting = match find_setting(setting_name) {
        Some(s) => s,
        None => {
            let mut out = format!("{}\n❌ 未找到设置「{}」\n\n📋 可用设置:", prefix, setting_name);
            for s in SETTINGS {
                out.push_str(&format!("\n  {} {}", s.emoji, s.name));
            }
            return out;
        }
    };

    // 验证值是否合法
    if !setting.options.contains(&value) {
        return format!(
            "{}\n❌ {} 不是 {} 的有效值！\n可选: {}",
            prefix,
            value,
            setting.name,
            setting.options.join(" / ")
        );
    }

    let old_val = get_setting(db, user_id, setting.key);
    set_setting(db, user_id, setting.key, value);

    format!(
        "{}\n✅ 设置已更新！\n\n{} {}: {} → {}",
        prefix, setting.emoji, setting.name, old_val, value
    )
}

/// 重置设置 — 恢复所有默认值
pub fn cmd_reset_settings(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let section = format!("player_settings_{}", user_id);
    for s in SETTINGS {
        db.global_set(&section, s.key, s.default);
    }

    let mut out = format!("{}\n✅ 所有设置已恢复默认值！", prefix);
    for s in SETTINGS {
        out.push_str(&format!("\n{} {}: {}", s.emoji, s.name, s.default));
    }
    out
}

// ==================== 内部API ====================

/// 检查是否启用自动攻击
#[allow(dead_code)]
pub fn is_auto_attack_enabled(db: &Database, user_id: &str) -> bool {
    get_setting(db, user_id, "auto_attack") == "开启"
}

/// 检查是否启用自动拾取
#[allow(dead_code)]
pub fn is_auto_loot_enabled(db: &Database, user_id: &str) -> bool {
    get_setting(db, user_id, "auto_loot") == "开启"
}

/// 获取自动用药阈值 (返回百分比，0表示关闭)
#[allow(dead_code)]
pub fn get_auto_potion_threshold(db: &Database, user_id: &str) -> i32 {
    match get_setting(db, user_id, "auto_use_potion").as_str() {
        "10%" => 10,
        "20%" => 20,
        "30%" => 30,
        "50%" => 50,
        _ => 0,
    }
}

/// 获取伤害显示模式
#[allow(dead_code)]
pub fn get_damage_display_mode(db: &Database, user_id: &str) -> String {
    get_setting(db, user_id, "damage_display")
}

/// 获取通知模式
#[allow(dead_code)]
pub fn get_notify_mode(db: &Database, user_id: &str) -> String {
    get_setting(db, user_id, "notify_mode")
}

/// 检查是否隐身
#[allow(dead_code)]
pub fn is_privacy_mode(db: &Database, user_id: &str) -> bool {
    get_setting(db, user_id, "privacy_mode") != "公开"
}

/// 检查战斗风格是否锁定
#[allow(dead_code)]
pub fn is_combat_style_locked(db: &Database, user_id: &str) -> bool {
    get_setting(db, user_id, "combat_style_lock") == "开启"
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setting_count() {
        assert_eq!(SETTINGS.len(), 8, "应有8个设置项");
    }

    #[test]
    fn test_setting_keys_unique() {
        let mut keys: Vec<&str> = SETTINGS.iter().map(|s| s.key).collect();
        let before = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(before, keys.len(), "设置key不应重复");
    }

    #[test]
    fn test_setting_names_unique() {
        let mut names: Vec<&str> = SETTINGS.iter().map(|s| s.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "设置名不应重复");
    }

    #[test]
    fn test_setting_defaults_in_options() {
        for s in SETTINGS {
            assert!(
                s.options.contains(&s.default),
                "{}: 默认值 '{}' 不在选项列表中",
                s.name,
                s.default
            );
        }
    }

    #[test]
    fn test_find_setting_by_key() {
        let s = find_setting("auto_attack").unwrap();
        assert_eq!(s.name, "自动攻击");
    }

    #[test]
    fn test_find_setting_by_name() {
        let s = find_setting("自动攻击").unwrap();
        assert_eq!(s.key, "auto_attack");
    }

    #[test]
    fn test_find_setting_fuzzy() {
        let s = find_setting("攻击");
        assert!(s.is_some(), "模糊匹配'攻击'应找到自动攻击");
    }

    #[test]
    fn test_find_setting_not_found() {
        assert!(find_setting("不存在的设置").is_none());
    }

    #[test]
    fn test_all_options_non_empty() {
        for s in SETTINGS {
            assert!(!s.options.is_empty(), "{}: 选项不应为空", s.name);
            for opt in s.options {
                assert!(!opt.is_empty(), "{}: 有空选项", s.name);
            }
        }
    }

    #[test]
    fn test_setting_emojis() {
        for s in SETTINGS {
            assert!(!s.emoji.is_empty(), "{}: emoji不应为空", s.name);
        }
    }
}
