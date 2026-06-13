/// CakeGame 装备锁定系统
/// 锁定装备后无法出售/丢弃/分解，防止误操作丢失珍贵装备
/// 存储: Global 表, section = 'equip_lock', ID = user_id, DATA = 锁定槽位JSON
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 装备槽位列表
const EQUIP_SLOTS: &[(&str, &str)] = &[
    ("武器", SLOT_WEAPON),
    ("头盔", SLOT_HELMET),
    ("铠甲", SLOT_ARMOR),
    ("护腿", SLOT_LEG),
    ("靴子", SLOT_BOOTS),
    ("项链", SLOT_NECKLACE),
    ("戒指", SLOT_RING),
    ("翅膀", SLOT_WING),
    ("时装", SLOT_FASHION),
    ("称号", SLOT_TITLE),
];

/// 从 Global 表读取用户锁定的槽位列表
fn get_locked_slots(db: &Database, user_id: &str) -> Vec<String> {
    let data = db.global_get("equip_lock", user_id);
    if data.is_empty() {
        return Vec::new();
    }
    data.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存锁定槽位列表到 Global 表
fn save_locked_slots(db: &Database, user_id: &str, slots: &[String]) {
    let data = slots.join(",");
    db.global_set("equip_lock", user_id, &data);
}

/// 解析槽位名 → 内部slot key
fn resolve_slot_name(name: &str) -> Option<&'static str> {
    let name_lower = name.trim();
    for (display, key) in EQUIP_SLOTS {
        if display == &name_lower || key == &name_lower {
            return Some(key);
        }
    }
    None
}

/// 检查指定槽位是否被锁定（供其他模块调用）
#[allow(dead_code)]
pub fn is_slot_locked(db: &Database, user_id: &str, slot_key: &str) -> bool {
    let locked = get_locked_slots(db, user_id);
    locked.iter().any(|s| s == slot_key)
}

/// 检查装备栏中某个装备名是否在锁定槽位中（供出售/丢弃/分解检查）
pub fn is_equip_name_locked(db: &Database, user_id: &str, equip_name: &str) -> bool {
    let locked = get_locked_slots(db, user_id);
    if locked.is_empty() {
        return false;
    }
    let equips = db.equip_all(user_id);
    for eq in &equips {
        if eq.name == equip_name && locked.iter().any(|s| s == &eq.slot) {
            return true;
        }
    }
    false
}

/// 锁定装备
pub fn cmd_lock_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let slot_name = args.trim();
    if slot_name.is_empty() {
        let mut out = format!("{}\n🔒 ══════ 装备锁定 ══════ 🔒", prefix);
        out.push_str("\n\n用法: 锁定装备+槽位名");
        out.push_str("\n\n可锁定的槽位:");
        for (display, _) in EQUIP_SLOTS {
            out.push_str(&format!("\n  · {}", display));
        }
        out.push_str("\n\n💡 锁定后该槽位的装备无法出售/丢弃/分解");
        out.push_str("\n💡 使用「解锁装备+槽位名」解除锁定");
        out.push_str("\n💡 使用「查看锁定」查看所有锁定的装备");
        return out;
    }

    let slot_key = match resolve_slot_name(slot_name) {
        Some(k) => k,
        None => {
            return format!(
                "{}\n❌ 未找到槽位「{}」，可选: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, slot_name
            )
        }
    };

    // 检查该槽位是否有装备
    let equips = db.equip_all(user_id);
    let has_equip = equips.iter().any(|e| e.slot == slot_key);
    if !has_equip {
        return format!("{}\n❌ {} 槽位没有装备，无需锁定。", prefix, slot_name);
    }

    let mut locked = get_locked_slots(db, user_id);
    if locked.iter().any(|s| s == slot_key) {
        return format!("{}\n⚠️ {} 已经处于锁定状态了。", prefix, slot_name);
    }

    locked.push(slot_key.to_string());
    save_locked_slots(db, user_id, &locked);

    let equip_name = equips
        .iter()
        .find(|e| e.slot == slot_key)
        .map(|e| e.name.as_str())
        .unwrap_or("未知");
    format!(
        "{}\n🔒 已锁定 {} [{}]\n💡 该装备现在无法出售/丢弃/分解\n💡 使用「解锁装备+{}」解除锁定",
        prefix, slot_name, equip_name, slot_name
    )
}

/// 解锁装备
pub fn cmd_unlock_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let slot_name = args.trim();
    if slot_name.is_empty() {
        return format!("{}\n请指定要解锁的槽位名。\n用法: 解锁装备+槽位名", prefix);
    }

    let slot_key = match resolve_slot_name(slot_name) {
        Some(k) => k,
        None => return format!("{}\n❌ 未找到槽位「{}」", prefix, slot_name),
    };

    let mut locked = get_locked_slots(db, user_id);
    if !locked.iter().any(|s| s == slot_key) {
        return format!("{}\n⚠️ {} 并未处于锁定状态。", prefix, slot_name);
    }

    locked.retain(|s| s != slot_key);
    save_locked_slots(db, user_id, &locked);

    let equips = db.equip_all(user_id);
    let equip_name = equips
        .iter()
        .find(|e| e.slot == slot_key)
        .map(|e| e.name.as_str())
        .unwrap_or("未知");
    format!(
        "{}\n🔓 已解锁 {} [{}]\n⚠️ 该装备现在可以被出售/丢弃/分解",
        prefix, slot_name, equip_name
    )
}

/// 查看所有锁定的装备
pub fn cmd_view_locks(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let locked = get_locked_slots(db, user_id);
    let equips = db.equip_all(user_id);

    let mut out = format!("{}\n🔒 ══════ 锁定装备 ══════ 🔒", prefix);

    if locked.is_empty() {
        out.push_str("\n\n当前没有锁定任何装备。");
        out.push_str("\n💡 使用「锁定装备+槽位名」锁定装备");
        return out;
    }

    let mut lock_count = 0;
    for (display, key) in EQUIP_SLOTS {
        if locked.iter().any(|s| s == key) {
            lock_count += 1;
            let equip_name = equips
                .iter()
                .find(|e| e.slot == *key)
                .map(|e| e.name.as_str())
                .unwrap_or("（空）");
            out.push_str(&format!("\n🔒 {} — [{}]", display, equip_name));
        }
    }

    out.push_str(&format!("\n\n📊 共锁定 {} 个槽位", lock_count));
    out.push_str("\n💡 使用「解锁装备+槽位名」解除锁定");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_slot_name() {
        assert_eq!(resolve_slot_name("武器"), Some(SLOT_WEAPON));
        assert_eq!(resolve_slot_name("头盔"), Some(SLOT_HELMET));
        assert_eq!(resolve_slot_name("铠甲"), Some(SLOT_ARMOR));
        assert_eq!(resolve_slot_name("护腿"), Some(SLOT_LEG));
        assert_eq!(resolve_slot_name("靴子"), Some(SLOT_BOOTS));
        assert_eq!(resolve_slot_name("项链"), Some(SLOT_NECKLACE));
        assert_eq!(resolve_slot_name("戒指"), Some(SLOT_RING));
        assert_eq!(resolve_slot_name("翅膀"), Some(SLOT_WING));
        assert_eq!(resolve_slot_name("时装"), Some(SLOT_FASHION));
        assert_eq!(resolve_slot_name("称号"), Some(SLOT_TITLE));
        assert_eq!(resolve_slot_name("不存在"), None);
        assert_eq!(resolve_slot_name(""), None);
    }

    #[test]
    fn test_equip_slots_count() {
        assert_eq!(EQUIP_SLOTS.len(), 10);
    }

    #[test]
    fn test_slot_key_uniqueness() {
        let mut keys: Vec<&str> = EQUIP_SLOTS.iter().map(|(_, k)| *k).collect();
        let original_len = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), original_len, "slot keys must be unique");
    }
}
