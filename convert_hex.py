"""
gamedata.sdb 十六进制解码工具
将所有 hex GBK/ASCII 编码的字段转回可读文本
支持多层嵌套解码（有些字段被编码了多次）
"""
import sqlite3
import shutil
import sys
import os

DB_PATH = "/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb"
BACKUP_PATH = DB_PATH + ".bak"

def is_hex_string(s):
    """检查字符串是否像 hex 编码（纯 0-9a-fA-F，偶数长度，>=4字符）"""
    if not s or len(s) < 4 or len(s) % 2 != 0:
        return False
    try:
        int(s, 16)
        return True
    except ValueError:
        return False

def try_decode_hex(s):
    """尝试将 hex 字符串解码为可读文本"""
    if not is_hex_string(s):
        return s, False
    
    try:
        raw_bytes = bytes.fromhex(s)
        # 先尝试 GBK
        try:
            decoded = raw_bytes.decode('gbk')
            # 检查解码结果是否包含可读字符
            if any('\u4e00' <= c <= '\u9fff' for c in decoded):
                return decoded, True
            # 纯 ASCII 解码结果
            if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                return decoded, True
        except (UnicodeDecodeError, ValueError):
            pass
        
        # 尝试 UTF-8
        try:
            decoded = raw_bytes.decode('utf-8')
            if any('\u4e00' <= c <= '\u9fff' for c in decoded):
                return decoded, True
            if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                return decoded, True
        except (UnicodeDecodeError, ValueError):
            pass
        
        # 尝试 ASCII
        try:
            decoded = raw_bytes.decode('ascii')
            if all(c.isprintable() or c in '\r\n\t' for c in decoded):
                return decoded, True
        except (UnicodeDecodeError, ValueError):
            pass
    except ValueError:
        pass
    
    return s, False

def deep_decode(s, max_depth=5):
    """深度解码：反复尝试解码，直到无法继续"""
    if not s:
        return s
    
    current = s
    for _ in range(max_depth):
        decoded, ok = try_decode_hex(current)
        if not ok or decoded == current:
            break
        current = decoded
    
    return current

def main():
    # 备份
    if not os.path.exists(BACKUP_PATH):
        print(f"备份数据库到 {BACKUP_PATH}")
        shutil.copy2(DB_PATH, BACKUP_PATH)
    else:
        print(f"备份已存在: {BACKUP_PATH}")
    
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    
    # 获取所有表
    c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
    tables = [r[0] for r in c.fetchall()]
    
    total_updated = 0
    tables_updated = 0
    
    for tname in tables:
        c.execute(f"PRAGMA table_info({tname})")
        cols = c.fetchall()
        
        # 只处理 VARCHAR/TEXT 列
        text_cols = []
        for col in cols:
            col_name = col[1]
            col_type = col[2].upper()
            if col_type in ('VARCHAR', 'TEXT', 'CHAR', ''):
                text_cols.append(col_name)
        
        if not text_cols:
            continue
        
        # 获取主键（用于 WHERE 条件）
        c.execute(f"PRAGMA table_info({tname})")
        all_cols = [(col[0], col[1]) for col in c.fetchall()]
        
        # 读取所有行
        col_names = [col[1] for col in cols]
        col_list = ', '.join(f'[{cn}]' for cn in col_names)
        c.execute(f"SELECT rowid, {col_list} FROM {tname}")
        rows = c.fetchall()
        
        table_updated = 0
        for row in rows:
            rowid = row[0]
            values = row[1:]
            
            # 检查每列是否需要解码
            updates = {}
            for i, (col_name, val) in enumerate(zip(col_names, values)):
                if col_name not in text_cols:
                    continue
                if val is None:
                    continue
                
                val_str = str(val)
                decoded = deep_decode(val_str)
                if decoded != val_str:
                    updates[col_name] = decoded
            
            if updates:
                # 构造 UPDATE 语句
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
            tables_updated += 1
            total_updated += table_updated
            print(f"  {tname}: {table_updated}/{len(rows)} 行已解码")
    
    conn.commit()
    conn.close()
    
    print(f"\n完成! 共更新 {tables_updated} 张表, {total_updated} 行")

if __name__ == "__main__":
    main()
