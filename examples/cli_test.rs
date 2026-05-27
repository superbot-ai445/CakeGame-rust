/// CakeGame 命令行测试程序
/// 演示 RPG 引擎功能

use std::io::{self, Write};

use cakegame::core::GameEngine;
use cakegame::core::MessageType;
use cakegame::db_loader;

fn main() {
    println!("=== CakeGame RPG 引擎测试 ===\n");

    let db_path = "/opt/data/CakeGame调试器/data/CakeGameData/gamedata.sdb";

    println!("加载数据库: {}", db_path);
    let mut engine = match db_loader::load_database(db_path) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("加载失败: {}", e);
            return;
        }
    };

    println!("✓ 数据库加载成功");
    println!("  物品: {}", engine.items.len());
    println!("  怪物: {}", engine.monsters.len());
    println!("  地图: {}", engine.maps.len());
    println!("  任务: {}", engine.tasks.len());
    println!("  配方: {}", engine.composites.len());
    println!("  商店: {}", engine.shops.len());
    println!("  套装: {}", engine.suits.len());

    // 创建角色
    println!("\n创建角色...");
    match engine.create_character("测试角色", "战士") {
        Ok(_) => println!("✓ 角色创建成功"),
        Err(e) => {
            // 如果职业不存在，使用默认
            println!("职业 '战士' 不存在，使用默认职业");
            engine.user.name = "测试角色".to_string();
            engine.user.occupation = "冒险者".to_string();
        }
    }

    println!("\n{}", engine.get_user_info());

    // 测试移动
    println!("\n=== 测试移动 ===");
    let map_names: Vec<String> = engine.maps.keys().take(3).cloned().collect();
    for map_name in &map_names {
        match engine.move_to(map_name) {
            Ok(_) => println!("✓ 移动到: {}", map_name),
            Err(e) => println!("✗ 移动失败: {} - {}", map_name, e),
        }
    }

    // 测试战斗
    println!("\n=== 测试战斗 ===");
    let monster_names: Vec<String> = engine.monsters.keys().take(2).cloned().collect();
    for monster_name in &monster_names {
        println!("\n尝试战斗: {}", monster_name);
        match engine.start_combat(monster_name) {
            Ok(_) => {
                // 处理消息
                while let Some(msg) = engine.poll_message() {
                    match msg.msg_type {
                        MessageType::Combat => println!("  ⚔️ {}", msg.content),
                        _ => {}
                    }
                }

                // 攻击几次
                for _i in 0..5 {
                    match engine.attack() {
                        Ok(_) => {
                            while let Some(msg) = engine.poll_message() {
                                match msg.msg_type {
                                    MessageType::Combat => println!("  {}", msg.content),
                                    MessageType::ExpChange => println!("  📈 {}", msg.content),
                                    MessageType::GoldChange => println!("  💰 {}", msg.content),
                                    MessageType::ItemGet => println!("  📦 {}", msg.content),
                                    MessageType::LevelUp => println!("  🎉 {}", msg.content),
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            println!("  战斗结束: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => println!("  战斗失败: {}", e),
        }
    }

    // 测试物品
    println!("\n=== 测试物品 ===");
    let items: Vec<String> = engine.items.keys().take(5).cloned().collect();
    for item_id in &items {
        match engine.add_item(item_id, 1) {
            Ok(_) => println!("✓ 添加物品: {}", item_id),
            Err(e) => println!("✗ 添加失败: {} - {}", item_id, e),
        }
    }

    println!("\n背包:");
    for (id, name, count) in engine.get_inventory_list() {
        println!("  {} x{}", name, count);
    }

    // 测试存档
    println!("\n=== 测试存档 ===");
    match engine.save_state() {
        Ok(json) => {
            println!("✓ 存档成功 ({} 字节)", json.len());
            // 保存到文件
            std::fs::write("/opt/data/cakegame-rs/test_save.json", &json).ok();
        }
        Err(e) => println!("✗ 存档失败: {}", e),
    }

    println!("\n最终状态:");
    println!("{}", engine.get_user_info());

    println!("\n测试完成!");
}
