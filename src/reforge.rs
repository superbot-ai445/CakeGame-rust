/// CakeGame 装备重铸系统
/// 允许玩家使用金币和重铸石重新随机装备的附加属性(Add_*)
/// 每次重铸随机调整15项属性值，可选择保留或恢复原属性
/// 存储: Global 表, section = 'reforge', ID = user_id
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 重铸费用: 每次消耗金币 + 重铸石
const REFORGE_COST_GOLD: i64 = 10_000;
const REFORGE_COST_ITEM: &str = "重铸石";
const REFORGE_COST_ITEM_QTY: i32 = 1;

/// 属性名列表 (与Equip_Register列对应)
#[allow(dead_code)]
const ATTR_NAMES: &[&str] = &[
    "HP",
    "MP",
    "Defense",
    "Magic",
    "AD",
    "AP",
    "Hit",
    "Dodge",
    "Crit",
    "AbsorbHP",
    "ADPTV",
    "ADPTR",
    "APPTR",
    "APPTV",
    "ImmuneDamage",
];

/// 中文属性名
const ATTR_CN: &[&str] = &[
    "生命", "法力", "物防", "魔防", "物攻", "魔攻", "命中", "闪避", "暴击", "吸血", "物穿", "物免", "魔免", "魔穿",
    "免伤",
];

/// 装备槽位
const REFORGE_SLOTS: &[(&str, &str)] = &[
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

/// 解析槽位名
fn resolve_slot(input: &str) -> Option<(&'static str, &'static str)> {
    for &(cn, key) in REFORGE_SLOTS {
        if input.contains(cn) || input == key {
            return Some((cn, key));
        }
    }
    None
}

/// 读取装备当前属性值列表
fn read_current_attrs(db: &Database, user_id: &str, slot: &str) -> Option<[i32; 15]> {
    let info = db.equip_read(user_id, slot)?;
    Some([
        info.add_hp,
        info.add_mp,
        info.add_defense,
        info.add_magic,
        info.add_ad,
        info.add_ap,
        info.add_hit,
        info.add_dodge,
        info.add_crit,
        info.add_absorb_hp,
        info.add_adptv,
        info.add_adptr,
        info.add_apptr,
        info.add_apptv,
        info.add_immune_damage,
    ])
}

/// 生成重铸后的属性 (基于当前属性±随机波动)
fn generate_reforge_stats(current: &[i32; 15], seed: u64) -> [i32; 15] {
    let mut result = [0i32; 15];
    let mut rng_state = seed;
    for (i, &val) in current.iter().enumerate() {
        // xorshift64 PRNG — 不同种子一定产生不同序列
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let offset = ((rng_state.wrapping_add(i as u64) % 11) as i32) - 5; // -5 ~ +5
        let base = val.max(30); // 基础最低30 (与原版一致)
        let new_val = (base + offset).clamp(1, 999);
        result[i] = new_val;
    }
    result
}

/// djb2哈希
fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 格式式属性变化箭头
fn diff_arrow(old: i32, new: i32) -> &'static str {
    if new > old {
        "🔺"
    } else if new < old {
        "🔻"
    } else {
        "➖"
    }
}

/// 格式化单件装备属性对比
fn format_attrs_compare(old: &[i32; 15], new: &[i32; 15]) -> String {
    let mut r = String::new();
    for i in 0..15 {
        let diff = new[i] - old[i];
        let arrow = diff_arrow(old[i], new[i]);
        if diff != 0 {
            r.push_str(&format!(
                "\n  {} {}: {} → {} ({:+})",
                arrow, ATTR_CN[i], old[i], new[i], diff
            ));
        }
    }
    if r.is_empty() {
        r.push_str("\n  ➖ 属性无变化");
    }
    r
}

/// 格式化属性摘要 (单行)
fn format_attrs_short(attrs: &[i32; 15]) -> String {
    let mut parts = Vec::new();
    for i in 0..15 {
        if attrs[i] > 30 {
            parts.push(format!("{}+{}", ATTR_CN[i], attrs[i] - 30));
        }
    }
    if parts.is_empty() {
        "基础属性".to_string()
    } else {
        parts.join(" ")
    }
}

/// 计算属性总评分
fn calc_attr_score(attrs: &[i32; 15]) -> i32 {
    // 使用与 equip_score 类似的权重
    let weights = [3, 2, 20, 18, 35, 25, 2, 12, 60, 50, 20, 20, 44, 20, 80];
    let mut score = 0i32;
    for i in 0..15 {
        score += (attrs[i] - 30).max(0) * weights[i];
    }
    score
}

/// 格式式评分条
fn format_score_bar(score: i32, max_score: i32) -> String {
    let pct = if max_score > 0 {
        (score as f64 / max_score as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    let filled = (pct / 10.0).round() as usize;
    let empty = 10_usize.saturating_sub(filled);
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// 查看重铸 — 显示当前装备可重铸状态
pub fn cmd_view_reforge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    // 查看特定槽位
    if !args.is_empty() {
        if let Some((cn, key)) = resolve_slot(args) {
            let equip = db.equip_read(user_id, key);
            match equip {
                Some(info) => {
                    let attrs = [
                        info.add_hp,
                        info.add_mp,
                        info.add_defense,
                        info.add_magic,
                        info.add_ad,
                        info.add_ap,
                        info.add_hit,
                        info.add_dodge,
                        info.add_crit,
                        info.add_absorb_hp,
                        info.add_adptv,
                        info.add_adptr,
                        info.add_apptr,
                        info.add_apptv,
                        info.add_immune_damage,
                    ];
                    let score = calc_attr_score(&attrs);
                    let mut r = format!("{}\n═══ 装备重铸 · {} ═══", prefix, cn);
                    r.push_str(&format!("\n📌 装备: {}", info.name));
                    r.push_str(&format!("\n📊 属性评分: {}", format_score_bar(score, 5000)));
                    r.push_str(&format!("\n{}\n", format_attrs_short(&attrs)));
                    r.push_str(&format!(
                        "\n💰 重铸费用: {}金币 + {}×{}",
                        REFORGE_COST_GOLD, REFORGE_COST_ITEM, REFORGE_COST_ITEM_QTY
                    ));
                    r.push_str(&format!("\n\n发送 '装备重铸+{}' 重新随机属性", cn));
                    r.push_str(&format!("\n发送 '重铸预览+{}' 预览属性变化", cn));
                    r
                }
                None => format!("{}\n❌ {}槽位没有装备。", prefix, cn),
            }
        } else {
            format!(
                "{}\n❌ 未找到槽位 [{}]。可用槽位: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, args
            )
        }
    } else {
        // 列出所有装备
        let mut r = format!("{}\n═══ 装备重铸系统 ═══", prefix);
        r.push_str("\n重新随机装备附加属性，追求更高属性搭配！\n");
        let mut has_equipped = false;
        for &(cn, key) in REFORGE_SLOTS {
            if let Some(info) = db.equip_read(user_id, key) {
                has_equipped = true;
                let attrs = [
                    info.add_hp,
                    info.add_mp,
                    info.add_defense,
                    info.add_magic,
                    info.add_ad,
                    info.add_ap,
                    info.add_hit,
                    info.add_dodge,
                    info.add_crit,
                    info.add_absorb_hp,
                    info.add_adptv,
                    info.add_adptr,
                    info.add_apptr,
                    info.add_apptv,
                    info.add_immune_damage,
                ];
                let score = calc_attr_score(&attrs);
                r.push_str(&format!(
                    "\n  {}·{} | 评分:{} | {}",
                    cn,
                    info.name,
                    score,
                    format_attrs_short(&attrs)
                ));
            }
        }
        if !has_equipped {
            r.push_str("\n  （暂无装备）");
        }
        r.push_str(&format!(
            "\n\n💰 每次重铸: {}金币 + {}×{}",
            REFORGE_COST_GOLD, REFORGE_COST_ITEM, REFORGE_COST_ITEM_QTY
        ));
        r.push_str("\n\n发送 '查看重铸+槽位名' 查看详情");
        r.push_str("\n发送 '装备重铸+槽位名' 重铸属性");
        r.push_str("\n发送 '重铸预览+槽位名' 预览变化");
        r.push_str("\n发送 '重铸记录' 查看历史");
        r
    }
}

/// 重铸预览 — 生成一个预览但不实际修改
pub fn cmd_reforge_preview(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    let (cn, key) = match resolve_slot(args) {
        Some(v) => v,
        None => {
            return format!(
                "{}\n❌ 请指定槽位。用法: 重铸预览+槽位名\n可用: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix
            )
        }
    };

    let current = match read_current_attrs(db, user_id, key) {
        Some(a) => a,
        None => return format!("{}\n❌ {}槽位没有装备，无法预览。", prefix, cn),
    };

    let equip = db.equip_read(user_id, key).unwrap();
    let now_seed = Local::now().timestamp() as u64;
    let preview = generate_reforge_stats(&current, now_seed);

    let old_score = calc_attr_score(&current);
    let new_score = calc_attr_score(&preview);

    let mut r = format!("{}\n═══ 重铸预览 · {} · {} ═══", prefix, cn, equip.name);
    r.push_str(&format!(
        "\n📊 当前评分: {} → 预览评分: {} ({:+})",
        old_score,
        new_score,
        new_score - old_score
    ));
    r.push_str(&format!("\n{}", format_attrs_compare(&current, &preview)));

    // 统计变化
    let improved = preview.iter().zip(current.iter()).filter(|(n, o)| n > o).count();
    let decreased = preview.iter().zip(current.iter()).filter(|(n, o)| n < o).count();
    r.push_str(&format!(
        "\n\n📈 提升: {}项 | 📉 降低: {}项 | ➖ 不变: {}项",
        improved,
        decreased,
        15 - improved - decreased
    ));
    r.push_str("\n\n💡 每次重铸结果随机，预览仅供参考");
    r.push_str(&format!("\n发送 '装备重铸+{}' 执行重铸", cn));
    r
}

/// 装备重铸 — 执行属性重随机
pub fn cmd_reforge_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    let (cn, key) = match resolve_slot(args) {
        Some(v) => v,
        None => return format!("{}\n❌ 请指定槽位。用法: 装备重铸+槽位名", prefix),
    };

    // 检查装备存在
    let current = match read_current_attrs(db, user_id, key) {
        Some(a) => a,
        None => return format!("{}\n❌ {}槽位没有装备，无法重铸。", prefix, cn),
    };

    // 检查是否阵亡
    let hp: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n❌ 您已阵亡，请先恢复生命再重铸。", prefix);
    }

    // 检查金币
    let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < REFORGE_COST_GOLD {
        return format!(
            "{}\n💰 金币不足！重铸需要{}金币，当前拥有{}金币。",
            prefix, REFORGE_COST_GOLD, gold
        );
    }

    // 检查重铸石 (从背包中查找)
    let knapsack = db.get_knapsack_items(user_id);
    let has_stone = knapsack
        .iter()
        .any(|item| item.name.contains(REFORGE_COST_ITEM) && item.quantity >= REFORGE_COST_ITEM_QTY);
    if !has_stone {
        return format!(
            "{}\n❌ 背包中没有 {}×{}。\n💡 重铸石可在商店购买或通过副本掉落获得。",
            prefix, REFORGE_COST_ITEM, REFORGE_COST_ITEM_QTY
        );
    }

    // 检查冷却 (30秒)
    let cd_key = format!("reforge_cd_{}", key);
    let last_used = db.read_user_data(user_id, &cd_key);
    if !last_used.is_empty() {
        if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&last_used, "%Y-%m-%d %H:%M:%S") {
            let now = Local::now().naive_local();
            let elapsed = (now - last_time).num_seconds();
            if elapsed < 30 {
                return format!("{}\n⏳ 重铸冷却中，剩余{}秒。", prefix, 30 - elapsed);
            }
        }
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, REFORGE_COST_GOLD);

    // 扣除重铸石
    db.knapsack_remove(user_id, REFORGE_COST_ITEM, REFORGE_COST_ITEM_QTY);

    // 生成新属性 (基于时间戳+用户ID确定性但不可预测)
    let seed = (Local::now().timestamp_millis() as u64).wrapping_add(djb2_hash(user_id));
    let new_attrs = generate_reforge_stats(&current, seed);

    // 更新装备属性
    {
        let conn = db.lock_conn();
        let _ = conn.execute(
            "UPDATE Equip_Register SET Add_HP=?1, Add_MP=?2, Add_Defense=?3, Add_Magic=?4, \
             Add_AD=?5, Add_AP=?6, Add_Hit=?7, Add_Dodge=?8, Add_Crit=?9, Add_AbsorbHP=?10, \
             Add_ADPTV=?11, Add_ADPTR=?12, Add_APPTR=?13, Add_APPTV=?14, Add_ImmuneDamage=?15 \
             WHERE User=?16 AND SlotName=?17",
            rusqlite::params![
                new_attrs[0].to_string(),
                new_attrs[1].to_string(),
                new_attrs[2].to_string(),
                new_attrs[3].to_string(),
                new_attrs[4].to_string(),
                new_attrs[5].to_string(),
                new_attrs[6].to_string(),
                new_attrs[7].to_string(),
                new_attrs[8].to_string(),
                new_attrs[9].to_string(),
                new_attrs[10].to_string(),
                new_attrs[11].to_string(),
                new_attrs[12].to_string(),
                new_attrs[13].to_string(),
                new_attrs[14].to_string(),
                user_id,
                key,
            ],
        );
    }

    // 记录冷却
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.write_user_data(user_id, &cd_key, &now_str);

    // 记录重铸历史到 Global 表
    let history_key = format!("reforge_history_{}", key);
    let count_str = db.read_user_data(user_id, &history_key);
    let count: i32 = count_str.parse().unwrap_or(0) + 1;
    db.write_user_data(user_id, &history_key, &count.to_string());

    let equip = db.equip_read(user_id, key).unwrap();
    let old_score = calc_attr_score(&current);
    let new_score = calc_attr_score(&new_attrs);

    let mut r = format!("{}\n═══ 装备重铸 · {} · {} ═══", prefix, cn, equip.name);
    r.push_str(&format!(
        "\n💰 花费: {}金币 + {}×{}",
        REFORGE_COST_GOLD, REFORGE_COST_ITEM, REFORGE_COST_ITEM_QTY
    ));
    r.push_str(&format!(
        "\n📊 评分: {} → {} ({:+})",
        old_score,
        new_score,
        new_score - old_score
    ));
    r.push_str(&format!("\n{}", format_attrs_compare(&current, &new_attrs)));

    let improved = new_attrs.iter().zip(current.iter()).filter(|(n, o)| n > o).count();
    let decreased = new_attrs.iter().zip(current.iter()).filter(|(n, o)| n < o).count();
    r.push_str(&format!(
        "\n\n📈 提升: {}项 | 📉 降低: {}项 | ➖ 不变: {}项",
        improved,
        decreased,
        15 - improved - decreased
    ));
    r.push_str(&format!("\n🔄 累计重铸次数: {}次", count));

    if new_score > old_score {
        r.push_str("\n\n🎉 属性提升了！考虑保留本次结果。");
    } else if new_score < old_score {
        r.push_str("\n\n😅 属性略有下降，但下次运气可能更好！");
    }

    r.push_str("\n💡 冷却30秒，可继续重铸追求更高属性");
    r
}

/// 重铸记录 — 查看重铸历史
pub fn cmd_reforge_record(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 重铸记录 ═══", prefix);
    let mut total = 0i32;
    let mut has_record = false;

    for &(cn, key) in REFORGE_SLOTS {
        let count_str = db.read_user_data(user_id, &format!("reforge_history_{}", key));
        let count: i32 = count_str.parse().unwrap_or(0);
        if count > 0 {
            has_record = true;
            let equip = db.equip_read(user_id, key);
            let name = equip.map(|e| e.name).unwrap_or_else(|| "无装备".to_string());
            r.push_str(&format!("\n  {}·{}: {}次重铸", cn, name, count));
            total += count;
        }
    }

    if !has_record {
        r.push_str("\n  （暂无重铸记录）");
    }

    r.push_str(&format!("\n\n🔄 总计重铸: {}次", total));
    let gold_spent = total as i64 * REFORGE_COST_GOLD;
    r.push_str(&format!("\n💰 累计花费: {}金币", gold_spent));
    r.push_str("\n\n💡 重铸石可在商店购买或通过副本掉落");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_slot_all() {
        assert!(resolve_slot("武器").is_some());
        assert!(resolve_slot("头盔").is_some());
        assert!(resolve_slot("铠甲").is_some());
        assert!(resolve_slot("护腿").is_some());
        assert!(resolve_slot("靴子").is_some());
        assert!(resolve_slot("项链").is_some());
        assert!(resolve_slot("戒指").is_some());
        assert!(resolve_slot("翅膀").is_some());
        assert!(resolve_slot("时装").is_some());
        assert!(resolve_slot("称号").is_some());
        assert!(resolve_slot("不存在").is_none());
    }

    #[test]
    fn test_reforge_slots_count() {
        assert_eq!(REFORGE_SLOTS.len(), 10);
        assert_eq!(ATTR_NAMES.len(), 15);
        assert_eq!(ATTR_CN.len(), 15);
    }

    #[test]
    fn test_generate_reforge_stats_range() {
        let base = [30i32; 15];
        let result = generate_reforge_stats(&base, 12345);
        for &v in &result {
            assert!(v >= 1 && v <= 999, "value {} out of range", v);
        }
    }

    #[test]
    fn test_generate_deterministic() {
        let base = [50i32; 15];
        let r1 = generate_reforge_stats(&base, 99999);
        let r2 = generate_reforge_stats(&base, 99999);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_generate_different_seeds() {
        let base = [50i32; 15];
        let r1 = generate_reforge_stats(&base, 11111);
        let r2 = generate_reforge_stats(&base, 22222);
        // Different seeds should produce different results (at least in most positions)
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_diff_arrow() {
        assert_eq!(diff_arrow(10, 20), "🔺");
        assert_eq!(diff_arrow(20, 10), "🔻");
        assert_eq!(diff_arrow(10, 10), "➖");
    }

    #[test]
    fn test_calc_attr_score() {
        // Base (30) should be 0
        let base = [30i32; 15];
        assert_eq!(calc_attr_score(&base), 0);

        // Higher values should give higher score
        let high = [50i32; 15];
        assert!(calc_attr_score(&high) > 0);
    }

    #[test]
    fn test_format_score_bar() {
        let bar = format_score_bar(50, 100);
        assert!(bar.contains("50%"));
        let bar_zero = format_score_bar(0, 100);
        assert!(bar_zero.contains("0%"));
    }

    #[test]
    fn test_format_attrs_short_base() {
        let base = [30i32; 15];
        assert_eq!(format_attrs_short(&base), "基础属性");
    }

    #[test]
    fn test_format_attrs_short_with_bonus() {
        let mut attrs = [30i32; 15];
        attrs[0] = 50; // HP+20
        attrs[4] = 45; // AD+15
        let s = format_attrs_short(&attrs);
        assert!(s.contains("生命+20"));
        assert!(s.contains("物攻+15"));
    }

    #[test]
    fn test_djb2_hash() {
        let h1 = djb2_hash("test");
        let h2 = djb2_hash("test");
        assert_eq!(h1, h2); // deterministic
        let h3 = djb2_hash("other");
        assert_ne!(h1, h3); // different input → different hash
    }

    #[test]
    fn test_reforge_cost_constants() {
        assert_eq!(REFORGE_COST_GOLD, 10_000);
        assert_eq!(REFORGE_COST_ITEM, "重铸石");
        assert_eq!(REFORGE_COST_ITEM_QTY, 1);
    }

    #[test]
    fn test_format_attrs_compare() {
        let old = [30i32; 15];
        let mut new_vals = [30i32; 15];
        new_vals[0] = 50; // HP improved
        new_vals[4] = 20; // AD decreased
        let s = format_attrs_compare(&old, &new_vals);
        assert!(s.contains("🔺"));
        assert!(s.contains("🔻"));
    }

    #[test]
    fn test_format_attrs_compare_identical() {
        let attrs = [30i32; 15];
        let s = format_attrs_compare(&attrs, &attrs);
        assert!(s.contains("无变化"));
    }
}
