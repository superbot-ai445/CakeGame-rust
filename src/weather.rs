/// CakeGame 天气/环境系统
///
/// 动态天气影响战斗和游戏体验:
/// - 每张地图有独立天气，基于日期+地图名哈希确定性生成
/// - 天气影响战斗属性加成/减成
/// - 天气预报查看未来天气变化
/// - 7种天气类型: 晴天☀️/雨天🌧️/雷暴⛈️/大风💨/雾天🌫️/雪天❄️/沙暴🏜️
///
/// 指令: 查看天气, 天气预报, 天气效果
use crate::db::Database;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 天气类型定义
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeatherType {
    Sunny,     // 晴天
    Rainy,     // 雨天
    Storm,     // 雷暴
    Windy,     // 大风
    Foggy,     // 雾天
    Snowy,     // 雪天
    Sandstorm, // 沙暴
}

impl WeatherType {
    pub fn from_index(idx: usize) -> Self {
        match idx % 7 {
            0 => Self::Sunny,
            1 => Self::Rainy,
            2 => Self::Storm,
            3 => Self::Windy,
            4 => Self::Foggy,
            5 => Self::Snowy,
            6 => Self::Sandstorm,
            _ => Self::Sunny,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Sunny => "晴天",
            Self::Rainy => "雨天",
            Self::Storm => "雷暴",
            Self::Windy => "大风",
            Self::Foggy => "雾天",
            Self::Snowy => "雪天",
            Self::Sandstorm => "沙暴",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Sunny => "☀️",
            Self::Rainy => "🌧️",
            Self::Storm => "⛈️",
            Self::Windy => "💨",
            Self::Foggy => "🌫️",
            Self::Snowy => "❄️",
            Self::Sandstorm => "🏜️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Sunny => "万里无云，阳光普照",
            Self::Rainy => "细雨绵绵，道路湿滑",
            Self::Storm => "电闪雷鸣，风雨交加",
            Self::Windy => "狂风大作，飞沙走石",
            Self::Foggy => "浓雾弥漫，视野受限",
            Self::Snowy => "大雪纷飞，银装素裹",
            Self::Sandstorm => "黄沙漫天，遮天蔽日",
        }
    }

    /// 天气对战斗属性的影响 (属性名, 加成百分比)
    pub fn combat_effects(&self) -> Vec<(&'static str, i32)> {
        match self {
            Self::Sunny => vec![("AD", 5), ("AP", 5)],        // 晴天全攻击小幅加成
            Self::Rainy => vec![("AP", 10), ("Dodge", -5)],   // 雨天魔攻加成，闪避降低
            Self::Storm => vec![("Crit", 15), ("Hit", -8)],   // 雷暴暴击大增，命中降低
            Self::Windy => vec![("Dodge", 12), ("Hit", -5)],  // 大风闪避增强，命中降低
            Self::Foggy => vec![("Hit", -15), ("Dodge", 10)], // 雾天命中大降，闪避增强
            Self::Snowy => vec![("Defense", 10), ("AD", -5)], // 雪天防御增强，物攻降低
            Self::Sandstorm => vec![("MagicResistance", 10), ("AP", -8)], // 沙暴魔抗增强，魔攻降低
        }
    }
}

/// 根据日期和地图名确定性计算天气
pub fn get_weather_for_map(date: &str, map_name: &str) -> WeatherType {
    let mut hasher = DefaultHasher::new();
    date.hash(&mut hasher);
    map_name.hash(&mut hasher);
    let hash = hasher.finish();
    WeatherType::from_index((hash % 7) as usize)
}

/// 获取今天的日期字符串
fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 获取N天后的日期字符串
fn days_later_str(n: i64) -> String {
    (chrono::Local::now() + chrono::Duration::days(n))
        .format("%Y-%m-%d")
        .to_string()
}

/// 查看天气指令
pub fn cmd_view_weather(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_name = if args.is_empty() {
        db.read_basic(user_id, "Map")
    } else {
        args.trim().to_string()
    };

    if map_name.is_empty() {
        return format!("ID：{}\n📍 您尚未进入任何地图！请先进入地图后查看天气。", user_id);
    }

    let today = today_str();
    let weather = get_weather_for_map(&today, &map_name);
    let effects = weather.combat_effects();

    let mut result = format!(
        "ID：{}\n═══ {} 天气 ═══\n{} {} {}\n📝 {}\n\n⚔️ 战斗效果:\n",
        user_id,
        map_name,
        weather.emoji(),
        weather.name(),
        weather.emoji(),
        weather.description()
    );

    if effects.is_empty() {
        result.push_str("  无特殊效果\n");
    } else {
        for (attr, pct) in &effects {
            let sign = if *pct > 0 { "+" } else { "" };
            let icon = if *pct > 0 { "📈" } else { "📉" };
            let attr_name = match *attr {
                "AD" => "物攻",
                "AP" => "魔攻",
                "Defense" => "防御",
                "MagicResistance" => "魔抗",
                "Hit" => "命中",
                "Dodge" => "闪避",
                "Crit" => "暴击",
                _ => attr,
            };
            result.push_str(&format!("  {} {} {}{}%\n", icon, attr_name, sign, pct));
        }
    }

    // 检查天气对玩家的属性影响
    let user_level = db.read_basic(user_id, "Level").parse::<i32>().unwrap_or(1);
    let occupation = db.read_basic(user_id, "Occupation");
    result.push_str(&format!(
        "\n📊 当前加成基于: Lv.{} {}\n💡 提示: 输入 天气预报 查看未来3天天气",
        user_level, occupation
    ));

    result
}

/// 天气预报指令 - 查看当前地图未来3天天气
pub fn cmd_weather_forecast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_name = if args.is_empty() {
        db.read_basic(user_id, "Map")
    } else {
        args.trim().to_string()
    };

    if map_name.is_empty() {
        return format!("ID：{}\n📍 您尚未进入任何地图！请先进入地图后查看天气预报。", user_id);
    }

    let today = today_str();
    let mut result = format!("ID：{}\n═══ {} 天气预报 ═══\n", user_id, map_name);

    // 今天的天气
    let today_weather = get_weather_for_map(&today, &map_name);
    result.push_str(&format!(
        "📅 今天 ({}): {} {} {}\n",
        today,
        today_weather.emoji(),
        today_weather.name(),
        today_weather.description()
    ));

    // 未来3天预报
    for i in 1..=3 {
        let date = days_later_str(i);
        let weather = get_weather_for_map(&date, &map_name);
        result.push_str(&format!(
            "📅 {} ({}+{}天): {} {} {}\n",
            if i == 1 {
                "明天"
            } else if i == 2 {
                "后天"
            } else {
                "大后天"
            },
            date,
            i,
            weather.emoji(),
            weather.name(),
            weather.description()
        ));
    }

    result.push_str(&format!(
        "\n🗺️ 不同地图天气不同！输入 天气预报+地图名 查看其他地图天气\n\
         💡 当前地图: {}",
        map_name
    ));

    result
}

/// 天气效果指令 - 查看天气对所有属性的影响详情
pub fn cmd_weather_effects(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let today = today_str();
    let map_name = db.read_basic(user_id, "Map");
    let occupation = db.read_basic(user_id, "Occupation");

    let mut result = format!("ID：{}\n═══ 天气效果总览 ═══\n\n", user_id);

    // 列出所有天气类型及效果
    let all_weathers = [
        WeatherType::Sunny,
        WeatherType::Rainy,
        WeatherType::Storm,
        WeatherType::Windy,
        WeatherType::Foggy,
        WeatherType::Snowy,
        WeatherType::Sandstorm,
    ];

    for w in &all_weathers {
        let effects = w.combat_effects();
        result.push_str(&format!("{} {} — {}\n", w.emoji(), w.name(), w.description()));
        if effects.is_empty() {
            result.push_str("  无特殊效果\n");
        } else {
            let effect_strs: Vec<String> = effects
                .iter()
                .map(|(attr, pct)| {
                    let sign = if *pct > 0 { "+" } else { "" };
                    let attr_name = match *attr {
                        "AD" => "物攻",
                        "AP" => "魔攻",
                        "Defense" => "防御",
                        "MagicResistance" => "魔抗",
                        "Hit" => "命中",
                        "Dodge" => "闪避",
                        "Crit" => "暴击",
                        _ => attr,
                    };
                    format!("{}{}{}%", attr_name, sign, pct)
                })
                .collect();
            result.push_str(&format!("  {}\n", effect_strs.join(" | ")));
        }
        result.push('\n');
    }

    // 当前地图天气高亮
    if !map_name.is_empty() {
        let current = get_weather_for_map(&today, &map_name);
        let effects = current.combat_effects();
        result.push_str(&format!(
            "📍 当前地图 [{}] 天气: {} {}\n",
            map_name,
            current.emoji(),
            current.name()
        ));
        if !effects.is_empty() {
            result.push_str("⚔️ 今日战斗加成:\n");
            for (attr, pct) in &effects {
                let sign = if *pct > 0 { "+" } else { "" };
                let attr_name = match *attr {
                    "AD" => "物攻",
                    "AP" => "魔攻",
                    "Defense" => "防御",
                    "MagicResistance" => "魔抗",
                    "Hit" => "命中",
                    "Dodge" => "闪避",
                    "Crit" => "暴击",
                    _ => attr,
                };
                result.push_str(&format!("  • {} {}{}%\n", attr_name, sign, pct));
            }
        }
    }

    result.push_str(&format!("\n📊 职业: {} | 💡 不同天气适合不同职业作战策略", occupation));

    result
}

/// 获取天气战斗加成百分比（供 combat 系统调用）
/// 返回 (AD%, AP%, Defense%, MR%, Hit%, Dodge%, Crit%) 加成
#[allow(dead_code)]
pub fn get_weather_combat_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32, i32, i32) {
    let map_name = db.read_basic(user_id, "Map");
    if map_name.is_empty() {
        return (0, 0, 0, 0, 0, 0, 0);
    }
    let today = today_str();
    let weather = get_weather_for_map(&today, &map_name);
    let effects = weather.combat_effects();

    let mut ad_bonus = 0;
    let mut ap_bonus = 0;
    let mut def_bonus = 0;
    let mut mr_bonus = 0;
    let mut hit_bonus = 0;
    let mut dodge_bonus = 0;
    let mut crit_bonus = 0;

    for (attr, pct) in &effects {
        match *attr {
            "AD" => ad_bonus += pct,
            "AP" => ap_bonus += pct,
            "Defense" => def_bonus += pct,
            "MagicResistance" => mr_bonus += pct,
            "Hit" => hit_bonus += pct,
            "Dodge" => dodge_bonus += pct,
            "Crit" => crit_bonus += pct,
            _ => {}
        }
    }

    (
        ad_bonus,
        ap_bonus,
        def_bonus,
        mr_bonus,
        hit_bonus,
        dodge_bonus,
        crit_bonus,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_types_count() {
        // 7种天气类型
        let all = [
            WeatherType::Sunny,
            WeatherType::Rainy,
            WeatherType::Storm,
            WeatherType::Windy,
            WeatherType::Foggy,
            WeatherType::Snowy,
            WeatherType::Sandstorm,
        ];
        assert_eq!(all.len(), 7);
        for (i, w) in all.iter().enumerate() {
            assert_eq!(*w, WeatherType::from_index(i));
        }
    }

    #[test]
    fn test_weather_from_index_wraps() {
        // 索引应该 mod 7 循环
        assert_eq!(WeatherType::from_index(0), WeatherType::Sunny);
        assert_eq!(WeatherType::from_index(7), WeatherType::Sunny);
        assert_eq!(WeatherType::from_index(14), WeatherType::Sunny);
        assert_eq!(WeatherType::from_index(8), WeatherType::Rainy);
    }

    #[test]
    fn test_weather_names_not_empty() {
        let all = [
            WeatherType::Sunny,
            WeatherType::Rainy,
            WeatherType::Storm,
            WeatherType::Windy,
            WeatherType::Foggy,
            WeatherType::Snowy,
            WeatherType::Sandstorm,
        ];
        for w in &all {
            assert!(!w.name().is_empty());
            assert!(!w.emoji().is_empty());
            assert!(!w.description().is_empty());
        }
    }

    #[test]
    fn test_weather_effects_non_empty() {
        // 每种天气都应有战斗效果
        let all = [
            WeatherType::Sunny,
            WeatherType::Rainy,
            WeatherType::Storm,
            WeatherType::Windy,
            WeatherType::Foggy,
            WeatherType::Snowy,
            WeatherType::Sandstorm,
        ];
        for w in &all {
            assert!(!w.combat_effects().is_empty(), "Weather {:?} should have effects", w);
        }
    }

    #[test]
    fn test_weather_deterministic() {
        // 同一日期+地图 = 同一天气
        let w1 = get_weather_for_map("2026-01-01", "主城");
        let w2 = get_weather_for_map("2026-01-01", "主城");
        assert_eq!(w1, w2);
    }

    #[test]
    fn test_weather_different_dates() {
        // 不同日期天气回合循环但可能不同
        let mut seen = std::collections::HashSet::new();
        for day in 1..=30 {
            let date = format!("2026-01-{:02}", day);
            let w = get_weather_for_map(&date, "主城");
            seen.insert(w as u8);
        }
        // 30天内应该至少出现3种不同天气
        assert!(
            seen.len() >= 3,
            "Expected at least 3 weather types in 30 days, got {}",
            seen.len()
        );
    }

    #[test]
    fn test_weather_different_maps() {
        // 同一天不同地图天气可能不同
        let maps = ["主城", "森林", "沙漠", "雪山", "海底"];
        let mut seen = std::collections::HashSet::new();
        for map in &maps {
            let w = get_weather_for_map("2026-06-10", map);
            seen.insert(w as u8);
        }
        // 5张地图应至少出现2种不同天气
        assert!(
            seen.len() >= 2,
            "Expected at least 2 weather types across 5 maps, got {}",
            seen.len()
        );
    }

    #[test]
    fn test_combat_effects_values_reasonable() {
        // 所有效果值在 -20% ~ +20% 范围内
        let all = [
            WeatherType::Sunny,
            WeatherType::Rainy,
            WeatherType::Storm,
            WeatherType::Windy,
            WeatherType::Foggy,
            WeatherType::Snowy,
            WeatherType::Sandstorm,
        ];
        for w in &all {
            for (attr, pct) in w.combat_effects() {
                assert!(!attr.is_empty());
                assert!(
                    pct >= -20 && pct <= 20,
                    "Weather {:?} effect {} = {}% out of range",
                    w,
                    attr,
                    pct
                );
            }
        }
    }
}
