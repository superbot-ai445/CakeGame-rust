/// CakeGame 编码工具
/// 判断字符串是否为十六进制编码（GBK）
pub fn is_hex_encoded(s: &str) -> bool {
    if s.is_empty() || !s.len().is_multiple_of(2) {
        return false;
    }
    s.chars().all(|c| c.is_ascii_hexdigit())
}

/// 将 GBK 十六进制字符串解码为 UTF-8
pub fn decode_hex_gbk(s: &str) -> String {
    if !is_hex_encoded(s) {
        return s.to_string();
    }

    // 将十六进制转换为字节
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect();

    if bytes.is_empty() {
        return s.to_string();
    }

    // 尝试 GBK 解码
    let (decoded, _, _) = encoding_rs::GBK.decode(&bytes);
    decoded.into_owned()
}

/// 智能解码：如果是十六进制就尝试解码，否则原样返回
pub fn smart_decode(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    // 检查是否看起来像十六进制
    if s.len() >= 4 && s.len().is_multiple_of(2) && is_hex_encoded(s) {
        // 检查是否包含 GBK 特征字节 (A1-F7)
        let has_gbk_pattern = (0..s.len()).step_by(2).any(|i| {
            let byte = u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0);
            (0xA1..=0xF7).contains(&byte)
        });

        if has_gbk_pattern {
            return decode_hex_gbk(s);
        }

        // 纯 ASCII hex (如 [NULL] = 5B4E554C4C5D)，尝试 GBK 解码
        let bytes: Vec<u8> = (0..s.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
            .collect();
        if !bytes.is_empty() {
            let (decoded, _, _) = encoding_rs::GBK.decode(&bytes);
            let result = decoded.into_owned();
            // 只有解码结果是可打印字符才返回
            if result
                .chars()
                .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace() || c == '[' || c == ']')
            {
                return result;
            }
        }
    }

    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_gbk() {
        // 【普通】生命药水
        let hex = "A1BEC6D5CDA8A1BFC9FAC3FCD2A9CBAE";
        let result = decode_hex_gbk(hex);
        assert_eq!(result, "【普通】生命药水");
    }

    #[test]
    fn test_smart_decode() {
        // 非 hex 字符串原样返回
        assert_eq!(smart_decode("hello"), "hello");
        assert_eq!(smart_decode(""), "");

        // GBK hex 解码
        let hex = "A1BEC6D5CDA8A1BFC9FAC3FCD2A9CBAE";
        assert_eq!(smart_decode(hex), "【普通】生命药水");
    }

    #[test]
    fn test_is_hex() {
        assert!(is_hex_encoded("A1BEC6D5"));
        assert!(!is_hex_encoded("hello"));
        assert!(!is_hex_encoded(""));
        assert!(!is_hex_encoded("A1BEC6D")); // 奇数长度
    }
}
