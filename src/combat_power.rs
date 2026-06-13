/// CakeGame 战力评分详情系统
/// 战力计算公式（来自 MessageTemplate 玩家角色查询）:
///   战力 = (物攻×35 + 法强×217.5 + 生命上限×2.7 + 魔法上限×1.4 + 防御×20 + 魔抗×18
///          + 攻速×2.25 + 移动×12 + 暴击率×60 + 物穿值×20 + 法穿值×44.44) / 10
///
/// 本模块提供详细的战力拆分，显示每个属性来源的贡献值
use crate::core::UserInfo;
use crate::db::Database;
use crate::user;

/// 战力权重
const W_AD: f64 = 35.0;
const W_AP: f64 = 217.5;
const W_HP: f64 = 2.7;
const W_MP: f64 = 1.4;
const W_DEF: f64 = 20.0;
const W_MRES: f64 = 18.0;
const W_HIT: f64 = 2.25;
const W_DODGE: f64 = 12.0;
const W_CRIT: f64 = 60.0;
const W_ADPTV: f64 = 20.0;
const W_APPTV: f64 = 44.44;

/// 计算总战力
pub fn calc_combat_power(info: &UserInfo) -> f64 {
    (info.ad as f64 * W_AD
        + info.ap as f64 * W_AP
        + info.hp_max as f64 * W_HP
        + info.mp_max as f64 * W_MP
        + info.defense as f64 * W_DEF
        + info.magic_res as f64 * W_MRES
        + info.hit as f64 * W_HIT
        + info.dodge as f64 * W_DODGE
        + info.crit as f64 * W_CRIT
        + info.ad_ptv as f64 * W_ADPTV
        + info.ap_ptv as f64 * W_APPTV)
        / 10.0
}

/// 战力来源明细
struct AttrSource {
    name: &'static str,
    value: i32,
    weight: f64,
    contribution: f64,
}

/// 查看战力详情
pub fn cmd_combat_power_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);
    let total_power = calc_combat_power(&info);

    let mut r = format!("{}\n═══ 战力评分详情 ═══\n", prefix);
    r.push_str(&format!("📊 总战力: {:.0}\n", total_power));
    r.push_str(&format!("👤 角色: {} Lv.{}\n\n", info.name, info.level));

    // 收集所有属性贡献
    let sources = [
        AttrSource {
            name: "物攻",
            value: info.ad,
            weight: W_AD,
            contribution: info.ad as f64 * W_AD / 10.0,
        },
        AttrSource {
            name: "法强",
            value: info.ap,
            weight: W_AP,
            contribution: info.ap as f64 * W_AP / 10.0,
        },
        AttrSource {
            name: "生命上限",
            value: info.hp_max,
            weight: W_HP,
            contribution: info.hp_max as f64 * W_HP / 10.0,
        },
        AttrSource {
            name: "魔法上限",
            value: info.mp_max,
            weight: W_MP,
            contribution: info.mp_max as f64 * W_MP / 10.0,
        },
        AttrSource {
            name: "防御",
            value: info.defense,
            weight: W_DEF,
            contribution: info.defense as f64 * W_DEF / 10.0,
        },
        AttrSource {
            name: "魔抗",
            value: info.magic_res,
            weight: W_MRES,
            contribution: info.magic_res as f64 * W_MRES / 10.0,
        },
        AttrSource {
            name: "攻速",
            value: info.hit,
            weight: W_HIT,
            contribution: info.hit as f64 * W_HIT / 10.0,
        },
        AttrSource {
            name: "移动",
            value: info.dodge,
            weight: W_DODGE,
            contribution: info.dodge as f64 * W_DODGE / 10.0,
        },
        AttrSource {
            name: "暴击率",
            value: info.crit,
            weight: W_CRIT,
            contribution: info.crit as f64 * W_CRIT / 10.0,
        },
        AttrSource {
            name: "物穿值",
            value: info.ad_ptv,
            weight: W_ADPTV,
            contribution: info.ad_ptv as f64 * W_ADPTV / 10.0,
        },
        AttrSource {
            name: "法穿值",
            value: info.ap_ptv,
            weight: W_APPTV,
            contribution: info.ap_ptv as f64 * W_APPTV / 10.0,
        },
    ];

    // 按贡献值排序（降序）
    let mut sorted: Vec<&AttrSource> = sources.iter().collect();
    sorted.sort_by(|a, b| b.contribution.partial_cmp(&a.contribution).unwrap());

    r.push_str("━━━ 属性贡献排名 ━━━\n");
    for (i, src) in sorted.iter().enumerate() {
        let pct = if total_power > 0.0 {
            src.contribution / total_power * 100.0
        } else {
            0.0
        };
        let bar_len = (pct / 3.0).round() as usize;
        let bar: String = "█".repeat(bar_len.min(20));
        r.push_str(&format!(
            "\n{}. {} +{} (×{:.1})\n   战力贡献: {:.0} ({:.1}%)\n   {}",
            i + 1,
            src.name,
            src.value,
            src.weight,
            src.contribution,
            pct,
            bar
        ));
    }

    // 额外属性（不计入战力但有用）
    r.push_str("\n\n━━━ 其他属性 ━━━");
    r.push_str(&format!("\n吸血比: {}%", info.absorb_hp));
    r.push_str(&format!("\n伤害免疫: {}%", info.immune));
    r.push_str(&format!("\n护盾值: {}", info.shield));

    // 灵兽加成
    if let Some((beast_name, b_hp, b_ad, b_def, b_mdf, skill)) = crate::beast::get_active_beast_bonus(db, user_id) {
        r.push_str(&format!("\n\n━━━ 出战灵兽: {} ━━━", beast_name));
        r.push_str(&format!("\n  HP+{} AD+{} DEF+{} MDF+{}", b_hp, b_ad, b_def, b_mdf));
        r.push_str(&format!("\n  技能: {}", skill));
    }

    // 竞技积分
    let pvp_score: i32 = db.read_user_data(user_id, "match_score").parse().unwrap_or(1000);
    r.push_str(&format!("\n\n🏆 竞技积分: {}", pvp_score));

    format!("{}\n", r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_power_formula() {
        let info = UserInfo {
            ad: 100,
            ap: 50,
            hp_max: 1000,
            mp_max: 200,
            defense: 30,
            magic_res: 20,
            hit: 10,
            dodge: 5,
            crit: 3,
            ad_ptv: 0,
            ap_ptv: 0,
            ..Default::default()
        };
        let power = calc_combat_power(&info);
        // Expected: (100*35 + 50*217.5 + 1000*2.7 + 200*1.4 + 30*20 + 20*18
        //           + 10*2.25 + 5*12 + 3*60 + 0 + 0) / 10
        // = (3500 + 10875 + 2700 + 280 + 600 + 360 + 22.5 + 60 + 180) / 10
        // = 18577.5 / 10 = 1857.75
        assert!((power - 1857.75).abs() < 0.1, "Expected ~1857.75, got {}", power);
    }

    #[test]
    fn test_combat_power_zero() {
        let info = UserInfo::default();
        let power = calc_combat_power(&info);
        assert_eq!(power, 0.0);
    }
}
