/// CakeGame 公会红包系统
/// 公会成员可以发放金币/钻石红包，其他成员可抢红包
/// 红包金额随机分配，增加公会互动乐趣
/// 数据存储: Global 表 SECTION='guild_red_packet' / 'red_packet'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD, OP_SUB};
use crate::db::Database;
use crate::user;

/// 红包类型
#[derive(Debug, Clone, PartialEq)]
enum PacketType {
    Gold,
    Diamond,
}

impl PacketType {
    fn as_str(&self) -> &'static str {
        match self {
            PacketType::Gold => "gold",
            PacketType::Diamond => "diamond",
        }
    }
    fn currency_key(&self) -> &'static str {
        match self {
            PacketType::Gold => CURRENCY_GOLD,
            PacketType::Diamond => CURRENCY_DIAMOND,
        }
    }
    fn currency_name(&self) -> &'static str {
        match self {
            PacketType::Gold => "金币",
            PacketType::Diamond => "钻石",
        }
    }
    fn emoji(&self) -> &'static str {
        match self {
            PacketType::Gold => "💰",
            PacketType::Diamond => "💎",
        }
    }
}

/// 红包记录
#[derive(Debug, Clone)]
struct RedPacket {
    id: String,
    sender: String,
    sender_name: String,
    packet_type: PacketType,
    total_amount: i64,
    remaining_amount: i64,
    max_grabbers: i32,
    grabbed_count: i32,
    message: String,
    create_ts: u64,
    expire_ts: u64,
    /// 已领取记录: [(user_id, amount)]
    grabs: Vec<(String, i64)>,
}

/// 红包有效期（秒）
const PACKET_EXPIRE_SECS: u64 = 3600;

/// 最小红包金额
const MIN_GOLD_AMOUNT: i64 = 100;
const MIN_DIAMOND_AMOUNT: i64 = 10;

/// 最大红包份数
const MAX_GRABBERS: i32 = 20;

/// 每日发红包上限
const DAILY_SEND_LIMIT: i32 = 10;

/// 每日抢红包上限
const DAILY_GRAB_LIMIT: i32 = 30;

/// 抢红包冷却（秒）
const GRAB_COOLDOWN_SECS: u64 = 5;

/// djb2 哈希
fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 生成红包ID
fn gen_packet_id(user_id: &str) -> String {
    let ts = now_ts();
    let hash = djb2_hash(&format!("{}{}", user_id, ts));
    format!("RP-{}-{:08x}", ts % 100000, hash % 0xFFFFFFFF)
}

/// 获取当前时间戳
fn now_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 解析红包数据
fn parse_packet_data(data: &str) -> Option<RedPacket> {
    let lines: Vec<&str> = data.split('\n').collect();
    if lines.len() < 11 {
        return None;
    }
    let id = lines[0].to_string();
    let sender = lines[1].to_string();
    let sender_name = lines[2].to_string();
    let packet_type = match lines[3] {
        "gold" => PacketType::Gold,
        "diamond" => PacketType::Diamond,
        _ => return None,
    };
    let total_amount: i64 = lines[4].parse().ok()?;
    let remaining_amount: i64 = lines[5].parse().ok()?;
    let max_grabbers: i32 = lines[6].parse().ok()?;
    let grabbed_count: i32 = lines[7].parse().ok()?;
    let message = lines[8].to_string();
    let create_ts: u64 = lines[9].parse().ok()?;
    let expire_ts: u64 = lines[10].parse().ok()?;

    let mut grabs = Vec::new();
    for line in lines.iter().skip(11) {
        if let Some((uid, amt)) = line.split_once(':') {
            if let Ok(amt) = amt.parse::<i64>() {
                grabs.push((uid.to_string(), amt));
            }
        }
    }

    Some(RedPacket {
        id,
        sender,
        sender_name,
        packet_type,
        total_amount,
        remaining_amount,
        max_grabbers,
        grabbed_count,
        message,
        create_ts,
        expire_ts,
        grabs,
    })
}

/// 序列化红包数据
fn serialize_packet(p: &RedPacket) -> String {
    let mut out = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        p.id,
        p.sender,
        p.sender_name,
        p.packet_type.as_str(),
        p.total_amount,
        p.remaining_amount,
        p.max_grabbers,
        p.grabbed_count,
        p.message,
        p.create_ts,
        p.expire_ts,
    );
    for (uid, amt) in &p.grabs {
        out.push_str(&format!("\n{}:{}", uid, amt));
    }
    out
}

/// 检查用户是否有公会
fn get_user_guild(db: &Database, user_id: &str) -> Option<String> {
    let guild = db.read_basic(user_id, "公会");
    if guild.is_empty() || guild == "无" || guild == "[NULL]" {
        None
    } else {
        Some(guild)
    }
}

/// 随机分配红包金额 (手气红包)
/// 使用二倍均值法确保公平且每人至少1
#[cfg(test)]
fn split_amount(total: i64, count: i32) -> Vec<i64> {
    if count <= 0 || total <= 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![total];
    }

    let mut rng_seed = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    };

    let mut remaining = total;
    let mut result = Vec::new();
    let cnt = count as i64;

    for i in 0..count {
        if i == count - 1 {
            result.push(remaining.max(1));
            break;
        }
        let people_left = cnt - i as i64;
        let max_grab = (remaining - people_left + 1).max(2);
        if max_grab <= 1 {
            result.push(1);
            remaining -= 1;
            continue;
        }
        rng_seed = rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let rand_val = (rng_seed >> 33) as i64;
        let grab = (rand_val % max_grab).max(1).min(max_grab - 1);
        result.push(grab);
        remaining -= grab;
    }

    // 确保总数正确
    let sum: i64 = result.iter().sum();
    if sum != total && !result.is_empty() {
        let diff = total - sum;
        let last = result.last_mut().unwrap();
        *last = (*last + diff).max(1);
    }

    result
}

/// 发放公会红包
/// 用法: 发红包+金币/钻石+金额+份数+祝福语
pub fn cmd_send_red_packet(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您未加入任何公会，请先加入公会再发红包！", prefix),
    };

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!(
            "{}\n🧧 发红包用法：发红包+类型+金额+份数+祝福语\n💡 示例：发红包+金币+5000+5+新年快乐！\n📌 类型：金币 或 钻石\n📌 金额：最低 {}金币 / {}钻石\n📌 份数：1~{}人\n📌 祝福语可省略",
            prefix, MIN_GOLD_AMOUNT, MIN_DIAMOND_AMOUNT, MAX_GRABBERS
        );
    }

    // 解析类型
    let packet_type = match parts[0] {
        "金币" | "gold" | "金" => PacketType::Gold,
        "钻石" | "diamond" | "钻" => PacketType::Diamond,
        _ => return format!("{}\n❌ 红包类型只能是「金币」或「钻石」", prefix),
    };

    // 解析金额
    let amount: i64 = match parts[1].parse() {
        Ok(a) if a > 0 => a,
        _ => return format!("{}\n❌ 请输入有效的红包金额", prefix),
    };

    // 检查最小金额
    let min_amount = match packet_type {
        PacketType::Gold => MIN_GOLD_AMOUNT,
        PacketType::Diamond => MIN_DIAMOND_AMOUNT,
    };
    if amount < min_amount {
        return format!(
            "{}\n❌ {}红包最低金额为 {}{}",
            prefix,
            packet_type.emoji(),
            min_amount,
            packet_type.currency_name()
        );
    }

    // 解析份数
    let grabbers: i32 = parts
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .clamp(1, MAX_GRABBERS);

    // 检查份数不超过金额
    if grabbers as i64 > amount {
        return format!(
            "{}\n❌ 红包份数不能超过金额（每份至少1{}）",
            prefix,
            packet_type.currency_name()
        );
    }

    // 解析祝福语
    let message = if parts.len() > 3 {
        let raw = parts[3..].join("+");
        if raw.len() > 50 {
            raw[..50].to_string()
        } else {
            raw
        }
    } else {
        format!("恭喜发财，{}拿来！", packet_type.currency_name())
    };

    // 检查余额
    let current_balance = db.read_currency(user_id, packet_type.currency_key());
    if current_balance < amount {
        return format!(
            "{}\n{} {}不足！需要 {}{}，当前 {}{}",
            prefix,
            packet_type.emoji(),
            packet_type.currency_name(),
            amount,
            packet_type.currency_name(),
            current_balance,
            packet_type.currency_name()
        );
    }

    // 检查每日发送限制
    let today = today_str();
    let send_count_key = format!("send_{}_{}", user_id, today);
    let send_count: i32 = db.global_get("red_packet", &send_count_key).parse().unwrap_or(0);
    if send_count >= DAILY_SEND_LIMIT {
        return format!(
            "{}\n❌ 今日发红包已达上限（{}/{}）",
            prefix, DAILY_SEND_LIMIT, DAILY_SEND_LIMIT
        );
    }

    // 扣除余额
    db.modify_currency(user_id, packet_type.currency_key(), OP_SUB, amount);
    db.global_set("red_packet", &send_count_key, &(send_count + 1).to_string());

    // 获取昵称
    let nickname = db.read_basic(user_id, "昵称");
    let nickname = if nickname.is_empty() || nickname == "[NULL]" {
        user_id.to_string()
    } else {
        nickname
    };

    // 创建红包
    let ts = now_ts();
    let packet = RedPacket {
        id: gen_packet_id(user_id),
        sender: user_id.to_string(),
        sender_name: nickname,
        packet_type: packet_type.clone(),
        total_amount: amount,
        remaining_amount: amount,
        max_grabbers: grabbers,
        grabbed_count: 0,
        message: message.clone(),
        create_ts: ts,
        expire_ts: ts + PACKET_EXPIRE_SECS,
        grabs: Vec::new(),
    };

    // 保存红包
    let packet_data = serialize_packet(&packet);
    db.global_set("guild_red_packet", &packet.id, &packet_data);

    // 记录到公会红包列表
    let list_key = format!("guild_packets_{}", guild);
    let existing_list = db.global_get("red_packet", &list_key);
    let new_list = if existing_list.is_empty() {
        packet.id.clone()
    } else {
        format!("{}|{}", existing_list, packet.id)
    };
    db.global_set("red_packet", &list_key, &new_list);

    format!(
        "{}\n🧧 ═══ 红包发放成功！═══\n🎉 {} 发了一个 {}红包\n{} {}{} ({}份)\n💬 {}\n📢 公会「{}」成员发送「抢红包」即可领取！\n⏰ 有效期1小时",
        prefix, packet.sender_name, packet_type.currency_name(),
        packet_type.emoji(), amount, packet_type.currency_name(), grabbers,
        message, guild,
    )
}

/// 抢公会红包
pub fn cmd_grab_red_packet(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您未加入任何公会，无法抢红包！", prefix),
    };

    // 检查每日抢红包限制
    let today = today_str();
    let grab_count_key = format!("grab_{}_{}", user_id, today);
    let grab_count: i32 = db.global_get("red_packet", &grab_count_key).parse().unwrap_or(0);
    if grab_count >= DAILY_GRAB_LIMIT {
        return format!(
            "{}\n❌ 今日抢红包已达上限（{}/{}）",
            prefix, DAILY_GRAB_LIMIT, DAILY_GRAB_LIMIT
        );
    }

    // 检查抢红包冷却
    let cooldown_key = format!("cooldown_{}", user_id);
    let last_grab_ts: u64 = db.global_get("red_packet", &cooldown_key).parse().unwrap_or(0);
    let now = now_ts();
    if now < last_grab_ts + GRAB_COOLDOWN_SECS {
        let remaining = (last_grab_ts + GRAB_COOLDOWN_SECS).saturating_sub(now);
        return format!("{}\n⏰ 抢红包冷却中，请 {}秒后再试", prefix, remaining);
    }

    // 获取公会红包列表
    let list_key = format!("guild_packets_{}", guild);
    let packet_list = db.global_get("red_packet", &list_key);
    if packet_list.is_empty() {
        return format!("{}\n📭 公会暂无可抢的红包，快去发一个吧！", prefix);
    }

    let packet_ids: Vec<&str> = packet_list.split('|').collect();

    // 查找可抢的红包（排除自己发的和已抢过的）
    let mut available: Vec<RedPacket> = Vec::new();
    for pid in &packet_ids {
        let data = db.global_get("guild_red_packet", pid);
        if data.is_empty() {
            continue;
        }
        if let Some(packet) = parse_packet_data(&data) {
            if now > packet.expire_ts {
                continue;
            }
            if packet.grabbed_count >= packet.max_grabbers {
                continue;
            }
            if packet.sender == user_id {
                continue;
            }
            if packet.grabs.iter().any(|(uid, _)| uid == user_id) {
                continue;
            }
            available.push(packet);
        }
    }

    if available.is_empty() {
        return format!(
            "{}\n📭 暂无可抢的红包\n💡 可能是：自己发的/已抢过/已抢完/已过期",
            prefix
        );
    }

    // 优先抢最新的
    let packet = available.last_mut().unwrap();

    // 计算可抢金额（二倍均值法）
    let people_left = packet.max_grabbers - packet.grabbed_count;
    let grab_amount = if people_left <= 1 {
        packet.remaining_amount
    } else {
        let avg = packet.remaining_amount / people_left as i64;
        let max_grab = (avg * 2).min(packet.remaining_amount - (people_left as i64 - 1)).max(2);
        let rng_val = {
            let seed = (now as u128)
                .wrapping_mul((user_id.len() as u128).wrapping_add(1))
                .wrapping_add(packet.remaining_amount as u128);
            ((seed >> 33) as i64 % max_grab + 1).max(1).min(max_grab - 1)
        };
        rng_val.max(1)
    };

    // 更新红包
    packet.remaining_amount -= grab_amount;
    packet.grabbed_count += 1;
    packet.grabs.push((user_id.to_string(), grab_amount));
    let updated_data = serialize_packet(packet);
    db.global_set("guild_red_packet", &packet.id, &updated_data);

    // 更新抢红包次数
    db.global_set("red_packet", &grab_count_key, &(grab_count + 1).to_string());
    db.global_set("red_packet", &cooldown_key, &now.to_string());

    // 发放金币/钻石
    db.modify_currency(user_id, packet.packet_type.currency_key(), OP_ADD, grab_amount);

    // 获取昵称
    let nickname = db.read_basic(user_id, "昵称");
    let nickname = if nickname.is_empty() || nickname == "[NULL]" {
        user_id.to_string()
    } else {
        nickname
    };

    // 判断是否手气最佳
    let is_best = packet.grabbed_count > 1 && packet.grabs.iter().all(|(_, amt)| *amt <= grab_amount);

    let status = if packet.grabbed_count >= packet.max_grabbers {
        "🎊 红包已抢完！"
    } else {
        "📦 还有可抢"
    };

    let mut out = format!(
        "{}\n🧧 ═══ 抢到红包！═══\n{} 抢到 {} {}{}\n💬 来自 {} 的祝福：{}\n{}",
        prefix,
        nickname,
        grab_amount,
        packet.packet_type.currency_name(),
        if is_best { " 🏆 手气最佳！" } else { "" },
        packet.sender_name,
        packet.message,
        status,
    );

    if is_best {
        out.push_str("\n🍀 手气王就是你！");
    }
    if packet.grabbed_count >= packet.max_grabbers {
        out.push_str(&format!("\n🎊 {}份红包已被抢完！", packet.max_grabbers));
    } else {
        out.push_str(&format!(
            "\n📦 已抢 {}/{} 份",
            packet.grabbed_count, packet.max_grabbers
        ));
    }

    out
}

/// 查看公会红包列表
pub fn cmd_view_red_packets(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您未加入任何公会！", prefix),
    };

    let now = now_ts();
    let list_key = format!("guild_packets_{}", guild);
    let packet_list = db.global_get("red_packet", &list_key);

    if packet_list.is_empty() {
        return format!(
            "{}\n📭 公会「{}」暂无红包记录\n💡 发送「发红包」来发放一个！",
            prefix, guild
        );
    }

    let packet_ids: Vec<&str> = packet_list.split('|').collect();
    let mut active_count = 0;
    let mut finished_count = 0;

    let mut out = format!("{}\n═══ 🧧 公会红包 ═══\n📢 公会: {}\n", prefix, guild);

    // 显示最近的10个红包
    let start = if packet_ids.len() > 10 {
        packet_ids.len() - 10
    } else {
        0
    };
    for pid in &packet_ids[start..] {
        let data = db.global_get("guild_red_packet", pid);
        if data.is_empty() {
            continue;
        }
        if let Some(packet) = parse_packet_data(&data) {
            let expired = now > packet.expire_ts;
            let finished = packet.grabbed_count >= packet.max_grabbers;
            let status_icon = if expired {
                "⏰"
            } else if finished {
                "✅"
            } else {
                "🧧"
            };

            if expired || finished {
                finished_count += 1;
            } else {
                active_count += 1;
            }

            let self_grabbed = packet.grabs.iter().any(|(uid, _)| uid == user_id);
            let self_tag = if self_grabbed {
                " ✅已抢"
            } else if packet.sender == user_id {
                " 📤我发的"
            } else if !expired && !finished {
                " 🔓可抢"
            } else {
                ""
            };

            let my_amount: i64 = packet
                .grabs
                .iter()
                .find(|(uid, _)| uid == user_id)
                .map(|(_, amt)| *amt)
                .unwrap_or(0);

            let status_text = if finished {
                "已抢完".to_string()
            } else {
                format!("{}/{}", packet.grabbed_count, packet.max_grabbers)
            };

            out.push_str(&format!(
                "\n{} {}的{}红包 | {}{} {}/{}份{}\n💬 {}",
                status_icon,
                packet.sender_name,
                packet.packet_type.currency_name(),
                status_text,
                packet.packet_type.emoji(),
                packet.total_amount,
                packet.packet_type.currency_name(),
                self_tag,
                packet.message,
            ));

            if my_amount > 0 {
                out.push_str(&format!(
                    "\n   → 你抢到 {}{}",
                    my_amount,
                    packet.packet_type.currency_name()
                ));
            }
            out.push('\n');
        }
    }

    out.push_str(&format!(
        "\n📊 可抢: {}个 | 已抢完/过期: {}个",
        active_count, finished_count
    ));
    out.push_str("\n💡 发送「抢红包」领取红包");
    out.push_str("\n💡 发送「发红包」发放红包");

    out
}

/// 红包统计
pub fn cmd_red_packet_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您未加入任何公会！", prefix),
    };

    let now = now_ts();
    let list_key = format!("guild_packets_{}", guild);
    let packet_list = db.global_get("red_packet", &list_key);

    let mut total_sent_gold: i64 = 0;
    let mut total_sent_diamond: i64 = 0;
    let mut total_sent_count: i32 = 0;
    let mut total_grabbed_gold: i64 = 0;
    let mut total_grabbed_diamond: i64 = 0;
    let mut total_grabbed_count: i32 = 0;
    let mut best_grab: i64 = 0;
    let mut active_count = 0;

    if !packet_list.is_empty() {
        for pid in packet_list.split('|') {
            let data = db.global_get("guild_red_packet", pid);
            if data.is_empty() {
                continue;
            }
            if let Some(packet) = parse_packet_data(&data) {
                if now <= packet.expire_ts && packet.grabbed_count < packet.max_grabbers {
                    active_count += 1;
                }
                if packet.sender == user_id {
                    total_sent_count += 1;
                    match packet.packet_type {
                        PacketType::Gold => total_sent_gold += packet.total_amount,
                        PacketType::Diamond => total_sent_diamond += packet.total_amount,
                    }
                }
                if let Some((_, amt)) = packet.grabs.iter().find(|(uid, _)| uid == user_id) {
                    total_grabbed_count += 1;
                    match packet.packet_type {
                        PacketType::Gold => total_grabbed_gold += *amt,
                        PacketType::Diamond => total_grabbed_diamond += *amt,
                    }
                    if *amt > best_grab {
                        best_grab = *amt;
                    }
                }
            }
        }
    }

    let today = today_str();
    let send_count_key = format!("send_{}_{}", user_id, today);
    let send_count: i32 = db.global_get("red_packet", &send_count_key).parse().unwrap_or(0);
    let grab_count_key = format!("grab_{}_{}", user_id, today);
    let grab_count: i32 = db.global_get("red_packet", &grab_count_key).parse().unwrap_or(0);

    format!(
        "{}\n═══ 🧧 红包统计 ═══\n📤 我发的红包：{}个\n├ 💰 金币红包：{}\n└ 💎 钻石红包：{}\n\n📥 我抢的红包：{}个\n├ 💰 抢到金币：{}\n├ 💎 抢到钻石：{}\n└ 🏆 最佳手气：{}\n\n📅 今日统计\n├ 📤 已发：{}/{}\n└ 📥 已抢：{}/{}\n\n🧧 当前可抢：{}个\n📢 公会「{}」",
        prefix,
        total_sent_count,
        format_num(total_sent_gold),
        format_num(total_sent_diamond),
        total_grabbed_count,
        format_num(total_grabbed_gold),
        format_num(total_grabbed_diamond),
        if best_grab > 0 {
            format_num(best_grab)
        } else {
            "无".to_string()
        },
        send_count,
        DAILY_SEND_LIMIT,
        grab_count,
        DAILY_GRAB_LIMIT,
        active_count,
        guild,
    )
}

/// 格式化数字（千分位）
fn format_num(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 获取今天日期字符串 (YYYY-MM-DD)
fn today_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = ts / 86400;
    let mut y = 1970i32;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap_year(y);
    let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0usize;
    while m < 12 && remaining >= month_days[m] {
        remaining -= month_days[m];
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m + 1, remaining + 1)
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type_str() {
        assert_eq!(PacketType::Gold.as_str(), "gold");
        assert_eq!(PacketType::Diamond.as_str(), "diamond");
        assert_eq!(PacketType::Gold.currency_name(), "金币");
        assert_eq!(PacketType::Diamond.currency_name(), "钻石");
        assert_eq!(PacketType::Gold.emoji(), "💰");
        assert_eq!(PacketType::Diamond.emoji(), "💎");
    }

    #[test]
    fn test_packet_type_currency_key() {
        assert_eq!(PacketType::Gold.currency_key(), CURRENCY_GOLD);
        assert_eq!(PacketType::Diamond.currency_key(), CURRENCY_DIAMOND);
    }

    #[test]
    fn test_split_amount_single() {
        let result = split_amount(100, 1);
        assert_eq!(result, vec![100]);
    }

    #[test]
    fn test_split_amount_sum() {
        let result = split_amount(1000, 5);
        assert_eq!(result.len(), 5);
        assert_eq!(result.iter().sum::<i64>(), 1000);
    }

    #[test]
    fn test_split_amount_minimum() {
        let result = split_amount(5, 5);
        assert_eq!(result.len(), 5);
        assert_eq!(result.iter().sum::<i64>(), 5);
        for amt in &result {
            assert!(*amt >= 1);
        }
    }

    #[test]
    fn test_split_amount_zero() {
        let result = split_amount(0, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_split_amount_zero_count() {
        let result = split_amount(100, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_gen_packet_id_format() {
        let id = gen_packet_id("user123");
        assert!(id.starts_with("RP-"));
    }

    #[test]
    fn test_gen_packet_id_unique() {
        let id1 = gen_packet_id("user1");
        let id2 = gen_packet_id("user2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_serialize_parse_roundtrip() {
        let packet = RedPacket {
            id: "RP-12345-ABCDEF".to_string(),
            sender: "user1".to_string(),
            sender_name: "测试玩家".to_string(),
            packet_type: PacketType::Gold,
            total_amount: 1000,
            remaining_amount: 600,
            max_grabbers: 5,
            grabbed_count: 2,
            message: "新年快乐".to_string(),
            create_ts: 1700000000,
            expire_ts: 1700003600,
            grabs: vec![("user2".to_string(), 200), ("user3".to_string(), 200)],
        };
        let serialized = serialize_packet(&packet);
        let parsed = parse_packet_data(&serialized).unwrap();
        assert_eq!(parsed.id, packet.id);
        assert_eq!(parsed.sender, packet.sender);
        assert_eq!(parsed.packet_type, packet.packet_type);
        assert_eq!(parsed.total_amount, packet.total_amount);
        assert_eq!(parsed.remaining_amount, packet.remaining_amount);
        assert_eq!(parsed.max_grabbers, packet.max_grabbers);
        assert_eq!(parsed.grabbed_count, packet.grabbed_count);
        assert_eq!(parsed.grabs.len(), 2);
        assert_eq!(parsed.grabs[0].1, 200);
    }

    #[test]
    fn test_parse_invalid_data() {
        assert!(parse_packet_data("").is_none());
        assert!(parse_packet_data("too short").is_none());
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
        assert_eq!(format_num(-5000), "-5,000");
    }

    #[test]
    fn test_constants() {
        assert!(MIN_GOLD_AMOUNT >= 1);
        assert!(MIN_DIAMOND_AMOUNT >= 1);
        assert!(MAX_GRABBERS >= 1);
        assert!(DAILY_SEND_LIMIT >= 1);
        assert!(DAILY_GRAB_LIMIT >= 1);
        assert!(GRAB_COOLDOWN_SECS >= 1);
        assert!(PACKET_EXPIRE_SECS >= 60);
    }

    #[test]
    fn test_split_amount_large() {
        let result = split_amount(100000, 10);
        assert_eq!(result.len(), 10);
        assert_eq!(result.iter().sum::<i64>(), 100000);
        for amt in &result {
            assert!(*amt >= 1);
        }
    }

    #[test]
    fn test_today_str_format() {
        let date = today_str();
        assert_eq!(date.len(), 10);
        assert!(date.contains('-'));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn test_djb2_hash_consistency() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_djb2_hash_different() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("world");
        assert_ne!(h1, h2);
    }
}
