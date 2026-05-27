/// CakeGame 编辑器测试程序
/// 演示如何使用编辑器创建和管理游戏数据

use cakegame::editor::GameEditor;

fn main() {
    println!("=== CakeGame 编辑器测试 ===\n");

    // 创建编辑器实例
    let mut editor = GameEditor::new();

    // 创建新项目
    editor.new_project("我的第一个游戏", "开发者");
    println!("✓ 创建新项目");

    // 添加场景
    editor.add_scene("s1", "第一章", "故事开始的地方").unwrap();
    editor.add_scene("s2", "第二章", "冒险继续").unwrap();
    println!("✓ 添加场景");

    // 设置起始场景
    editor.set_start_scene("s1");

    // 添加对话
    editor.add_dialog("d1", "", "你醒来发现自己在一个陌生的地方。", "d2").unwrap();
    editor.add_dialog("d2", "你", "这是哪里？我什么都不记得了...", "d3").unwrap();
    editor.add_dialog("d3", "", "你看到前方有两条路。", "").unwrap();
    println!("✓ 添加对话");

    // 添加选项
    editor.add_choice("d3", "走左边的小路", "d4").unwrap();
    editor.add_choice("d3", "走右边的大路", "d5").unwrap();
    println!("✓ 添加选项");

    // 添加更多对话
    editor.add_dialog("d4", "", "你沿着小路走去，发现了一个宝箱！", "").unwrap();
    editor.add_dialog("d5", "", "你沿着大路走去，遇到了一个旅人。", "d6").unwrap();
    editor.add_dialog("d6", "旅人", "你好，旅行者！这条路通向城镇。", "").unwrap();

    // 添加物品
    editor.add_item("sword", "铁剑", "一把普通的铁剑", 1, true).unwrap();
    editor.add_item("potion", "治疗药水", "恢复30点生命值", 10, true).unwrap();
    editor.add_item("gold", "金币", "通用货币", 9999, false).unwrap();
    println!("✓ 添加物品");

    // 添加角色
    editor.add_character("traveler", "旅人", "友善的旅行者", "一个看起来很友善的旅人").unwrap();
    editor.add_character("merchant", "商人", "旅行商人", "经营各种商品的商人").unwrap();
    println!("✓ 添加角色");

    // 添加变量
    editor.add_variable("health", "生命值", "100", "玩家的生命值").unwrap();
    editor.add_variable("attack", "攻击力", "10", "玩家的攻击力").unwrap();
    println!("✓ 添加变量");

    // 添加标记
    editor.add_flag("met_traveler", "遇到旅人", "是否遇到了旅人").unwrap();
    editor.add_flag("got_sword", "获得铁剑", "是否获得了铁剑").unwrap();
    println!("✓ 添加标记");

    // 验证项目
    let errors = editor.validate();
    if errors.is_empty() {
        println!("✓ 项目验证通过!");
    } else {
        println!("✗ 发现 {} 个错误:", errors.len());
        for error in &errors {
            println!("  - {}", error);
        }
    }

    // 获取统计信息
    let stats = editor.get_stats();
    println!("\n项目统计:");
    for (key, value) in &stats {
        println!("  {}: {}", key, value);
    }

    // 保存项目
    match editor.save() {
        Ok(json) => {
            println!("\n生成的JSON数据 (前500字符):");
            let preview = if json.len() > 500 { &json[..500] } else { &json };
            println!("{}...", preview);

            // 保存到文件
            std::fs::write("/opt/data/cakegame-rs/examples/editor_output.json", &json)
                .expect("保存文件失败");
            println!("\n✓ 已保存到 editor_output.json");
        }
        Err(e) => eprintln!("保存失败: {}", e),
    }

    println!("\n编辑器测试完成!");
}
