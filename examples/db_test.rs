/// 详细测试数据库中每一个值
use cakegame::db_loader;
use std::collections::HashMap;

fn main() {
    let db_path = "/opt/data/hermes_cakegame/CakeGame调试器/data/CakeGameData/gamedata.sdb";
    
    println!("=== 详细数据库测试 ===\n");
    println!("数据库路径: {}", db_path);
    
    let engine = match db_loader::load_database(db_path) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("加载失败: {}", e);
            return;
        }
    };

    // ==================== 物品测试 ====================
    println!("\n==================== 物品系统测试 ====================");
    println!("物品总数: {}", engine.items.len());
    
    let mut equip_count = 0;
    let mut potion_count = 0;
    let mut material_count = 0;
    let mut other_count = 0;
    
    for (id, item) in &engine.items {
        match item.item_type {
            cakegame::core::ItemType::Equip => equip_count += 1,
            cakegame::core::ItemType::Potion => potion_count += 1,
            cakegame::core::ItemType::Material => material_count += 1,
            _ => other_count += 1,
        }
    }
    
    println!("  装备: {}", equip_count);
    println!("  药水: {}", potion_count);
    println!("  材料: {}", material_count);
    println!("  其他: {}", other_count);
    
    // 测试前10个物品
    println!("\n前10个物品详情:");
    for (i, (id, item)) in engine.items.iter().take(10).enumerate() {
        println!("  {}. {} [{}] HP:{} MP:{} AD:{} AP:{} DEF:{} MAG:{}",
            i + 1, item.name, id,
            item.data.add_hp, item.data.add_mp,
            item.data.add_ad, item.data.add_ap,
            item.data.add_defense, item.data.add_magic
        );
    }

    // ==================== 怪物测试 ====================
    println!("\n==================== 怪物系统测试 ====================");
    println!("怪物总数: {}", engine.monsters.len());
    
    let mut ordinary_count = 0;
    let mut elite_count = 0;
    let mut boss_count = 0;
    
    for (_, monster) in &engine.monsters {
        match monster.monster_type {
            cakegame::core::MonsterType::Ordinary => ordinary_count += 1,
            cakegame::core::MonsterType::Elite => elite_count += 1,
            cakegame::core::MonsterType::Boss => boss_count += 1,
        }
    }
    
    println!("  普通: {}", ordinary_count);
    println!("  精英: {}", elite_count);
    println!("  BOSS: {}", boss_count);
    
    // 测试所有怪物
    println!("\n所有怪物详情:");
    for (i, (name, monster)) in engine.monsters.iter().enumerate() {
        println!("  {}. {} | HP:{} AD:{} DEF:{} | 经验:{} 金币:{}",
            i + 1, name, monster.hp, monster.ad, monster.defense,
            monster.reward_exp, monster.reward_gold
        );
        if !monster.reward_goods.is_empty() {
            println!("     掉落: {:?}", monster.reward_goods);
        }
    }

    // ==================== 地图测试 ====================
    println!("\n==================== 地图系统测试 ====================");
    println!("地图总数: {}", engine.maps.len());
    
    let mut security_count = 0;
    let mut hidden_count = 0;
    
    for (_, map) in &engine.maps {
        if map.security { security_count += 1; }
        if map.hid { hidden_count += 1; }
    }
    
    println!("  安全区: {}", security_count);
    println!("  隐藏地图: {}", hidden_count);
    
    // 测试所有地图
    println!("\n所有地图详情:");
    for (i, (name, map)) in engine.maps.iter().enumerate() {
        println!("  {}. {} | Lv:{} | 安全:{} | 隐藏:{}",
            i + 1, name, map.lv, map.security, map.hid
        );
        if !map.up.is_empty() { println!("     上: {}", map.up); }
        if !map.down.is_empty() { println!("     下: {}", map.down); }
        if !map.left.is_empty() { println!("     左: {}", map.left); }
        if !map.right.is_empty() { println!("     右: {}", map.right); }
    }

    // ==================== 任务测试 ====================
    println!("\n==================== 任务系统测试 ====================");
    println!("任务总数: {}", engine.tasks.len());
    
    let mut main_count = 0;
    let mut branch_count = 0;
    let mut daily_count = 0;
    
    for (_, task) in &engine.tasks {
        match task.task_type {
            cakegame::core::TaskType::Main => main_count += 1,
            cakegame::core::TaskType::Branch => branch_count += 1,
            cakegame::core::TaskType::Daily => daily_count += 1,
            _ => {}
        }
    }
    
    println!("  主线: {}", main_count);
    println!("  支线: {}", branch_count);
    println!("  日常: {}", daily_count);
    
    // 测试所有任务
    println!("\n所有任务详情:");
    for (i, (name, task)) in engine.tasks.iter().enumerate() {
        println!("  {}. {} | Lv:{} | 类型:{:?} | 前置:{}",
            i + 1, name, task.lv, task.task_type, task.complete_task
        );
        println!("     奖励: 金币:{} 钻石:{} 经验:{}",
            task.reward_gold, task.reward_diamonds, task.reward_exp
        );
        if !task.reward_goods.is_empty() {
            println!("     物品奖励: {:?}", task.reward_goods);
        }
    }

    // ==================== 合成配方测试 ====================
    println!("\n==================== 合成系统测试 ====================");
    println!("配方总数: {}", engine.composites.len());
    
    // 测试所有配方
    println!("\n所有配方详情:");
    for (i, (name, recipe)) in engine.composites.iter().enumerate() {
        println!("  {}. {} | 成功率:{}%",
            i + 1, name, recipe.success_rate
        );
        println!("     消耗金币:{}", recipe.consume_gold);
        println!("     消耗钻石:{}", recipe.consume_diamond);
        if !recipe.consume_goods.is_empty() {
            println!("     消耗物品: {:?}", recipe.consume_goods);
        }
    }

    // ==================== 商店测试 ====================
    println!("\n==================== 商店系统测试 ====================");
    for (shop_id, items) in &engine.shops {
        println!("商店 [{}]: {} 个商品", shop_id, items.len());
        for (i, item) in items.iter().take(5).enumerate() {
            println!("  {}. {} | 货币:{} | 价格:{} | 限量:{}",
                i + 1, item.name, item.currency, item.price, item.limit_number
            );
        }
    }

    // ==================== 套装测试 ====================
    println!("\n==================== 套装系统测试 ====================");
    println!("套装总数: {}", engine.suits.len());
    
    // 测试所有套装
    println!("\n所有套装详情:");
    for (i, (name, suit)) in engine.suits.iter().enumerate() {
        println!("  {}. {} | HP:{} MP:{} AD:{} AP:{} DEF:{} MAG:{}",
            i + 1, name, suit.add_hp, suit.add_mp,
            suit.add_ad, suit.add_ap, suit.add_defense, suit.add_magic
        );
    }

    // ==================== 十六进制解码测试 ====================
    println!("\n==================== 十六进制解码测试 ====================");
    
    let test_cases = vec![
        ("4850", "HP"),
        ("4D50", "MP"),
        ("4144", "AD"),
        ("313030", "100"),
        ("46414C5345", "FALSE"),
        ("54525545", "TRUE"),
    ];
    
    for (hex, expected) in test_cases {
        let decoded = db_loader::decode_hex_gbk(hex);
        let status = if decoded == expected { "✓" } else { "✗" };
        println!("  {} {} -> {} (期望: {})", status, hex, decoded, expected);
    }

    // ==================== 数据完整性检查 ====================
    println!("\n==================== 数据完整性检查 ====================");
    
    let mut issues = Vec::new();
    
    // 检查物品数据
    for (id, item) in &engine.items {
        if item.name.is_empty() {
            issues.push(format!("物品 {} 没有名称", id));
        }
    }
    
    // 检查怪物数据
    for (name, monster) in &engine.monsters {
        if monster.hp <= 0 {
            issues.push(format!("怪物 {} HP异常: {}", name, monster.hp));
        }
    }
    
    // 检查地图数据
    for (name, map) in &engine.maps {
        if map.lv <= 0 {
            issues.push(format!("地图 {} 等级异常: {}", name, map.lv));
        }
    }
    
    if issues.is_empty() {
        println!("✓ 数据完整性检查通过");
    } else {
        println!("✗ 发现 {} 个问题:", issues.len());
        for issue in &issues {
            println!("  - {}", issue);
        }
    }

    println!("\n=== 测试完成 ===");
}
