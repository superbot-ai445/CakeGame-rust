"""
gamedata.sdb 第二轮深度解码
处理嵌套在 JSON/config 格式内的 hex 编码
"""
import sqlite3
import re
import os

DB_PATH = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb"

def decode_embedded_hex(s):
    """解码嵌入在文本中的 hex 模式"""
    if not s or not isinstance(s, str):
        return s
    
    # 匹配 6+ 位连续 hex 字符（至少3个字节，避免误匹配数字）
    # 但排除明显不是 hex 的模式（如纯数字、JSON key 等）
    def replace_hex(match):
        hex_str = match.group(0)
        try:
            raw = bytes.fromhex(hex_str)
            # 尝试 GBK
            try:
                decoded = raw.decode('gbk')
                if any('\u4e00' <= c <= '\u9fff' for c in decoded):
                    return decoded
                if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                    return decoded
            except:
                pass
            # 尝试 UTF-8
            try:
                decoded = raw.decode('utf-8')
                if any('\u4e00' <= c <= '\u9fff' for c in decoded):
                    return decoded
                if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                    return decoded
            except:
                pass
            # 尝试 ASCII
            try:
                decoded = raw.decode('ascii')
                if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                    return decoded
            except:
                pass
        except:
            pass
        return hex_str  # 无法解码则原样返回
    
    # 匹配 6+ 位 hex（避免误匹配短数字）
    result = re.sub(r'(?<![0-9A-Fa-f])[0-9A-Fa-f]{6,}(?![0-9A-Fa-f])', replace_hex, s)
    
    # 再匹配更长的 hex（8+位，更保守）
    if result == s:
        result = re.sub(r'[0-9A-Fa-f]{8,}', replace_hex, s)
    
    return result

def main():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    
    c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
    tables = [r[0] for r in c.fetchall()]
    
    total_updated = 0
    
    for tname in tables:
        c.execute(f"PRAGMA table_info({tname})")
        cols = c.fetchall()
        col_names = [col[1] for col in cols]
        col_types = [col[2].upper() for col in cols]
        
        # 只处理文本列
        text_indices = [i for i, t in enumerate(col_types) if t in ('VARCHAR', 'TEXT', 'CHAR', '')]
        if not text_indices:
            continue
        
        col_list = ', '.join(f'[{cn}]' for cn in col_names)
        c.execute(f"SELECT rowid, {col_list} FROM {tname}")
        rows = c.fetchall()
        
        table_updated = 0
        for row in rows:
            rowid = row[0]
            values = row[1:]
            
            updates = {}
            for i in text_indices:
                val = values[i]
                if val is None:
                    continue
                
                val_str = str(val)
                decoded = decode_embedded_hex(val_str)
                if decoded != val_str:
                    updates[col_names[i]] = decoded
            
            if updates:
                set_parts = []
                set_values = []
                for col_name, new_val in updates.items():
                    set_parts.append(f"[{col_name}]=?")
                    set_values.append(new_val)
                
                set_values.append(rowid)
                sql = f"UPDATE [{tname}] SET {', '.join(set_parts)} WHERE rowid=?"
                c.execute(sql, set_values)
                table_updated += 1
        
        if table_updated > 0:
            total_updated += table_updated
            print(f"  {tname}: {table_updated} 行深度解码")
    
    conn.commit()
    conn.close()
    
    print(f"\n完成! 共更新 {total_updated} 行")

if __name__ == "__main__":
    main()
