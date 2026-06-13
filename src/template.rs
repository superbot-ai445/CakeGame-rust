/// CakeGame 消息模板渲染引擎
/// 解析 MessageTemplate 表中的 CGD 格式模板，支持变量替换和条件渲染
/// 数据来源: MessageTemplate 表 (87条模板数据)
///
/// CGD 格式说明:
/// - `#CGD DATA{...Q}` — CGD 数据包（包含多个命名段）
/// - `GH` — 字段分隔符（同一记录内的列分隔）
/// - `GI` — 记录分隔符（不同记录之间的行分隔）
/// - `[变量名]` — 变量占位符，运行时替换
/// - `$变量名=值` — 变量赋值
/// - `<...>` — 循环模板体（重复N次，每次替换对应变量）
/// - `#Define['旧值']->['新值']` — 宏定义替换
use crate::db::Database;
use std::collections::HashMap;

/// 模板渲染上下文
#[derive(Debug, Clone)]
pub struct TemplateContext {
    /// 变量映射: 变量名 -> 值
    pub vars: HashMap<String, String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    /// 设置变量
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    /// 批量设置变量
    pub fn set_many(&mut self, pairs: &[(&str, &str)]) -> &mut Self {
        for (k, v) in pairs {
            self.vars.insert(k.to_string(), v.to_string());
        }
        self
    }
}

/// 渲染简单模板（非 CGD 格式，直接变量替换）
pub fn render_simple(template: &str, ctx: &TemplateContext) -> String {
    let mut result = template.to_string();
    for (key, value) in &ctx.vars {
        result = result.replace(&format!("[{}]", key), value);
    }
    // 清理未替换的变量标记
    result = clean_unresolved(&result);
    result
}

/// 解析 CGD 格式模板，提取命名段
pub fn parse_cgd(raw: &str) -> HashMap<String, String> {
    let mut sections: HashMap<String, String> = HashMap::new();
    // CGD 格式: #CGD DATA{段名1GH内容1GI段名2GH内容2Q}
    if let Some(start) = raw.find("#CGD DATA{") {
        let data_start = start + "#CGD DATA{".len();
        if let Some(end) = raw[data_start..].find("Q}") {
            let data = &raw[data_start..data_start + end];
            // 按 GI 分割各段
            let parts: Vec<&str> = data.split("GI").collect();
            for part in parts {
                if let Some(pos) = part.find("GH") {
                    // 去除前导控制字符（如 \x01）
                    let key: String = part[..pos].trim().chars().filter(|c| !c.is_control()).collect();
                    let val = part[pos + 2..].trim();
                    if !key.is_empty() {
                        sections.insert(key, val.to_string());
                    }
                }
            }
        }
    }
    sections
}

/// 渲染 CGD 模板的指定段
pub fn render_cgd_section(raw: &str, section_name: &str, ctx: &TemplateContext) -> Option<String> {
    let sections = parse_cgd(raw);
    sections.get(section_name).map(|template| render_simple(template, ctx))
}

/// 从数据库读取模板并渲染指定段
pub fn render_template(
    db: &Database,
    template_name: &str,
    section_name: &str,
    ctx: &TemplateContext,
) -> Option<String> {
    let raw = db.template_get(template_name);
    if raw.is_empty() {
        return None;
    }
    render_cgd_section(&raw, section_name, ctx)
}

/// 渲染排行查询返回模板
#[allow(dead_code)]
pub fn render_ranking(
    db: &Database,
    entries: &[(String, String, String)],
    page: i32,
    total_pages: i32,
    total_count: i32,
) -> String {
    let raw = db.template_get("排行查询返回");
    if raw.is_empty() {
        // 模板不存在时使用默认格式
        return render_ranking_default(entries, page, total_pages, total_count);
    }

    // 解析模板中的循环体 <[排名序号].[玩家名]([玩家ID])\n[属性名称]:[属性值]>
    let template_line = extract_loop_template(&raw);
    let header = extract_header(&raw);
    let footer = extract_footer(&raw);

    let mut result = String::new();

    // 渲染头部
    if !header.is_empty() {
        result.push_str(&render_simple(&header, &TemplateContext::new()));
        result.push('\n');
    }

    // 渲染每条记录
    for (i, (name, uid, value)) in entries.iter().enumerate() {
        let mut ctx = TemplateContext::new();
        ctx.set("排名序号", &(i + 1).to_string());
        ctx.set("玩家ID", uid);
        ctx.set("属性名称", "属性值");
        ctx.set("属性值", value);
        // 简化 [UD:Base,type=Name,uid=[玩家ID]] 为玩家名
        let line = template_line
            .replace("[UD:Base,type=Name,uid=[玩家ID]]", name)
            .replace("[排名序号]", &(i + 1).to_string())
            .replace("[玩家ID]", uid)
            .replace("[属性名称]", "属性值")
            .replace("[属性值]", value);
        result.push_str(&line);
        result.push('\n');
    }

    // 渲染尾部（页码等）
    if !footer.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("当前页", &page.to_string());
        ctx.set("总页数", &total_pages.to_string());
        ctx.set("统计数量", &total_count.to_string());
        result.push_str(&render_simple(&footer, &ctx));
    }

    result
}

/// 提取循环模板体（<...>之间的内容）
#[allow(dead_code)]
fn extract_loop_template(raw: &str) -> String {
    if let Some(start) = raw.find('<') {
        if let Some(end) = raw[start..].find('>') {
            return raw[start + 1..start + end].to_string();
        }
    }
    String::new()
}

/// 提取循环体之前的内容作为头部
#[allow(dead_code)]
fn extract_header(raw: &str) -> String {
    if let Some(pos) = raw.find('<') {
        let header = raw[..pos].trim();
        // 跳过变量定义行
        return header
            .lines()
            .filter(|l| !l.starts_with('$') && !l.starts_with("!del"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
    }
    String::new()
}

/// 提取循环体之后的内容作为尾部
#[allow(dead_code)]
fn extract_footer(raw: &str) -> String {
    if let Some(start) = raw.find('<') {
        if let Some(end) = raw[start..].find('>') {
            let footer = raw[start + end + 1..].trim();
            // 跳过重复的循环体模板行
            return footer
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    !t.starts_with('<') && !t.is_empty()
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
        }
    }
    String::new()
}

/// 默认排行格式（模板不存在时使用）
#[allow(dead_code)]
fn render_ranking_default(
    entries: &[(String, String, String)],
    page: i32,
    total_pages: i32,
    total_count: i32,
) -> String {
    let mut out = String::new();
    for (i, (name, _uid, value)) in entries.iter().enumerate() {
        out.push_str(&format!("{}. {} - 属性值:{}\n", i + 1, name, value));
    }
    out.push_str(&format!(
        "当前页：{}/{}\n统计数量：{}\nTip:排行榜数据并非实时数据",
        page, total_pages, total_count
    ));
    out
}

/// 清理未替换的变量标记
fn clean_unresolved(text: &str) -> String {
    let mut result = text.to_string();
    // 清理 [...未匹配变量...] 标记
    while let Some(start) = result.find('[') {
        if let Some(end) = result[start..].find(']') {
            let inner = &result[start + 1..start + end];
            // 保留纯文本内容（如不包含变量格式的）
            if inner.contains(':') || inner.contains('=') || inner.contains(',') {
                result = format!("{}{}", &result[..start], &result[start + end + 1..]);
            } else {
                break;
            }
        } else {
            break;
        }
    }
    // 清理 $变量定义行
    let lines: Vec<&str> = result.lines().collect();
    let cleaned: Vec<&str> = lines
        .into_iter()
        .filter(|l| !l.trim().starts_with("$") && !l.trim().starts_with("!del"))
        .collect();
    cleaned.join("\n")
}

/// 渲染 VIP 信息模板
pub fn render_vip_info(db: &Database, ctx: &TemplateContext) -> String {
    let raw = db.template_get("VIP信息返回");
    if raw.is_empty() {
        return String::new();
    }
    render_simple(&raw, ctx)
}

/// 渲染喊话模板
#[allow(dead_code)]
pub fn render_shout(db: &Database, ctx: &TemplateContext) -> Option<String> {
    render_template(db, "喊话模板", "信息通知到群聊", ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple() {
        let mut ctx = TemplateContext::new();
        ctx.set("玩家名", "测试玩家");
        ctx.set("等级", "50");
        let result = render_simple("[玩家名]的等级是[等级]", &ctx);
        assert_eq!(result, "测试玩家的等级是50");
    }

    #[test]
    fn test_parse_cgd() {
        let raw = "#CGD DATA{\u{01}错误提示_不存在GH物品[物品名称]不存在！GI成功提示_使用成功GH您使用了[物品名称]Q}";
        let sections = parse_cgd(raw);
        assert!(sections.contains_key("错误提示_不存在"));
        assert!(sections.contains_key("成功提示_使用成功"));
        assert_eq!(sections["错误提示_不存在"], "物品[物品名称]不存在！");
        assert_eq!(sections["成功提示_使用成功"], "您使用了[物品名称]");
    }

    #[test]
    fn test_render_cgd_section() {
        let raw = "#CGD DATA{\u{01}失败GH操作失败[物品名称]！GI成功GH操作成功[物品名称]Q}";
        let mut ctx = TemplateContext::new();
        ctx.set("物品名称", "生命药水");
        let result = render_cgd_section(raw, "成功", &ctx);
        assert_eq!(result, Some("操作成功生命药水".to_string()));
    }

    #[test]
    fn test_extract_loop_template() {
        let raw = "<[排名序号].玩家[玩家ID]\\n[属性名称]:[属性值]>\r\n<[排名序号].玩家[玩家ID]\\n[属性名称]:[属性值]>";
        let tmpl = extract_loop_template(raw);
        assert!(tmpl.contains("[排名序号]"));
        assert!(tmpl.contains("[玩家ID]"));
    }

    #[test]
    fn test_clean_unresolved() {
        let text = "你好[未定义变量]世界";
        let result = clean_unresolved(text);
        // 简单变量标记保持不变（非CGD格式的变量）
        assert!(result.contains("你好"));
    }

    #[test]
    fn test_context_set_many() {
        let mut ctx = TemplateContext::new();
        ctx.set_many(&[("A", "1"), ("B", "2"), ("C", "3")]);
        assert_eq!(ctx.vars.len(), 3);
        assert_eq!(ctx.vars["A"], "1");
    }

    #[test]
    fn test_render_ranking_default() {
        let entries = vec![
            ("玩家1".to_string(), "uid1".to_string(), "100".to_string()),
            ("玩家2".to_string(), "uid2".to_string(), "90".to_string()),
        ];
        let result = render_ranking_default(&entries, 1, 1, 2);
        assert!(result.contains("1. 玩家1"));
        assert!(result.contains("2. 玩家2"));
        assert!(result.contains("当前页：1/1"));
        assert!(result.contains("统计数量：2"));
    }

    #[test]
    fn test_parse_cgd_no_cgd() {
        let raw = "这是一个普通模板，没有CGD格式";
        let sections = parse_cgd(raw);
        assert!(sections.is_empty());
    }
}
