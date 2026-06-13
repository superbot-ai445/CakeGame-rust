mod abyss;
mod arena_shop;

mod achievement;
mod activity;
mod activity_points;
mod admin_panel;
mod adventure;
mod ai_opponent;
mod alchemy;
mod arena_tournament;
mod armor_type;
mod attr_encyclopedia;
mod attr_trial;
mod bank;
mod battle_archive;
mod beast;
mod bestiary;
mod block;
mod boss;
mod boss_respawn;
mod boss_rush;
mod boss_summon;
mod bounty;
mod buff;
mod chest;
mod collection;
mod combat;
mod combat_power;
mod combat_stats;
mod combat_style;
mod combine;
mod compare;
mod cook;
mod core;
mod costume;
mod cultivation;
mod custom_attrs;
mod custom_variable;
mod daily_challenge;
mod daily_fortune;
mod daily_quest;
mod daily_report;
mod data_checker;
mod db;
mod duel;
mod dungeon;
mod economy_panel;
mod email;
mod enchant;
mod encoding;
mod equip_awakening;
mod equip_evolution;
mod equip_loadout;
mod equip_lock;
mod equip_recommend;
mod equip_score;
mod equip_skill;
mod equip_unique;
mod examgear;
mod excavation;
mod exchange;
mod extra;
mod fishing;
mod flash_sale;
mod food;
mod friend;
mod game_calendar;
mod gather;
mod gem;
mod gm_adjust;
mod growth_config;
mod growth_fund;
mod guild_alliance;
mod guild_banquet;
mod guild_building;
mod guild_bulletin;
mod guild_checkin;
mod guild_commission;
mod guild_realm;
mod guild_skill;
mod guild_trial;
mod guild_war;
mod guild_warehouse;
mod herbseed;
mod inheritance;
mod instance;
mod lottery;
mod lucky_wheel;
mod map_collect;
mod map_explore;
mod map_resource;
mod marriage;
mod maze;
mod mentor;
mod message_board;
mod military_rank;
mod minigame;
mod monster_hunter;
mod monster_kill_log;
mod monster_model;
mod mount;
mod npc_affinity;
mod offline_reward;
mod online_activity;
mod online_reward;
mod password;
mod permissions;
mod pharmacy;
mod player_journal;
mod player_lookup;
mod pray;
mod private_shop;
mod proficiency;
mod purchase_history;
mod pvp;
mod quest;
mod quiz;
mod rebirth;
mod recycle;
mod red_packet;
mod redeem;
mod refine;
mod reforge;
mod regen;
mod report;
mod reputation;
mod respec;
mod return_reward;
mod router;
mod season;
mod season_journey;
mod season_pass;
mod sell;
mod server_milestone;
mod server_notice;
mod settings;
mod shop;
mod sign_makeup;
mod skill_analytics;
mod skill_combo;
mod skill_set;
mod smelt;
mod social;
mod special;
mod stamina;
mod super_enhance;
mod talent_tree;
mod team_goals;
mod template;
mod template_render;
mod title;
mod trade;
mod training;
mod treasure_hunt;
mod tutorial;
mod type_effect;
mod user;
mod vip;
mod wanted;
mod warehouse;
mod wealth;
mod weather;
mod weekly_quest;
mod whisper;
mod wish_pool;
mod world_boss;
mod world_chat;
mod world_event;
mod world_level;
mod world_quest;

use db::Database;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

static DB: OnceCell<Mutex<Database>> = OnceCell::new();
static ROUTER: OnceCell<Mutex<router::Router>> = OnceCell::new();

/// 初始化引擎
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_init(db_path: *const std::os::raw::c_char) -> i32 {
    let path = unsafe {
        if db_path.is_null() {
            return -1;
        }
        std::ffi::CStr::from_ptr(db_path).to_str().unwrap_or("")
    };

    let database = match Database::open(path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("打开数据库失败: {}", e);
            return -2;
        }
    };

    let prefix = database.global_get("set", "TriggerPrefix");
    let mut r = router::Router::new(&prefix);

    // 基础指令
    r.register("注册", "注册", router::cmd_register);
    r.register("签到", "签到", router::cmd_sign_in);
    r.register("签到日历", "签到日历", router::cmd_sign_calendar);
    r.register("查看角色", "查看角色", router::cmd_view_character);
    r.register("帮助", "帮助", router::cmd_help);
    r.register("查看帮助", "查看帮助", router::cmd_db_help);
    r.register("修改昵称", "修改昵称", router::cmd_rename);

    // 物品指令
    r.register("查看背包", "查看背包", router::cmd_view_knapsack);
    r.register("使用物品", "使用", router::cmd_use_item);
    r.register("丢弃物品", "丢弃", router::cmd_drop_item);

    // 装备指令
    r.register("查看装备", "查看装备", router::cmd_view_equips);
    r.register("卸下装备", "卸下", router::cmd_unequip);

    // 技能指令
    r.register("查看技能", "查看技能", router::cmd_view_skills);

    // 职业指令
    r.register("查看职业", "查看职业", router::cmd_view_occupation);
    r.register("职业列表", "职业列表", router::cmd_occupation_list);
    r.register("转换职业", "转换职业", router::cmd_change_occupation);

    // 物品指令
    r.register("背包筛选", "背包筛选", router::cmd_filter_knapsack);
    r.register("背包整理", "背包整理", router::cmd_sort_knapsack);
    r.register("查看物品", "查看物品", router::cmd_view_item);
    r.register("赠送物品", "赠送物品", router::cmd_gift);
    r.register("赠送金币", "赠送金币", router::cmd_gift);
    r.register("赠送钻石", "赠送钻石", router::cmd_gift);

    // 地图指令
    r.register("查看地图", "查看地图", router::cmd_view_maps);
    r.register("进入地图", "进入", router::cmd_enter_map);
    r.register("位置信息", "位置信息", router::cmd_location_info);

    // 战斗指令
    r.register("搜索怪物", "搜索怪物", router::cmd_search_monster);
    r.register("锁定目标", "锁定目标", router::cmd_lock_target);
    r.register("锁定怪物", "锁定怪物", router::cmd_lock_target);
    r.register("解锁目标", "解锁目标", router::cmd_unlock_target);
    r.register("攻击", "攻击", router::cmd_attack);
    r.register("自动攻击", "自动攻击", router::cmd_auto_attack);
    r.register("释放技能", "释放技能", router::cmd_use_skill);

    // 排行指令
    r.register("等级排行", "等级排行", router::cmd_view_rank);
    r.register("生命排行", "生命排行", router::cmd_view_rank);
    r.register("魔法排行", "魔法排行", router::cmd_view_rank);
    r.register("物攻排行", "物攻排行", router::cmd_view_rank);
    r.register("魔攻排行", "魔攻排行", router::cmd_view_rank);
    r.register("防御排行", "防御排行", router::cmd_view_rank);
    r.register("魔抗排行", "魔抗排行", router::cmd_view_rank);
    r.register("金币排行", "金币排行", router::cmd_view_rank);
    r.register("钻石排行", "钻石排行", router::cmd_view_rank);

    // GM 指令
    // 商店指令
    r.register("查看商店", "查看商店", router::cmd_view_shop);
    r.register("购买商品", "购买", router::cmd_buy_item);
    r.register("查看合成", "查看合成", router::cmd_view_composite);
    r.register("合成物品", "合成", router::cmd_composite);
    r.register("合成列表", "合成列表", router::cmd_composite_list);

    // 公会指令
    r.register("创建公会", "创建公会", router::cmd_create_guild);
    r.register("我的公会", "我的公会", router::cmd_my_guild);
    r.register("退出公会", "退出公会", router::cmd_leave_guild);
    r.register("解散公会", "解散公会", router::cmd_disband_guild);
    r.register("转让公会", "转让公会", router::cmd_transfer_guild);
    r.register("公会列表", "公会列表", router::cmd_guild_list);
    r.register("公会捐献", "公会捐献", router::cmd_guild_donate);
    r.register("公会成员", "公会成员", router::cmd_guild_members);
    r.register("踢出成员", "踢出成员", router::cmd_kick_guild_member);
    r.register("申请加入公会", "申请加入公会", router::cmd_apply_guild);
    r.register("批准加入", "批准加入", router::cmd_approve_guild);
    r.register("拒绝加入", "拒绝加入", router::cmd_reject_guild);

    // 匹配竞技指令
    r.register("匹配", "匹配", router::cmd_match_arena);
    r.register("匹配信息", "匹配信息", router::cmd_match_info);
    r.register("匹配排行", "匹配排行", router::cmd_match_ranking);
    r.register("加入匹配队列", "加入匹配队列", router::cmd_match_queue_join);
    r.register("退出匹配队列", "退出匹配队列", router::cmd_match_queue_leave);
    r.register("匹配队列", "匹配队列", router::cmd_match_queue_view);

    // 队伍指令
    r.register("创建队伍", "创建队伍", router::cmd_create_team);
    r.register("加入队伍", "加入队伍", router::cmd_join_team);
    r.register("退出队伍", "退出队伍", router::cmd_leave_team);
    r.register("队伍成员", "队伍成员", router::cmd_team_members);
    r.register("请出成员", "请出成员", router::cmd_kick_team_member);

    // 任务指令
    r.register("全部任务", "全部任务", quest::cmd_all_tasks);
    r.register("我的任务", "我的任务", quest::cmd_my_tasks);
    r.register("任务信息", "任务信息", quest::cmd_task_info);
    r.register("领取任务", "领取任务", quest::cmd_accept_task);
    r.register("提交任务", "提交任务", quest::cmd_submit_task);
    r.register("放弃任务", "放弃任务", quest::cmd_abandon_task);

    // PVP 指令
    r.register("锁定玩家", "锁定玩家", pvp::cmd_lock_player);
    r.register("攻击玩家", "攻击玩家", pvp::cmd_attack_player);
    r.register("查看目标", "查看目标", router::cmd_view_target);
    r.register("位置玩家", "位置玩家", router::cmd_view_map_players);
    r.register("喊话", "喊话", router::cmd_shout);

    // 小黑屋指令
    r.register("邪恶值", "邪恶值", pvp::cmd_evil_value);
    r.register("PK值", "PK值", pvp::cmd_evil_value);
    r.register("小黑屋", "小黑屋", pvp::cmd_xiaoheiwu);

    // 私人商店指令
    r.register("商店列表", "商店列表", private_shop::cmd_private_shop_list);
    r.register("进入商店", "进入商店", private_shop::cmd_enter_private_shop);
    r.register("退出商店", "退出商店", private_shop::cmd_exit_private_shop);
    r.register("我的商店", "我的商店", private_shop::cmd_my_shop);
    r.register("上架商品", "上架商品", private_shop::cmd_add_shop_item);
    r.register("下架商品", "下架商品", private_shop::cmd_remove_shop_item);
    r.register("开启商店", "开启商店", private_shop::cmd_open_shop);
    r.register("关闭商店", "关闭商店", private_shop::cmd_close_shop);
    r.register("商店改名", "商店改名", private_shop::cmd_rename_shop);

    // NPC 指令
    r.register("查看NPC", "查看NPC", extra::cmd_view_npcs);
    r.register("对话NPC", "对话", extra::cmd_talk_npc);
    r.register("使用功能", "使用功能", extra::cmd_use_npc_function);

    // NPC好感度指令
    r.register("NPC好感", "NPC好感", npc_affinity_cmd_view);
    r.register("NPC对话", "NPC对话", npc_affinity_cmd_talk);
    r.register("NPC赠礼", "NPC赠礼", npc_affinity_cmd_gift);
    r.register("NPC好感详情", "NPC好感详情", npc_affinity_cmd_detail);
    r.register("NPC好感奖励", "NPC好感奖励", npc_affinity_cmd_reward);
    r.register("NPC好感排行", "NPC好感排行", npc_affinity_cmd_ranking);

    // 分解指令
    r.register("分解物品", "分解", extra::cmd_decompose);
    r.register("查看分解", "查看分解", extra::cmd_view_decompose);

    // 强化指令
    r.register("强化装备", "强化", extra::cmd_enhance);
    r.register("查看强化信息", "查看强化信息", extra::cmd_view_enhance_info);
    r.register("查看保底", "查看保底", extra::cmd_view_pity);

    // 套装指令
    r.register("查看套装", "查看套装", extra::cmd_view_suit);

    // 地宫指令
    r.register("查看地宫", "查看地宫", dungeon::cmd_view_dungeon);
    r.register("挑战地宫", "挑战地宫", dungeon::cmd_challenge_dungeon);

    // 副本系统指令
    r.register("查看副本列表", "查看副本列表", instance::cmd_instance_list);
    r.register("查看副本", "查看副本", instance::cmd_instance_info);
    r.register("挑战副本", "挑战副本", instance::cmd_instance_challenge);
    r.register("副本进度", "副本进度", instance::cmd_instance_progress);
    r.register("副本排行", "副本排行", instance::cmd_instance_ranking);
    r.register("副本扫荡", "副本扫荡", instance::cmd_instance_sweep);

    r.register("发放奖励", "发放奖励", router::cmd_gm_reward);

    // 锻造/训练指令
    r.register("锻造列表", "锻造列表", training::cmd_training_list);
    r.register("吐纳", "吐纳", training::cmd_training_hp);
    r.register("冥想", "冥想", training::cmd_training_mp);
    r.register("练武", "练武", training::cmd_training_ad);
    r.register("习法", "习法", training::cmd_training_ap);

    // 挂机修炼指令 (auto_evolution)
    r.register("开始修炼", "开始修炼", cultivation::cmd_start_cultivation);
    r.register("停止修炼", "停止修炼", cultivation::cmd_stop_cultivation);
    r.register("修炼状态", "修炼状态", cultivation::cmd_view_cultivation);
    r.register("修炼查询", "修炼查询", cultivation::cmd_cultivation_status);

    // 野外BOSS指令
    r.register("查看BOSS", "查看BOSS", boss::cmd_view_boss);
    r.register("挑战BOSS", "挑战BOSS", boss::cmd_challenge_boss);

    // 世界BOSS突袭系统
    r.register("查看世界BOSS", "查看世界BOSS", world_boss::cmd_view_world_boss);
    r.register("挑战世界BOSS", "挑战世界BOSS", world_boss::cmd_challenge_world_boss);
    r.register("世界BOSS排名", "世界BOSS排名", world_boss::cmd_world_boss_ranking);

    // BOSS刷新计时器系统
    r.register("BOSS状态", "BOSS状态", boss_respawn::cmd_boss_status);
    r.register("BOSS刷新", "BOSS刷新", boss_respawn::cmd_boss_respawn);
    r.register("BOSS击杀记录", "BOSS击杀记录", boss_respawn::cmd_boss_kill_log);

    // Boss Rush 连续挑战系统指令
    r.register("查看Boss Rush", "查看Boss Rush", boss_rush::cmd_view_boss_rush);
    r.register("开始Boss Rush", "开始Boss Rush", boss_rush::cmd_start_boss_rush);
    r.register("Boss Rush进度", "Boss Rush进度", boss_rush::cmd_boss_rush_progress);
    r.register("Boss Rush排行", "Boss Rush排行", boss_rush::cmd_boss_rush_ranking);
    r.register("Boss Rush记录", "Boss Rush记录", boss_rush::cmd_boss_rush_history);

    // Boss召唤系统指令
    r.register("召唤列表", "召唤列表", boss_summon::cmd_summon_list);
    r.register("召唤Boss", "召唤Boss", boss_summon::cmd_summon_boss);
    r.register("召唤记录", "召唤记录", boss_summon::cmd_summon_history);
    r.register("召唤排行", "召唤排行", boss_summon::cmd_summon_ranking);

    // 赛季通行证系统指令
    r.register("查看通行证", "查看通行证", season_pass::cmd_view_season_pass);
    r.register("领取通行证奖励", "领取通行证奖励", season_pass::cmd_claim_pass_reward);
    r.register("购买高级通行证", "购买高级通行证", season_pass::cmd_buy_premium_pass);
    r.register("通行证排行", "通行证排行", season_pass::cmd_pass_ranking);
    r.register("通行证帮助", "通行证帮助", season_pass::cmd_pass_help);
    // 赛季征途系统指令
    r.register("查看征途", "查看征途", season_journey::cmd_view_season_journey);
    r.register("征途详情", "征途详情+", season_journey::cmd_journey_detail);
    r.register("领取征途奖励", "领取征途奖励+", season_journey::cmd_claim_journey_reward);
    r.register("征途排行", "征途排行", season_journey::cmd_journey_ranking);
    r.register("征途统计", "征途统计", season_journey::cmd_journey_stats);

    // 推荐打怪系统指令
    r.register("推荐打怪", "推荐打怪", router::cmd_recommend_monster);
    r.register("怪物弱点", "怪物弱点", router::cmd_monster_weakness);
    r.register("查询掉落", "查询掉落", router::cmd_query_drops);

    // 种植系统指令
    r.register("查看种子", "查看种子", herbseed::cmd_view_seeds);
    r.register("购买种子", "购买种子", herbseed::cmd_buy_seed);
    r.register("种植", "种植", herbseed::cmd_plant_seed);
    r.register("我的花园", "我的花园", herbseed::cmd_view_garden);
    r.register("收获", "收获", herbseed::cmd_harvest);
    r.register("出售药材", "出售药材", herbseed::cmd_sell_herb);
    r.register("药园仓库", "药园仓库", herbseed::cmd_view_garden_warehouse);
    r.register("铲除作物", "铲除作物", herbseed::cmd_remove_crop);
    r.register("转换灵气", "转换灵气", herbseed::cmd_convert_reiki);
    r.register("偷摸药材", "偷摸药材", herbseed::cmd_steal_herb);
    r.register("药园安全", "药园安全", herbseed::cmd_view_steal_notify);
    r.register("布置守护", "布置守护", herbseed::cmd_set_garden_guard);

    // 邮件系统指令
    r.register("查看邮件", "查看邮件", email::cmd_view_mail);
    r.register("阅读邮件", "阅读邮件", email::cmd_read_mail);
    r.register("领取附件", "领取附件", email::cmd_claim_enclosure);
    r.register("删除邮件", "删除邮件", email::cmd_delete_mail);
    r.register("发送邮件", "发送邮件", email::cmd_send_mail);
    r.register("全服邮件", "全服邮件", email::cmd_gm_send_mail);
    r.register("系统邮件", "系统邮件", email::cmd_gm_send_mail);

    // 玩家留言板系统
    r.register("写留言", "写留言", message_board::cmd_write_message);
    r.register("查看留言", "查看留言", message_board::cmd_view_messages);
    r.register("回复留言", "回复留言", message_board::cmd_reply_message);
    r.register("删除留言", "删除留言", message_board::cmd_delete_message);
    r.register("清空留言", "清空留言", message_board::cmd_clear_messages);
    r.register("我的留言", "我的留言", message_board::cmd_my_sent_messages);

    // 玩家信息查询系统
    r.register("查看玩家", "查看玩家", player_lookup::cmd_player_info);
    r.register("周围玩家", "周围玩家", player_lookup::cmd_nearby_players);

    // 玩家日志系统
    r.register("查看日志", "查看日志", player_journal::cmd_view_journal);
    r.register("日志统计", "日志统计", player_journal::cmd_journal_stats);
    r.register("日志搜索", "日志搜索", player_journal::cmd_journal_search);
    r.register("记录日志", "记录日志", player_journal::cmd_add_journal);

    // NPC出售系统指令
    r.register("出售物品", "出售", sell::cmd_sell_item);
    r.register("出售价格", "出售价格", sell::cmd_view_sell_price);
    r.register("可出售", "可出售", sell::cmd_view_sellable);
    r.register("收购列表", "收购列表", sell::cmd_view_npc_sell_list);

    // 购买记录系统
    r.register("购买记录", "购买记录", purchase_history::cmd_purchase_history);
    r.register("交易统计", "交易统计", purchase_history::cmd_transaction_stats);

    // 邀请加入公会
    r.register("邀请加入", "邀请加入", social::cmd_invite_guild);

    // 答题装备指令
    r.register("查看答题装备", "查看答题装备", examgear::cmd_view_exam_gear);
    r.register("领取答题装备", "领取答题装备", examgear::cmd_claim_exam_gear);

    // 夜市指令
    r.register("查看夜市", "查看夜市", router::cmd_view_yesi);

    // 超界强化指令
    r.register("查看超界", "查看超界", super_enhance::cmd_view_super_enhance);
    r.register("超界强化", "超界强化", super_enhance::cmd_super_enhance);
    r.register("超界图鉴", "超界图鉴", super_enhance::cmd_es_codex);

    // 装备精炼系统指令
    r.register("查看精炼", "查看精炼", refine::cmd_view_refine);
    r.register("精炼升级", "精炼升级", refine::cmd_refine_upgrade);
    r.register("精炼属性", "精炼属性", refine::cmd_refine_attr);
    r.register("精炼等级", "精炼等级", refine::cmd_refine_levels);
    r.register("精炼排行", "精炼排行", refine::cmd_refine_ranking);

    // 采集系统指令
    r.register("采集信息", "采集信息", gather::cmd_view_gather);
    r.register("采集", "采集", gather::cmd_gather);
    r.register("采集统计", "采集统计", gather::cmd_gather_stats);
    r.register("副职", "副职", gather::cmd_view_subprofession);
    r.register("采集排行", "采集排行", gather::cmd_gather_ranking);
    r.register("采集排行榜", "采集排行榜", gather::cmd_gather_ranking);

    // 匹配竞技指令（重复已移除）

    // 装备技能指令
    r.register("查看装备技能", "查看装备技能", equip_skill::cmd_view_equip_skills);
    r.register("查看持续技能", "查看持续技能", equip_skill::cmd_view_continuous_skills);
    r.register("活跃效果", "活跃效果", equip_skill::cmd_view_active_effects);
    r.register("查看活跃效果", "查看活跃效果", equip_skill::cmd_view_active_effects);

    // 装备特技指令 (Config_Equipskills 表 — 专属武器技能)
    r.register("装备特技", "装备特技", equip_unique::cmd_view_equip_unique_skills);
    r.register("特技详情", "特技详情", equip_unique::cmd_equip_unique_info);
    r.register("使用装备特技", "使用装备特技", equip_unique::cmd_use_equip_unique_skill);

    // 装备评分系统
    r.register("装备评分", "装备评分", equip_score::cmd_equip_score);

    // 装备推荐系统
    r.register("推荐装备", "推荐装备", equip_recommend::cmd_recommend_equip);

    // 装备锁定系统
    r.register("锁定装备", "锁定装备", equip_lock::cmd_lock_equip);
    r.register("解锁装备", "解锁装备", equip_lock::cmd_unlock_equip);
    r.register("查看锁定", "查看锁定", equip_lock::cmd_view_locks);

    // 装备进化系统
    r.register("查看进化", "查看进化", equip_evolution::cmd_view_evolution);
    r.register("装备进化", "装备进化", equip_evolution::cmd_evolve_equipment);
    r.register("进化预览", "进化预览", equip_evolution::cmd_evolution_preview);

    // 装备觉醒系统
    r.register("查看觉醒", "查看觉醒", equip_awakening::cmd_view_awakening);
    r.register("觉醒预览", "觉醒预览", equip_awakening::cmd_awakening_preview);
    r.register("装备觉醒", "装备觉醒", equip_awakening::cmd_awaken_equipment);
    r.register("觉醒排行", "觉醒排行", equip_awakening::cmd_awakening_ranking);
    r.register("觉醒图鉴", "觉醒图鉴", equip_awakening::cmd_awakening_codex);

    // 装备重铸系统
    r.register("查看重铸", "查看重铸", reforge::cmd_view_reforge);
    r.register("装备重铸", "装备重铸", reforge::cmd_reforge_equip);
    r.register("重铸预览", "重铸预览", reforge::cmd_reforge_preview);
    r.register("重铸记录", "重铸记录", reforge::cmd_reforge_record);

    // 装备方案系统
    r.register("保存方案", "保存方案", equip_loadout::cmd_save_loadout);
    r.register("查看方案", "查看方案", equip_loadout::cmd_view_loadouts);
    r.register("方案详情", "方案详情", equip_loadout::cmd_loadout_info);
    r.register("加载方案", "加载方案", equip_loadout::cmd_load_loadout);
    r.register("删除方案", "删除方案", equip_loadout::cmd_delete_loadout);
    r.register("方案对比", "方案对比", equip_loadout::cmd_compare_loadouts);

    // 两步确认指令
    r.register("选择强化", "选择强化", extra::cmd_select_enhance);
    r.register("确认强化", "确认强化", extra::cmd_confirm_enhance);
    r.register("选择分解", "选择分解", extra::cmd_select_decompose);
    r.register("确认分解", "确认分解", extra::cmd_confirm_decompose);

    // 查看当前商店
    r.register("查看当前商店", "查看当前商店", extra::cmd_view_current_shops);

    // 食物系统指令
    r.register("查看食物", "查看食物", food::cmd_view_foods);
    r.register("购买食物", "购买食物", food::cmd_buy_food);
    r.register("使用食物", "使用食物", food::cmd_use_food);
    r.register("我的食物", "我的食物", food::cmd_my_foods);
    r.register("食物统计", "食物统计", food::cmd_food_stats);
    r.register("饥饿状态", "饥饿状态", food::cmd_hunger_status);
    r.register("饱食度", "饱食度", food::cmd_hunger_status);

    // 商品筛选
    r.register("商品筛选", "商品筛选", router::cmd_filter_shop_goods);

    // 自动查看
    r.register("自动查看", "自动查看", router::cmd_auto_view_info);
    r.register("查看详细信息", "查看详细信息", router::cmd_view_detailed_info);

    // 获取验证信息
    r.register("获取验证信息", "获取验证信息", router::cmd_get_verify_info);

    // 护盾系统指令
    r.register("查看护盾", "查看护盾", router::cmd_view_shield);
    r.register("获取护盾", "获取护盾", router::cmd_get_shield);

    // 烹饪系统指令
    r.register("查看烹饪", "查看烹饪", cook::cmd_view_cooking);
    r.register("烹饪", "烹饪", cook::cmd_cook);
    r.register("可烹饪", "可烹饪", cook::cmd_available_cooking);

    // 赏金任务指令
    r.register("赏金任务", "赏金任务", bounty::cmd_view_bounties);
    r.register("赏金板", "赏金板", bounty::cmd_view_bounties);
    r.register("接受赏金", "接受赏金", bounty::cmd_accept_bounty);
    r.register("赏金进度", "赏金进度", bounty::cmd_bounty_progress);
    r.register("提交赏金", "提交赏金", bounty::cmd_submit_bounty);
    r.register("放弃赏金", "放弃赏金", bounty::cmd_abandon_bounty);

    // 扩展技能指令
    r.register("扩展技能", "扩展技能", skill_set::cmd_list_ext_skills);
    r.register("查看技能详情", "查看技能详情", skill_set::cmd_view_ext_skill);

    // 制药系统指令
    r.register("查看制药", "查看制药", pharmacy::cmd_view_pharmacy);
    r.register("制药", "制药", pharmacy::cmd_craft_medicine);
    r.register("可制药", "可制药", pharmacy::cmd_available_pharmacy);

    // 钱庄系统指令
    r.register("查看钱庄", "查看钱庄", bank::cmd_view_bank);
    r.register("存款", "存款", bank::cmd_deposit);
    r.register("取款", "取款", bank::cmd_withdraw);
    r.register("我的存款", "我的存款", bank::cmd_my_deposit);

    // 宝石镶嵌系统指令
    r.register("查看宝石", "查看宝石", gem::cmd_view_gems);
    r.register("宝石背包", "宝石背包", gem::cmd_gem_inventory);
    r.register("镶嵌宝石", "镶嵌宝石", gem::cmd_socket_gem);
    r.register("卸下宝石", "卸下宝石", gem::cmd_unsocket_gem);
    r.register("合成宝石", "合成宝石", gem::cmd_merge_gem);
    r.register("宝石属性", "宝石属性", gem::cmd_gem_stats);

    // 挖掘系统指令
    r.register("查看挖掘", "查看挖掘", excavation::cmd_view_dig_sites);
    r.register("开始挖掘", "开始挖掘", excavation::cmd_excavate);
    r.register("挖掘背包", "挖掘背包", excavation::cmd_excavation_bag);
    r.register("挖掘排行", "挖掘排行", excavation::cmd_excavation_ranking);
    r.register("挖掘图鉴", "挖掘图鉴", excavation::cmd_excavation_codex);

    // 怪物图鉴指令
    r.register("怪物图鉴", "怪物图鉴", bestiary::cmd_bestiary);
    r.register("查看怪物", "查看怪物", bestiary::cmd_bestiary);

    // 属性克制指令
    r.register("查看属性克制", "查看属性克制", router::cmd_view_type_effect);
    r.register("类型图鉴", "类型图鉴", router::cmd_type_chart);
    r.register("职业类型", "职业类型", router::cmd_occupation_types);
    r.register("系统基础属性", "系统基础属性", router::cmd_view_system_attrs);
    r.register("全服统计", "全服统计", router::cmd_server_stats);

    // 全服活动系统指令
    r.register("全服活动", "全服活动", world_event::cmd_view_world_events);
    r.register("创建活动", "创建活动", world_event::cmd_create_world_event);
    r.register("结束活动", "结束活动", world_event::cmd_end_world_event);

    // 全服世界任务系统
    r.register("世界任务", "世界任务", world_quest::cmd_world_quest);
    r.register("世界任务排行", "世界任务排行", world_quest::cmd_world_quest_ranking);
    r.register("世界任务记录", "世界任务记录", world_quest::cmd_world_quest_history);

    // 战力评分指令
    r.register("查看战力", "查看战力", router::cmd_combat_power);
    r.register("战力", "战力", router::cmd_combat_power);

    // 自动修炼指令
    r.register("开始修炼", "开始修炼", training::cmd_auto_evo_start);
    r.register("停止修炼", "停止修炼", training::cmd_auto_evo_stop);
    r.register("修炼状态", "修炼状态", training::cmd_auto_evo_status);

    // 虚弱/死亡惩罚系统指令
    r.register("虚弱状态", "虚弱状态", user::cmd_weakness_status);

    // VIP会员系统指令
    r.register("VIP信息", "VIP信息", vip::cmd_vip_info);
    r.register("VIP签到", "VIP签到", vip::cmd_vip_sign_in);
    r.register("VIP充值", "VIP充值", vip::cmd_vip_recharge);
    r.register("首冲奖励", "首冲奖励", vip::cmd_first_recharge);
    r.register("累计充值", "累计充值", vip::cmd_cumulative_recharge);

    // 成就系统指令
    r.register("成就列表", "成就列表", achievement::cmd_achievement_list);
    r.register("我的成就", "我的成就", achievement::cmd_my_achievements);
    r.register("领取成就", "领取成就", achievement::cmd_claim_achievement);
    r.register("成就排行", "成就排行", achievement::cmd_achievement_ranking);
    r.register("成就商店", "成就商店", achievement::cmd_achievement_shop);
    r.register("成就兑换", "成就兑换", achievement::cmd_achievement_exchange);
    r.register("兑换统计", "兑换统计", achievement::cmd_achievement_exchange_stats);

    // 熔炼系统指令
    r.register("查看熔炼", "查看熔炼", smelt::cmd_view_smelt);
    r.register("熔炼", "熔炼", smelt::cmd_smelt);
    r.register("可熔炼", "可熔炼", smelt::cmd_smeltable);

    // 增益/减益系统指令
    r.register("查看增益", "查看增益", buff::cmd_view_buffs);
    r.register("增益信息", "增益信息", buff::cmd_buff_info);

    // 竞技扩展指令
    r.register("我的战绩", "我的战绩", pvp::cmd_my_pvp_record);
    r.register("罪恶值排行", "罪恶值排行", pvp::cmd_evil_ranking);
    r.register("被杀记录", "被杀记录", pvp::cmd_victim_log);

    // 装备图鉴
    r.register("装备图鉴", "装备图鉴", router::cmd_equipment_codex);

    // 战力排行榜
    r.register("战力排行", "战力排行", router::cmd_power_ranking);
    r.register("战力排行榜", "战力排行榜", router::cmd_power_ranking);

    // 公会捐赠排行
    r.register("公会捐赠排行", "公会捐赠排行", social::cmd_guild_donation_ranking);

    // 好友系统指令
    r.register("添加好友", "添加好友", friend::cmd_add_friend);
    r.register("删除好友", "删除好友", friend::cmd_remove_friend);
    r.register("好友列表", "好友列表", friend::cmd_friend_list);
    r.register("查看好友", "查看好友", friend::cmd_view_friend);

    // 玩家屏蔽系统指令
    r.register("屏蔽玩家", "屏蔽玩家", block::cmd_block_player);
    r.register("解除屏蔽", "解除屏蔽", block::cmd_unblock_player);
    r.register("屏蔽列表", "屏蔽列表", block::cmd_block_list);

    // 帮助中心
    r.register("帮助中心", "帮助中心", router::cmd_help_center);

    // 设置管理员
    r.register("设置管理员", "设置管理员", router::cmd_set_admin);

    // 权限等级系统
    r.register("查看权限", "查看权限", permissions::cmd_view_permission);
    r.register("权限列表", "权限列表", permissions::cmd_permission_list);
    r.register("设置权限", "设置权限", permissions::cmd_set_permission);
    r.register("系统属性", "系统属性", permissions::cmd_system_attrs);

    // 炼制系统指令
    r.register("查看炼制", "查看炼制", alchemy::cmd_view_alchemy);
    r.register("炼制", "炼制", alchemy::cmd_alchemy);
    r.register("可炼制", "可炼制", alchemy::cmd_alchemiable);

    // 竞技场锦标赛系统指令
    r.register("查看锦标赛", "查看锦标赛", arena_tournament::cmd_view_tournament);
    r.register("报名锦标赛", "报名锦标赛", arena_tournament::cmd_register_tournament);
    r.register("锦标赛对阵", "锦标赛对阵", arena_tournament::cmd_tournament_bracket);
    r.register("锦标赛排行", "锦标赛排行", arena_tournament::cmd_tournament_ranking);
    r.register("锦标赛历史", "锦标赛历史", arena_tournament::cmd_tournament_history);

    // 签到排行榜
    r.register("签到排行", "签到排行", router::cmd_sign_in_ranking);
    r.register("签到排行榜", "签到排行榜", router::cmd_sign_in_ranking);

    // 搜索商品 - 跨商店搜索
    r.register("搜索商品", "搜索商品", private_shop::cmd_search_shop_item);

    // 特技系统指令
    r.register("查看特技", "查看特技", special::cmd_view_specials);
    r.register("使用特技", "使用特技", special::cmd_use_special);

    // 战斗风格系统指令
    r.register("查看战斗风格", "查看战斗风格", combat_style::cmd_view_combat_style);
    r.register("切换战斗风格", "切换战斗风格", combat_style::cmd_switch_combat_style);
    r.register("战斗风格详情", "战斗风格详情", combat_style::cmd_combat_style_info);

    // 宝箱系统
    r.register("查看宝箱", "查看宝箱", chest::cmd_view_chests);
    r.register("开启宝箱", "开启宝箱", chest::cmd_open_chest);
    r.register("宝箱钥匙", "宝箱钥匙", chest::cmd_key_shop);

    // 段位排行
    r.register("段位排行", "段位排行", pvp::cmd_tier_ranking);
    r.register("赎罪减免", "赎罪减免", pvp::cmd_atonement_info);
    r.register("确认赎罪", "确认赎罪", pvp::cmd_confirm_atonement);

    // 每日任务系统
    r.register("每日任务", "每日任务", daily_quest::cmd_view_daily_quests);
    r.register("领取每日", "领取每日", daily_quest::cmd_claim_daily_quest);

    // 每日挑战系统
    r.register("日常挑战", "日常挑战", daily_challenge::cmd_daily_challenge);
    r.register("领取挑战奖励", "领取挑战奖励", daily_challenge::cmd_claim_challenge);
    r.register("挑战进度", "挑战进度", daily_challenge::cmd_challenge_progress);
    r.register("挑战排行", "挑战排行", daily_challenge::cmd_challenge_ranking);

    // 每日活跃度积分系统
    r.register("活跃度", "活跃度", activity_points::cmd_view_activity);
    r.register(
        "领取活跃奖励",
        "领取活跃奖励",
        activity_points::cmd_claim_activity_reward,
    );
    r.register("活跃排行", "活跃排行", activity_points::cmd_activity_ranking);

    // 周常任务系统
    r.register("周常任务", "周常任务", weekly_quest::cmd_view_weekly_quests);
    r.register("领取周常", "领取周常", weekly_quest::cmd_claim_weekly_quest);

    // 每日祈福系统
    r.register("查看祈福", "查看祈福", pray::cmd_view_pray);
    r.register("祈福", "祈福", pray::cmd_pray);
    r.register("祈福排行", "祈福排行", pray::cmd_pray_ranking);

    // 抽奖系统
    r.register("查看抽奖", "查看抽奖", lottery::cmd_view_lottery);
    r.register("抽奖", "抽奖", lottery::cmd_lottery);
    r.register("十连抽", "十连抽", lottery::cmd_lottery_ten);
    r.register("抽奖记录", "抽奖记录", lottery::cmd_lottery_history);
    r.register("保底信息", "保底信息", lottery::cmd_pity_info);
    r.register("暗黑抽奖", "暗黑抽奖", lottery::cmd_dark_lottery);
    r.register("dark抽奖", "dark抽奖", lottery::cmd_dark_lottery);
    // 每日幸运转盘系统
    r.register("查看转盘", "查看转盘", lucky_wheel::cmd_view_lucky_wheel);
    r.register("转动转盘", "转动转盘", lucky_wheel::cmd_spin_lucky_wheel);
    r.register("转盘记录", "转盘记录", lucky_wheel::cmd_wheel_history);

    // 称号系统
    r.register("查看称号", "查看称号", title::cmd_view_titles);
    r.register("装备称号", "装备称号", title::cmd_equip_title);
    r.register("卸下称号", "卸下称号", title::cmd_unequip_title);

    // 地图资源总览
    r.register("地图资源", "地图资源", map_resource::cmd_map_resource);
    // 地图资源采集扩展
    r.register("地图草药", "地图草药", map_collect::cmd_map_herbs);
    r.register("地图矿石", "地图矿石", map_collect::cmd_map_ores);
    r.register("采集草药", "采集草药", map_collect::cmd_map_collect);
    r.register("采集矿石", "采集矿石", map_collect::cmd_map_collect);

    // 地图探索进度系统
    r.register("探索进度", "探索进度", map_explore::cmd_view_explore);
    r.register("探索详情", "探索详情", map_explore::cmd_view_explore);
    r.register("探索奖励", "探索奖励", map_explore::cmd_claim_explore_reward);
    r.register("探索排行", "探索排行", map_explore::cmd_explore_ranking);

    // 每日活跃度系统
    r.register("查看活跃", "查看活跃", activity::cmd_view_activity);
    r.register("领取活跃", "领取活跃", activity::cmd_claim_activity);
    r.register("活跃兑换", "活跃兑换", exchange::cmd_activity_exchange);

    // 每日限时折扣系统
    r.register("查看折扣", "查看折扣", flash_sale::cmd_view_flash_sale);
    r.register("购买折扣", "购买折扣", flash_sale::cmd_buy_flash_sale);
    r.register("折扣统计", "折扣统计", flash_sale::cmd_flash_sale_stats);

    // 灵兽驯养系统
    r.register("查看灵兽", "查看灵兽", beast::cmd_view_beasts);
    r.register("捕获灵兽", "捕获灵兽", beast::cmd_capture_beast);
    r.register("我的灵兽", "我的灵兽", beast::cmd_my_beasts);
    r.register("灵兽出战", "灵兽出战", beast::cmd_set_active_beast);
    r.register("灵兽喂食", "灵兽喂食", beast::cmd_feed_beast);
    r.register("灵兽进化", "灵兽进化", beast::cmd_evolve_beast);
    r.register("灵兽图鉴", "灵兽图鉴", beast::cmd_beast_codex);
    r.register("放生灵兽", "放生灵兽", beast::cmd_release_beast);

    // 坐骑系统
    r.register("查看坐骑", "查看坐骑", mount::cmd_view_mount);
    r.register("坐骑列表", "坐骑列表", mount::cmd_mount_list);
    r.register("骑乘", "骑乘", mount::cmd_mount_ride);
    r.register("下骑", "下骑", mount::cmd_mount_dismount);
    r.register("喂养坐骑", "喂养坐骑", mount::cmd_feed_mount);
    r.register("坐骑进化", "坐骑进化", mount::cmd_mount_evolve);
    r.register("坐骑排行", "坐骑排行", mount::cmd_mount_ranking);

    // 兑换码系统
    r.register("生成兑换码", "生成兑换码", redeem::cmd_create_redeem);
    r.register("兑换码", "兑换码", redeem::cmd_redeem_code);
    r.register("兑换码列表", "兑换码列表", redeem::cmd_redeem_list);

    // 被动回复系统
    r.register("被动回复", "被动回复", regen::cmd_view_regen);

    // 全服公告系统
    r.register("发布公告", "发布公告", router::cmd_post_announcement);
    r.register("查看公告", "查看公告", router::cmd_view_announcements);
    r.register("删除公告", "删除公告", router::cmd_delete_announcement);

    // 私聊系统
    r.register("私聊", "私聊", whisper::cmd_whisper_send);
    r.register("查看私聊", "查看私聊", whisper::cmd_whisper_inbox);
    r.register("私聊记录", "私聊记录", whisper::cmd_whisper_history);
    r.register("删除私聊", "删除私聊", whisper::cmd_whisper_delete);

    // 在线奖励系统
    r.register("在线奖励", "在线奖励", online_reward::cmd_online_reward);
    r.register("领取在线", "领取在线", online_reward::cmd_claim_online_reward);

    // 在线活跃统计系统
    r.register("在线排行", "在线排行", online_activity::cmd_online_ranking);
    r.register("活跃排行", "活跃排行", online_activity::cmd_online_ranking);
    r.register("在线统计", "在线统计", online_activity::cmd_online_stats);
    r.register("活跃信息", "活跃信息", online_activity::cmd_activity_info);

    // 师徒系统指令
    r.register("拜师", "拜师", mentor::cmd_apprentice_request);
    r.register("收徒列表", "收徒列表", mentor::cmd_apprentice_list);
    r.register("同意收徒", "同意收徒", mentor::cmd_accept_apprentice);
    r.register("拒绝收徒", "拒绝收徒", mentor::cmd_reject_apprentice);
    r.register("逐师", "逐师", mentor::cmd_dismiss_apprentice);
    r.register("出师", "出师", mentor::cmd_graduate);
    r.register("师徒", "师徒", mentor::cmd_view_mentor);

    // 收集图鉴系统
    r.register("收集图鉴", "收集图鉴", collection::cmd_view_collection);
    r.register("收集统计", "收集统计", collection::cmd_collection_stats);

    // 地图传送系统
    r.register("地图传送", "地图传送", router::cmd_map_teleport);

    // 附魔系统
    r.register("查看附魔", "查看附魔", enchant::cmd_view_enchant);
    r.register("附魔", "附魔", enchant::cmd_enchant);
    r.register("可附魔", "可附魔", enchant::cmd_enchantable);

    // 玩家对比系统
    r.register("对比", "对比", compare::cmd_player_compare);
    r.register("装备对比", "装备对比", compare::cmd_item_compare);

    // GM属性调整系统
    r.register("属性调整", "属性调整", gm_adjust::cmd_gm_adjust_attr);

    // 装备品质鉴定系统
    r.register("鉴定装备", "鉴定装备", extra::cmd_appraise_equip);
    r.register("查看鉴定", "查看鉴定", extra::cmd_view_appraise);

    // 装备合成系统
    r.register("装备合成配方", "装备合成配方", combine::cmd_view_combine_list);
    r.register("装备合成列表", "装备合成列表", combine::cmd_view_combine_list);
    r.register("合成详情", "合成详情", combine::cmd_view_combine_detail);
    r.register("装备合成", "装备合成", combine::cmd_combine_equip);
    r.register("我的合成", "我的合成", combine::cmd_my_combine_bonus);

    // 装备类型系统
    r.register("装备类型", "装备类型", armor_type::cmd_view_armor_types);

    // 仓库系统
    r.register("查看仓库", "查看仓库", warehouse::cmd_view_warehouse);
    r.register("存入仓库", "存入仓库", warehouse::cmd_warehouse_deposit);
    r.register("取出仓库", "取出仓库", warehouse::cmd_warehouse_withdraw);
    r.register("仓库详情", "仓库详情", warehouse::cmd_warehouse_info);
    r.register("仓库升级", "仓库升级", warehouse::cmd_warehouse_upgrade);

    // 战斗统计 & 怪物进化系统 (Ext_Monster_hzxyx 2002条数据激活)
    r.register("战斗统计", "战斗统计", combat_stats::cmd_combat_stats);
    r.register("战斗日志", "战斗日志", combat_stats::cmd_combat_log);
    r.register("怪物进化", "怪物进化", combat_stats::cmd_monster_evolution);

    // 战斗战报档案系统
    r.register("战斗战报", "战斗战报", battle_archive::cmd_view_battle_log);
    r.register("战报统计", "战报统计", battle_archive::cmd_battle_stats);
    r.register("战报排行", "战报排行", battle_archive::cmd_battle_ranking);

    // 战力评分详情系统
    r.register("战力详情", "战力详情", combat_power::cmd_combat_power_detail);
    r.register("战力评分", "战力评分", combat_power::cmd_combat_power_detail);

    // 离线收益系统
    r.register("离线收益", "离线收益", offline_reward::cmd_view_offline_reward);
    r.register("领取离线收益", "领取离线收益", offline_reward::cmd_claim_offline_reward);

    // 技能熟练度系统
    r.register("查看熟练度", "查看熟练度", proficiency::cmd_view_proficiency);
    r.register("技能熟练度", "技能熟练度", proficiency::cmd_proficiency_detail);
    r.register("熟练度排行", "熟练度排行", proficiency::cmd_proficiency_ranking);
    r.register("熟练度加成", "熟练度加成", proficiency::cmd_proficiency_bonus);

    // 技能热度分析系统
    r.register("技能热度", "技能热度", skill_analytics::cmd_skill_popularity);
    r.register("职业技能", "职业技能", skill_analytics::cmd_occupation_skills);
    r.register("技能大师", "技能大师", skill_analytics::cmd_skill_masters);

    // 属性重置系统
    r.register("属性重置", "属性重置", respec::cmd_view_respec);
    r.register("确认重置", "确认重置", respec::cmd_confirm_respec);
    r.register("重置记录", "重置记录", respec::cmd_respec_history);

    // 公会试炼系统
    r.register("公会试炼", "公会试炼", guild_trial::cmd_guild_trial);
    r.register("挑战试炼", "挑战试炼", guild_trial::cmd_challenge_trial);
    r.register("试炼进度", "试炼进度", guild_trial::cmd_trial_progress);
    // 公会战系统
    r.register("查看公会战", "查看公会战", guild_war::cmd_view_guild_war);
    r.register("发起公会战", "发起公会战", guild_war::cmd_declare_guild_war);
    r.register("参与公会战", "参与公会战", guild_war::cmd_join_guild_war);
    r.register("公会战排名", "公会战排名", guild_war::cmd_guild_war_ranking);
    r.register("公会战奖励", "公会战奖励", guild_war::cmd_guild_war_reward);

    // 公会仓库系统 (guild_warehouse.rs)
    r.register(
        "查看公会仓库",
        "查看公会仓库",
        guild_warehouse::cmd_view_guild_warehouse,
    );
    r.register(
        "存入公会仓库",
        "存入公会仓库",
        guild_warehouse::cmd_deposit_guild_warehouse,
    );
    r.register(
        "取出公会仓库",
        "取出公会仓库",
        guild_warehouse::cmd_withdraw_guild_warehouse,
    );
    r.register(
        "公会仓库信息",
        "公会仓库信息",
        guild_warehouse::cmd_guild_warehouse_info,
    );
    r.register(
        "公会仓库扩容",
        "公会仓库扩容",
        guild_warehouse::cmd_expand_guild_warehouse,
    );
    r.register("公会仓库日志", "公会仓库日志", guild_warehouse::cmd_guild_warehouse_log);

    // 公会公告系统 (guild_bulletin.rs)
    r.register("发布公告", "发布公告", guild_bulletin::cmd_post_bulletin);
    r.register("查看公会公告", "查看公会公告", guild_bulletin::cmd_view_bulletins);
    r.register("删除公告", "删除公告", guild_bulletin::cmd_delete_bulletin);
    r.register("置顶公告", "置顶公告", guild_bulletin::cmd_pin_bulletin);
    r.register("公告统计", "公告统计", guild_bulletin::cmd_bulletin_stats);

    // 公会每日签到系统 (guild_checkin.rs)
    r.register("公会签到", "公会签到", guild_checkin::cmd_guild_checkin);
    r.register("签到状态", "签到状态", guild_checkin::cmd_checkin_status);
    r.register("签到记录", "签到记录", guild_checkin::cmd_checkin_ranking);

    // 公会每日委托系统 (guild_commission.rs)
    r.register("公会委托", "公会委托", guild_commission::cmd_view_commissions);
    r.register("接受委托", "接受委托", guild_commission::cmd_accept_commission);
    r.register("提交委托", "提交委托", guild_commission::cmd_submit_commission);
    r.register("委托进度", "委托进度", guild_commission::cmd_commission_progress);
    r.register("委托排行", "委托排行", guild_commission::cmd_commission_ranking);

    // 公会科技系统 (guild_skill.rs)
    r.register("公会科技", "公会科技", guild_skill::cmd_view_guild_tech);
    r.register("科技详情", "科技详情", guild_skill::cmd_tech_detail);
    r.register("研究科技", "研究科技", guild_skill::cmd_research_tech);
    r.register("科技贡献", "科技贡献", guild_skill::cmd_tech_contribute);

    // 怪物模板系统 (Monster_Model 表 EAV 自定义怪物)
    r.register("怪物模板", "怪物模板", monster_model::cmd_view_models);
    r.register("怪物详情", "怪物详情", monster_model::cmd_view_model_detail);
    r.register("创建怪物模板", "创建怪物模板", monster_model::cmd_create_model);
    r.register("修改怪物模板", "修改怪物模板", monster_model::cmd_edit_model);
    r.register("删除怪物模板", "删除怪物模板", monster_model::cmd_delete_model);
    r.register("模板统计", "模板统计", monster_model::cmd_model_stats);

    // 怪物/职业成长配置系统 (growth_config.rs)
    r.register("查看怪物成长", "查看怪物成长", growth_config::cmd_view_monster_growth);
    r.register(
        "查看职业成长",
        "查看职业成长",
        growth_config::cmd_view_occupation_growth,
    );
    r.register("修改怪物成长", "修改怪物成长", growth_config::cmd_set_monster_growth);
    r.register("修改职业成长", "修改职业成长", growth_config::cmd_set_occupation_growth);
    r.register("属性类别列表", "属性类别列表", growth_config::cmd_view_attribute_sets);

    // 无尽深渊系统指令
    r.register("查看深渊", "查看深渊", abyss::cmd_view_abyss);
    r.register("挑战深渊", "挑战深渊", abyss::cmd_challenge_abyss);
    r.register("深渊进度", "深渊进度", abyss::cmd_abyss_progress);
    r.register("深渊排行", "深渊排行", abyss::cmd_abyss_ranking);
    r.register("重置深渊", "重置深渊", abyss::cmd_reset_abyss);

    // 竞技场AI对手系统
    r.register("查看AI对手", "查看AI对手", ai_opponent::cmd_view_ai_opponents);
    r.register("挑战AI对手", "挑战AI对手", ai_opponent::cmd_challenge_ai);
    r.register("AI对手战绩", "AI对手战绩", ai_opponent::cmd_ai_arena_record);
    r.register("AI对手排行", "AI对手排行", ai_opponent::cmd_ai_arena_ranking);

    // 竞技场赛季系统
    r.register("查看赛季", "查看赛季", season::cmd_view_season);
    r.register("赛季排行", "赛季排行", season::cmd_season_ranking);
    r.register("赛季历史", "赛季历史", season::cmd_season_history);
    r.register("领取赛季奖励", "领取赛季奖励", season::cmd_claim_season_reward);
    r.register("重置赛季", "重置赛季", season::cmd_reset_season);

    // 技能连招系统
    r.register("查看连招", "查看连招", skill_combo::cmd_view_combos);
    r.register("连招信息", "连招信息", skill_combo::cmd_combo_info);
    r.register("连招记录", "连招记录", skill_combo::cmd_combo_record);

    // 拍卖行系统
    r.register("拍卖行", "拍卖行", auction::cmd_view_auction);
    r.register("上架拍卖", "上架拍卖", auction::cmd_list_auction);
    r.register("下架拍卖", "下架拍卖", auction::cmd_cancel_auction);
    r.register("购买拍卖", "购买拍卖", auction::cmd_buy_auction);
    r.register("搜索拍卖", "搜索拍卖", auction::cmd_search_auction);
    r.register("我的拍卖", "我的拍卖", auction::cmd_cancel_auction);

    // 转生系统
    r.register("转生信息", "转生信息", rebirth::cmd_rebirth_info);
    r.register("执行转生", "执行转生", rebirth::cmd_rebirth_execute);
    r.register("转生排行", "转生排行", rebirth::cmd_rebirth_ranking);

    // 消息模板系统
    r.register("查看模板", "查看模板", router::cmd_view_templates);

    // 属性百科系统
    r.register("属性百科", "属性百科", attr_encyclopedia::cmd_attr_encyclopedia);
    r.register("属性详情", "属性详情", attr_encyclopedia::cmd_attr_encyclopedia);

    // 属性试炼系统
    r.register("查看试炼", "查看试炼", attr_trial::cmd_view_attr_trials);
    r.register("挑战试炼", "挑战试炼", attr_trial::cmd_challenge_attr_trial);
    r.register("试炼排行", "试炼排行", attr_trial::cmd_attr_trial_ranking);

    // 自定义属性引擎 (CustomAttributes_Register 表激活)
    r.register("自定义属性", "自定义属性", custom_attrs::cmd_view_custom_attrs);
    r.register("属性来源", "属性来源", custom_attrs::cmd_view_custom_attrs);
    r.register("查看属性调整", "查看属性调整", custom_attrs::cmd_view_attr_adjustments);
    r.register("属性调整值", "属性调整值", custom_attrs::cmd_add_attr_adjustment);

    // 装备强化排行榜系统
    r.register("强化排行", "强化排行", router::cmd_enhance_ranking);

    // 小游戏系统
    r.register("猜拳", "猜拳", minigame::cmd_rps);
    r.register("掷骰子", "掷骰子", minigame::cmd_dice);
    r.register("猜数字", "猜数字", minigame::cmd_guess);
    r.register("游戏统计", "游戏统计", minigame::cmd_minigame_stats);

    // 队伍讨伐目标系统
    r.register("讨伐目标", "讨伐目标", team_goals::cmd_view_team_goals);
    r.register("我的讨伐目标", "我的讨伐目标", team_goals::cmd_my_team_goals);
    r.register("讨伐进度", "讨伐进度", team_goals::cmd_team_goal_progress);

    // 装备传承系统
    r.register("查看传承", "查看传承", inheritance::cmd_view_inherit);
    r.register("传承预览", "传承预览", inheritance::cmd_inherit_preview);
    r.register("装备传承", "装备传承", inheritance::cmd_inherit);

    // 每日答题挑战系统
    r.register("开始答题", "开始答题", quiz::cmd_start_quiz);
    r.register("答题", "答题", quiz::cmd_answer_quiz);
    r.register("答题排行", "答题排行", quiz::cmd_quiz_ranking);
    r.register("我的答题记录", "我的答题记录", quiz::cmd_my_quiz);

    // 装备评分排行榜系统
    r.register("装备评分排行", "装备评分排行", router::cmd_equip_score_ranking);

    // 幻境迷宫系统
    r.register("查看迷宫", "查看迷宫", maze::cmd_view_maze);
    r.register("进入迷宫", "进入迷宫", maze::cmd_enter_maze);
    r.register("选择路径", "选择路径", maze::cmd_choose_path);
    r.register("迷宫进度", "迷宫进度", maze::cmd_maze_progress);
    r.register("迷宫排行", "迷宫排行", maze::cmd_maze_ranking);

    // 密码系统
    r.register("设置密码", "设置密码", password::cmd_set_password);
    r.register("修改密码", "修改密码", password::cmd_change_password);
    r.register("验证密码", "验证密码", password::cmd_verify_password);
    r.register("清除密码", "清除密码", password::cmd_clear_password);
    r.register("密码状态", "密码状态", password::cmd_password_status);

    // 玩家设置系统
    r.register("玩家设置", "玩家设置", settings::cmd_view_settings);
    r.register("设置偏好", "设置", settings::cmd_set_preference);
    r.register("重置设置", "重置设置", settings::cmd_reset_settings);

    // 玩家举报系统
    r.register("举报玩家", "举报玩家", report::cmd_report_player);
    r.register("举报记录", "举报记录", report::cmd_my_reports);
    r.register("举报处理", "举报处理", report::cmd_handle_report);
    r.register("举报统计", "举报统计", report::cmd_report_stats);

    // 玩家交易系统
    r.register("发起交易", "发起交易", trade::cmd_propose_trade);
    r.register("接受交易", "接受交易", trade::cmd_accept_trade);
    r.register("拒绝交易", "拒绝交易", trade::cmd_reject_trade);
    r.register("添加交易", "添加交易", trade::cmd_add_trade_item);
    r.register("添加金币交易", "添加金币交易", trade::cmd_add_trade_gold);
    r.register("添加钻石交易", "添加钻石交易", trade::cmd_add_trade_diamond);
    r.register("查看交易", "查看交易", trade::cmd_view_trade);
    r.register("确认交易", "确认交易", trade::cmd_confirm_trade);
    r.register("取消交易", "取消交易", trade::cmd_cancel_trade);
    r.register("交易记录", "交易记录", trade::cmd_trade_history);

    // 玩家日报系统指令
    r.register("玩家日报", "玩家日报", daily_report::cmd_daily_report);
    r.register("周报总结", "周报总结", daily_report::cmd_weekly_report);
    r.register("成长里程碑", "成长里程碑", daily_report::cmd_growth_milestone);

    // 游戏管理面板系统
    r.register("服务器状态", "服务器状态", admin_panel::cmd_server_status);
    r.register("在线玩家", "在线玩家", admin_panel::cmd_online_players);
    r.register("封禁玩家", "封禁玩家", admin_panel::cmd_ban_player);
    r.register("解封玩家", "解封玩家", admin_panel::cmd_unban_player);
    r.register("系统统计", "系统统计", admin_panel::cmd_system_stats);
    r.register("玩家搜索", "玩家搜索", admin_panel::cmd_search_player);

    // 数据完整性检查系统
    r.register("数据检查", "数据检查", data_checker::cmd_data_check);
    r.register("表详情", "表详情", data_checker::cmd_table_info);
    r.register("修复孤立记录", "修复孤立记录", data_checker::cmd_fix_orphans);
    r.register("查看用户数据", "查看用户数据", data_checker::cmd_user_data);

    // 天气/环境系统
    r.register("查看天气", "查看天气", weather::cmd_view_weather);
    r.register("天气预报", "天气预报", weather::cmd_weather_forecast);
    r.register("天气效果", "天气效果", weather::cmd_weather_effects);

    // 声望系统指令
    r.register("查看声望", "查看声望", reputation::cmd_view_reputation);
    r.register("声望详情", "声望详情", reputation::cmd_reputation_detail);
    r.register("领取声望奖励", "领取声望奖励", reputation::cmd_claim_reputation_reward);
    r.register("声望排行", "声望排行", reputation::cmd_reputation_ranking);
    r.register("捐献声望", "捐献声望", reputation::cmd_donate_reputation);
    r.register("声望加成", "声望加成", reputation::cmd_reputation_bonus);

    // 每日运势系统指令
    r.register("查看运势", "查看运势", daily_fortune::cmd_view_fortune);
    r.register("运势加成", "运势加成", daily_fortune::cmd_fortune_bonus);
    r.register("运势历史", "运势历史", daily_fortune::cmd_fortune_history);

    // 怪物猎杀日志系统
    r.register("猎杀日志", "猎杀日志", monster_kill_log::cmd_kill_log);
    r.register("猎杀详情", "猎杀详情", monster_kill_log::cmd_kill_detail);
    r.register("猎杀排行", "猎杀排行", monster_kill_log::cmd_kill_ranking);
    r.register("猎杀里程碑", "猎杀里程碑", monster_kill_log::cmd_claim_milestone);
    r.register("掉落分析", "掉落分析", monster_kill_log::cmd_drop_analysis);

    // 怪物猎人等级系统指令
    r.register("猎人等级", "猎人等级", monster_hunter::cmd_hunter_rank);
    r.register("猎人详情", "猎人详情", monster_hunter::cmd_hunter_detail);
    r.register("猎人排行", "猎人排行", monster_hunter::cmd_hunter_ranking);
    r.register("猎人任务", "猎人任务", monster_hunter::cmd_hunter_quests);
    r.register("领取猎人奖励", "领取猎人奖励", monster_hunter::cmd_claim_hunt_reward);
    r.register("猎人商店", "猎人商店", monster_hunter::cmd_hunter_shop);
    r.register("猎人兑换", "猎人兑换", monster_hunter::cmd_hunter_exchange);

    // 财富评估系统指令
    r.register("玩家资产", "玩家资产", wealth::cmd_player_assets);
    r.register("财富排行", "财富排行", wealth::cmd_wealth_ranking);
    r.register("资产分析", "资产分析", wealth::cmd_asset_analysis);

    // 挖宝探险系统指令
    r.register("查看藏宝图", "查看藏宝图", treasure_hunt::cmd_view_treasure_maps);
    r.register("开始挖宝", "开始挖宝", treasure_hunt::cmd_treasure_dig);
    r.register("挖宝统计", "挖宝统计", treasure_hunt::cmd_treasure_stats);
    r.register("挖宝排行", "挖宝排行", treasure_hunt::cmd_treasure_ranking);

    // 全服公告系统指令
    r.register("查看公告", "查看公告", server_notice::cmd_view_notices);
    r.register("发布公告", "发布公告", server_notice::cmd_post_notice);
    r.register("删除公告", "删除公告", server_notice::cmd_delete_notice);
    r.register("公告置顶", "公告置顶", server_notice::cmd_pin_notice);
    r.register("公告统计", "公告统计", server_notice::cmd_notice_stats);
    r.register("全服通告", "全服通告", server_notice::cmd_server_announce);

    // 新手引导系统指令
    r.register("新手引导", "新手引导", tutorial::cmd_view_tutorial);
    r.register("当前引导", "当前引导", tutorial::cmd_tutorial_next);
    r.register("引导奖励", "引导奖励", tutorial::cmd_claim_tutorial_reward);

    // 公会红包系统
    r.register("发红包", "发红包", red_packet::cmd_send_red_packet);
    r.register("抢红包", "抢红包", red_packet::cmd_grab_red_packet);
    r.register("查看红包", "查看红包", red_packet::cmd_view_red_packets);
    r.register("红包统计", "红包统计", red_packet::cmd_red_packet_stats);

    // 系统变量管理系统
    r.register("查看变量", "查看变量", custom_variable::cmd_view_variables);
    r.register("变量详情", "变量详情", custom_variable::cmd_variable_detail);
    r.register("设置变量", "设置变量", custom_variable::cmd_set_variable);

    // 天赋树系统
    r.register("查看天赋", "查看天赋", talent_tree::cmd_view_talent);
    r.register("天赋详情", "天赋详情", talent_tree::cmd_talent_detail);
    r.register("分配天赋", "分配天赋", talent_tree::cmd_allocate_talent);
    r.register("重置天赋", "重置天赋", talent_tree::cmd_reset_talent);
    r.register("天赋排行", "天赋排行", talent_tree::cmd_talent_ranking);
    // 体力系统
    r.register("查看体力", "查看体力", stamina::cmd_view_stamina);
    r.register("恢复体力", "恢复体力", stamina::cmd_restore_stamina);
    r.register("体力预估", "体力预估", stamina::cmd_stamina_cost);

    // 钓鱼系统
    r.register("查看钓鱼", "查看钓鱼", fishing::cmd_view_fishing);
    r.register("开始钓鱼", "开始钓鱼", fishing::cmd_start_fishing);
    r.register("钓鱼背包", "钓鱼背包", fishing::cmd_fishing_bag);
    r.register("鱼类图鉴", "鱼类图鉴", fishing::cmd_fish_codex);
    r.register("钓鱼排行", "钓鱼排行", fishing::cmd_fishing_ranking);
    r.register("购买鱼饵", "购买鱼饵", fishing::cmd_buy_bait);
    r.register("钓鱼商店", "钓鱼商店", fishing::cmd_fishing_shop);
    r.register("出售鱼类", "出售鱼类", fishing::cmd_sell_fish);

    // 全服经济面板系统
    r.register("经济面板", "经济面板", economy_panel::cmd_economy_panel);
    r.register("经济排行", "经济排行", economy_panel::cmd_economy_ranking);
    r.register("经济统计", "经济统计", economy_panel::cmd_economy_stats);

    // 公会联盟系统
    r.register("创建联盟", "创建联盟", guild_alliance::cmd_create_alliance);
    r.register("邀请联盟", "邀请联盟", guild_alliance::cmd_invite_alliance);
    r.register("接受联盟", "接受联盟", guild_alliance::cmd_accept_alliance);
    r.register("拒绝联盟", "拒绝联盟", guild_alliance::cmd_reject_alliance);
    r.register("退出联盟", "退出联盟", guild_alliance::cmd_leave_alliance);
    r.register("解散联盟", "解散联盟", guild_alliance::cmd_disband_alliance);
    r.register("联盟信息", "联盟信息", guild_alliance::cmd_alliance_info);
    r.register("联盟列表", "联盟列表", guild_alliance::cmd_alliance_list);
    r.register("联盟捐献", "联盟捐献", guild_alliance::cmd_donate_alliance);
    r.register("联盟排行", "联盟排行", guild_alliance::cmd_alliance_ranking);
    r.register("联盟日志", "联盟日志", guild_alliance::cmd_alliance_log);
    r.register("联盟公告", "联盟公告", guild_alliance::cmd_alliance_notice);

    // 奇遇历险指令
    r.register("查看奇遇", "查看奇遇", adventure::cmd_view_adventure);
    r.register("选择奇遇", "选择奇遇", adventure::cmd_choose_adventure);
    r.register("奇遇日志", "奇遇日志", adventure::cmd_adventure_log);
    r.register("奇遇排行", "奇遇排行", adventure::cmd_adventure_ranking);
    r.register("奇遇信息", "奇遇信息", adventure::cmd_adventure_info);

    // 公会秘境系统指令
    r.register("公会秘境", "公会秘境", guild_realm::cmd_guild_realm);
    r.register("秘境探索", "秘境探索", guild_realm::cmd_realm_explore);
    r.register("秘境排行", "秘境排行", guild_realm::cmd_realm_ranking);
    r.register("秘境奖励", "秘境奖励", guild_realm::cmd_realm_rewards);

    // 全服许愿池系统指令
    r.register("许愿池", "许愿池", wish_pool::cmd_view_wish_pool);
    r.register("许愿池捐献", "许愿池捐献", wish_pool::cmd_contribute_wish);
    r.register("许愿池排行", "许愿池排行", wish_pool::cmd_wish_pool_ranking);
    r.register("许愿池奖励", "许愿池奖励", wish_pool::cmd_wish_pool_rewards);

    // 公会盛宴系统 (guild_banquet.rs)
    r.register("查看盛宴", "查看盛宴", guild_banquet::cmd_view_feast);
    r.register("发起盛宴", "发起盛宴", guild_banquet::cmd_host_feast);
    r.register("参加盛宴", "参加盛宴", guild_banquet::cmd_join_feast);
    r.register("宴会排行", "宴会排行", guild_banquet::cmd_feast_ranking);
    r.register("宴会信息", "宴会信息", guild_banquet::cmd_feast_info);

    // 公会建筑系统指令
    r.register("公会建筑", "公会建筑", guild_building::cmd_view_buildings);
    r.register("建筑详情", "建筑详情", guild_building::cmd_building_detail);
    r.register("升级建筑", "升级建筑", guild_building::cmd_upgrade_building);
    r.register("建筑排行", "建筑排行", guild_building::cmd_building_ranking);
    r.register("捐献资金", "捐献资金", guild_building::cmd_donate_building_fund);
    r.register("建筑加成", "建筑加成", guild_building::cmd_building_effects);

    // 婚礼伴侣系统
    r.register("求婚", "求婚", marriage::cmd_propose);
    r.register("回应求婚", "回应求婚", marriage::cmd_respond_proposal);
    r.register("查看伴侣", "查看伴侣", marriage::cmd_view_marriage);
    r.register("我的伴侣", "我的伴侣", marriage::cmd_view_marriage);
    r.register("离婚", "离婚", marriage::cmd_divorce);
    r.register("伴侣加成", "伴侣加成", marriage::cmd_marriage_bonus);
    r.register("伴侣排行", "伴侣排行", marriage::cmd_marriage_ranking);

    // 竞技荣誉商店系统指令
    r.register("荣誉商店", "荣誉商店", arena_shop::cmd_honor_shop);
    r.register("荣誉兑换", "荣誉兑换", arena_shop::cmd_honor_exchange);
    r.register("荣誉余额", "荣誉余额", arena_shop::cmd_honor_balance);
    r.register("荣誉记录", "荣誉记录", arena_shop::cmd_honor_history_cmd);

    // 时装收集系统指令
    r.register("查看时装", "查看时装", costume::cmd_view_costumes);
    r.register("购买时装", "购买时装", costume::cmd_buy_costume);
    r.register("穿戴时装", "穿戴时装", costume::cmd_equip_costume);
    r.register("卸下时装", "卸下时装", costume::cmd_unequip_costume);
    r.register("时装图鉴", "时装图鉴", costume::cmd_costume_codex);
    r.register("时装排行", "时装排行", costume::cmd_costume_ranking);

    // 成长基金系统指令
    r.register("查看基金", "查看基金", growth_fund::cmd_view_fund);
    r.register("购买基金", "购买基金", growth_fund::cmd_buy_fund);
    r.register("领取基金", "领取基金", growth_fund::cmd_claim_fund);
    r.register("基金进度", "基金进度", growth_fund::cmd_fund_progress);
    r.register("基金排行", "基金排行", growth_fund::cmd_fund_ranking);

    // 世界聊天频道系统指令
    r.register("聊天", "发送消息", world_chat::cmd_chat_send);
    r.register("发送消息", "发送消息", world_chat::cmd_chat_send);
    r.register("查看聊天", "查看聊天", world_chat::cmd_chat_view);
    r.register("切换频道", "切换频道", world_chat::cmd_chat_switch);
    r.register("聊天历史", "聊天历史", world_chat::cmd_chat_history);
    r.register("聊天帮助", "聊天帮助", world_chat::cmd_chat_help);

    // 签到补签系统指令
    r.register("补签列表", "补签列表", sign_makeup::cmd_makeup_list);
    r.register("补签", "补签", sign_makeup::cmd_makeup_sign);

    // 全服悬赏通缉系统指令
    r.register("查看通缉", "查看通缉", wanted::cmd_view_wanted);
    r.register("发布通缉", "发布通缉", wanted::cmd_post_bounty);
    r.register("接受通缉", "接受通缉", wanted::cmd_accept_bounty);
    r.register("领取赏金", "领取赏金", wanted::cmd_claim_bounty);
    r.register("通缉详情", "通缉详情", wanted::cmd_wanted_info);
    r.register("通缉排行", "通缉排行", wanted::cmd_wanted_ranking);
    r.register("我的通缉", "我的通缉", wanted::cmd_my_wanted);

    // 游戏日历系统指令
    r.register("查看日历", "查看日历", game_calendar::cmd_view_calendar);
    r.register("今日任务", "今日任务", game_calendar::cmd_today_tasks);
    r.register("活动预告", "活动预告", game_calendar::cmd_event_preview);
    r.register("重置时间", "重置时间", game_calendar::cmd_reset_timers);
    r.register("赛季日历", "赛季日历", game_calendar::cmd_season_calendar);

    // 资源回收系统指令
    r.register("查看回收", "查看回收", recycle::cmd_view_recycle);
    r.register("回收物品", "回收物品", recycle::cmd_recycle_item);
    r.register("批量回收", "批量回收", recycle::cmd_recycle_batch);
    r.register("回收商店", "回收商店", recycle::cmd_recycle_shop);
    r.register("回收兑换", "回收兑换", recycle::cmd_recycle_exchange);
    r.register("回收排行", "回收排行", recycle::cmd_recycle_ranking);
    r.register("回收统计", "回收统计", recycle::cmd_recycle_stats);

    // 全服里程碑系统
    r.register("里程碑", "里程碑", server_milestone::cmd_view_milestones);
    r.register("里程碑进度", "里程碑进度", server_milestone::cmd_milestone_progress);
    r.register("领取里程碑", "领取里程碑", server_milestone::cmd_claim_milestone);
    r.register("里程碑排行", "里程碑排行", server_milestone::cmd_milestone_ranking);

    // 玩家回归奖励系统
    r.register("查看回归奖励", "查看回归奖励", return_reward::cmd_view_return_reward);
    r.register("领取回归奖励", "领取回归奖励", return_reward::cmd_claim_return_reward);
    r.register("回归排行", "回归排行", return_reward::cmd_return_ranking);
    r.register("回归统计", "回归统计", return_reward::cmd_return_stats);

    // 军衔战阶系统
    r.register("查看军衔", "查看军衔", military_rank::cmd_view_military_rank);
    r.register("军衔排行", "军衔排行", military_rank::cmd_military_ranking);
    r.register("军衔详情", "军衔详情", military_rank::cmd_military_detail);
    r.register("军衔奖励", "军衔奖励", military_rank::cmd_military_reward);
    r.register("军衔日志", "军衔日志", military_rank::cmd_military_log);

    // 决斗场系统
    r.register("决斗挑战", "决斗挑战", duel::cmd_duel_challenge);
    r.register("接受决斗", "接受决斗", duel::cmd_duel_accept);
    r.register("拒绝决斗", "拒绝决斗", duel::cmd_duel_decline);
    r.register("决斗历史", "决斗历史", duel::cmd_duel_history);
    r.register("决斗排行", "决斗排行", duel::cmd_duel_ranking);
    r.register("查看决斗", "查看决斗", duel::cmd_view_duels);
    r.register("决斗帮助", "决斗帮助", duel::cmd_duel_help);

    // === 世界等级系统 ===
    r.register("世界等级", "世界等级", world_level::cmd_world_level);
    r.register("世界等级详情", "世界等级详情", world_level::cmd_world_level_detail);
    r.register("世界等级排行", "世界等级排行", world_level::cmd_world_level_ranking);
    r.register("世界等级奖励", "世界等级奖励", world_level::cmd_world_level_reward);
    r.register("领取世界奖励", "领取世界奖励", world_level::cmd_claim_world_reward);
    r.register("世界等级历史", "世界等级历史", world_level::cmd_world_level_history);

    let _ = DB.set(Mutex::new(database));
    let _ = ROUTER.set(Mutex::new(r));

    0
}

/// 处理消息
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_process(
    msg: *const std::os::raw::c_char,
    msg_type: *const std::os::raw::c_char,
    group: *const std::os::raw::c_char,
    user_id: *const std::os::raw::c_char,
) -> *mut std::os::raw::c_char {
    let msg = unsafe { cstr_to_string(msg) };
    let msg_type = unsafe { cstr_to_string(msg_type) };
    let group = unsafe { cstr_to_string(group) };
    let user_id = unsafe { cstr_to_string(user_id) };

    let db = DB.get().expect("引擎未初始化");
    let router = ROUTER.get().expect("引擎未初始化");

    let db = db.lock().unwrap();
    let router = router.lock().unwrap();

    let result = router.handle(&db, &msg, &msg_type, &group, &user_id);

    if result.is_empty() {
        std::ptr::null_mut()
    } else {
        string_to_cchar(result)
    }
}

/// 释放字符串内存
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_free_string(s: *mut std::os::raw::c_char) {
    if !s.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(s);
        }
    }
}

/// 获取用户信息 JSON
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_get_user_info(user_id: *const std::os::raw::c_char) -> *mut std::os::raw::c_char {
    let user_id = unsafe { cstr_to_string(user_id) };
    let db = DB.get().expect("引擎未初始化");
    let db = db.lock().unwrap();

    let info = user::calc_total_attrs(&db, &user_id);
    let json = serde_json::to_string(&info).unwrap_or_default();
    string_to_cchar(json)
}

/// 获取背包 JSON
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_get_knapsack(user_id: *const std::os::raw::c_char) -> *mut std::os::raw::c_char {
    let user_id = unsafe { cstr_to_string(user_id) };
    let db = DB.get().expect("引擎未初始化");
    let db = db.lock().unwrap();

    let items = db.knapsack_all(&user_id);
    let json = serde_json::to_string(&items).unwrap_or_default();
    string_to_cchar(json)
}

/// 获取装备 JSON
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cg_get_equips(user_id: *const std::os::raw::c_char) -> *mut std::os::raw::c_char {
    let user_id = unsafe { cstr_to_string(user_id) };
    let db = DB.get().expect("引擎未初始化");
    let db = db.lock().unwrap();

    let equips = db.equip_all(&user_id);
    let json = serde_json::to_string(&equips).unwrap_or_default();
    string_to_cchar(json)
}

/// 关闭引擎
#[no_mangle]
pub extern "C" fn cg_shutdown() {}

// ==================== 辅助函数 ====================

unsafe fn cstr_to_string(s: *const std::os::raw::c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(s).to_str().unwrap_or("").to_string()
}

fn string_to_cchar(s: String) -> *mut std::os::raw::c_char {
    match std::ffi::CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ==================== NPC好感度系统包装函数 ====================

fn npc_affinity_cmd_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_view_npc_affinity(db, user_id)
}

fn npc_affinity_cmd_talk(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_npc_talk(db, user_id, args)
}

fn npc_affinity_cmd_gift(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_npc_gift(db, user_id, args)
}

fn npc_affinity_cmd_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_npc_affinity_detail(db, user_id, args)
}

fn npc_affinity_cmd_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_claim_npc_affinity_reward(db, user_id, args)
}

fn npc_affinity_cmd_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    npc_affinity::cmd_npc_affinity_ranking(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_flow() {
        let db_path = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb";
        let path_cstr = std::ffi::CString::new(db_path).unwrap();

        let result = cg_init(path_cstr.as_ptr());
        assert_eq!(result, 0);

        // 测试帮助
        let msg = std::ffi::CString::new("帮助").unwrap();
        let msg_type = std::ffi::CString::new("2").unwrap();
        let group = std::ffi::CString::new("123456").unwrap();
        let user_id = std::ffi::CString::new("test_user").unwrap();

        let result_ptr = cg_process(msg.as_ptr(), msg_type.as_ptr(), group.as_ptr(), user_id.as_ptr());
        if !result_ptr.is_null() {
            let result = unsafe { std::ffi::CStr::from_ptr(result_ptr) }.to_str().unwrap();
            println!("帮助结果: {}", result);
            cg_free_string(result_ptr);
        }

        cg_shutdown();
    }

    #[test]
    fn test_db_help() {
        let db_path = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb";
        let path_cstr = std::ffi::CString::new(db_path).unwrap();

        let result = cg_init(path_cstr.as_ptr());
        assert_eq!(result, 0);

        let msg_type = std::ffi::CString::new("2").unwrap();
        let group = std::ffi::CString::new("123456").unwrap();
        let user_id = std::ffi::CString::new("test_user").unwrap();

        // 测试查看帮助（列出所有话题）
        let msg = std::ffi::CString::new("查看帮助").unwrap();
        let result_ptr = cg_process(msg.as_ptr(), msg_type.as_ptr(), group.as_ptr(), user_id.as_ptr());
        if !result_ptr.is_null() {
            let result = unsafe { std::ffi::CStr::from_ptr(result_ptr) }.to_str().unwrap();
            println!("查看帮助（列表）: {}", result);
            assert!(result.contains("帮助主题"));
            assert!(result.contains("注册游戏") || result.contains("帮助"));
            cg_free_string(result_ptr);
        }

        // 测试查看帮助+具体话题
        let msg = std::ffi::CString::new("查看帮助+注册游戏").unwrap();
        let result_ptr = cg_process(msg.as_ptr(), msg_type.as_ptr(), group.as_ptr(), user_id.as_ptr());
        if !result_ptr.is_null() {
            let result = unsafe { std::ffi::CStr::from_ptr(result_ptr) }.to_str().unwrap();
            println!("查看帮助（注册游戏）: {}", result);
            assert!(result.contains("注册"));
            cg_free_string(result_ptr);
        }

        // 测试查看帮助+不存在的话题
        let msg = std::ffi::CString::new("查看帮助+不存在的话题").unwrap();
        let result_ptr = cg_process(msg.as_ptr(), msg_type.as_ptr(), group.as_ptr(), user_id.as_ptr());
        if !result_ptr.is_null() {
            let result = unsafe { std::ffi::CStr::from_ptr(result_ptr) }.to_str().unwrap();
            println!("查看帮助（模糊匹配）: {}", result);
            cg_free_string(result_ptr);
        }

        // 测试查看帮助+模糊匹配
        let msg = std::ffi::CString::new("查看帮助+攻击").unwrap();
        let result_ptr = cg_process(msg.as_ptr(), msg_type.as_ptr(), group.as_ptr(), user_id.as_ptr());
        if !result_ptr.is_null() {
            let result = unsafe { std::ffi::CStr::from_ptr(result_ptr) }.to_str().unwrap();
            println!("查看帮助（攻击）: {}", result);
            assert!(result.contains("攻击"));
            cg_free_string(result_ptr);
        }

        cg_shutdown();
    }
}
mod auction;
