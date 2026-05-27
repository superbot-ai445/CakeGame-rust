/// CakeGame DLL 调用示例 (C# P/Invoke)
///
/// 在 C# 中这样调用：
///
/// [DllImport("cakegame.dll")]
/// static extern int Cake_Initialize();
///
/// [DllImport("cakegame.dll")]
/// static extern int Cake_LoadGame(string path);
///
/// [DllImport("cakegame.dll")]
/// static extern int Cake_StartGame();
///
/// [DllImport("cakegame.dll")]
/// static extern int Cake_Choose(int index);
///
/// [DllImport("cakegame.dll")]
/// static extern int Cake_Continue();
///
/// [DllImport("cakegame.dll", CharSet = CharSet.Ansi)]
/// static extern int Cake_PollMessage(ref int msgType, out IntPtr content, out IntPtr speaker);
///
/// [DllImport("cakegame.dll")]
/// static extern void Cake_FreeString(IntPtr s);
///
/// // 使用示例：
/// // Cake_Initialize();
/// // Cake_LoadGame("game.json");
/// // Cake_StartGame();
/// //
/// // while (true) {
/// //     int msgType = 0;
/// //     IntPtr contentPtr, speakerPtr;
/// //     int hasMsg = Cake_PollMessage(ref msgType, out contentPtr, out speakerPtr);
/// //     
/// //     if (hasMsg > 0) {
/// //         string content = Marshal.PtrToStringAnsi(contentPtr);
/// //         string speaker = speakerPtr != IntPtr.Zero ? Marshal.PtrToStringAnsi(speakerPtr) : null;
/// //         
/// //         switch (msgType) {
/// //             case 1: // Dialog
/// //                 Console.WriteLine($"{speaker}: {content}");
/// //                 break;
/// //             case 2: // Choice
/// //                 int count = Cake_GetChoiceCount();
/// //                 for (int i = 0; i < count; i++) {
/// //                     IntPtr textPtr = Cake_GetChoiceText(i);
/// //                     Console.WriteLine($"  [{i}] {Marshal.PtrToStringAnsi(textPtr)}");
/// //                 }
/// //                 int choice = int.Parse(Console.ReadLine());
/// //                 Cake_Choose(choice);
/// //                 break;
/// //             case 3: // Narration
/// //                 Console.WriteLine(content);
/// //                 break;
/// //             case 7: // GameOver
/// //                 Console.WriteLine($"结局: {content}");
/// //                 return;
/// //         }
/// //         
/// //         Cake_FreeString(contentPtr);
/// //         if (speakerPtr != IntPtr.Zero) Cake_FreeString(speakerPtr);
/// //     }
/// // }

// 以下是编辑器的 C# 调用示例：

/// [DllImport("cakegame.dll")]
/// static extern int Editor_Initialize();
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_Load(string path);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_Save(string path);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_AddScene(string id, string name, string desc);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_AddDialog(string sceneId, string dialogId, string speaker, string text, string next);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_AddChoice(string dialogId, string text, string target);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_AddItem(string id, string name, string desc, int maxCount, int usable);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_AddCharacter(string id, string name, string title, string desc);
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_GetSceneCount();
///
/// [DllImport("cakegame.dll")]
/// static extern int Editor_Validate();
///
/// // 使用示例：
/// // Editor_Initialize();
/// //
/// // // 创建场景
/// // Editor_AddScene("chapter1", "第一章", "故事的开始...");
/// //
/// // // 添加对话
/// // Editor_AddDialog("chapter1", "ch1_d1", "旁白", "你醒来发现自己在一个陌生的地方...", "ch1_d2");
/// // Editor_AddDialog("chapter1", "ch1_d2", "你", "这是哪里？", "ch1_d3");
/// // Editor_AddDialog("chapter1", "ch1_d3", "??? ", "你终于醒了。", "");
/// //
/// // // 添加选项
/// // Editor_AddChoice("ch1_d3", "你是谁？", "ch1_d4a");
/// // Editor_AddChoice("ch1_d3", "我在哪？", "ch1_d4b");
/// //
/// // // 保存
/// // Editor_Save("game.json");
/// // int errors = Editor_Validate();

fn main() {
    println!("=== CakeGame Rust 重写示例 ===\n");
    
    // 这是一个文字对话游戏引擎
    // 通过 DLL 导出函数供 C# 等语言调用
    
    println!("功能列表：");
    println!("  - 场景管理 (Scene)");
    println!("  - 对话系统 (Dialog)");
    println!("  - 分支选择 (Choice)");
    println!("  - 变量系统 (Variable)");
    println!("  - 标记系统 (Flag)");
    println!("  - 物品系统 (Item)");
    println!("  - 角色系统 (Character)");
    println!("  - 好感度 (Affinity)");
    println!("  - 存档系统 (Save/Load)");
    println!("  - 数据编辑器 (Editor)");
    println!("  - 用户管理 (User)");
    
    println!("\n使用方式：");
    println!("  1. 用编辑器创建游戏数据 (game.json)");
    println!("  2. 用 Cake_LoadGame() 加载数据");
    println!("  3. 用 Cake_StartGame() 开始游戏");
    println!("  4. 用 Cake_PollMessage() 获取消息");
    println!("  5. 用 Cake_Choose() / Cake_Continue() 推进剧情");
    
    println!("\n编译：");
    println!("  cargo build --release");
    println!("  生成：target/release/cakegame.dll (Windows)");
    println!("        target/release/libcakegame.so (Linux)");
    println!("        target/release/libcakegame.dylib (macOS)");
}
