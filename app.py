"""CakeGame HF Space - FastAPI API server for the Rust engine (v2.1 - report system)"""
import ctypes
import os
import json
from fastapi import FastAPI, Query
from fastapi.responses import JSONResponse
import uvicorn

# ==================== FFI Setup ====================

LIB_PATH = os.path.join(os.path.dirname(__file__), "libcakegame.so")
DB_PATH = os.path.join(os.path.dirname(__file__), "gamedata.sdb")

lib = ctypes.CDLL(LIB_PATH)
lib.cg_init.argtypes = [ctypes.c_char_p]
lib.cg_init.restype = ctypes.c_int
lib.cg_process.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.cg_process.restype = ctypes.c_void_p
lib.cg_free_string.argtypes = [ctypes.c_void_p]
lib.cg_free_string.restype = None
lib.cg_get_user_info.argtypes = [ctypes.c_char_p]
lib.cg_get_user_info.restype = ctypes.c_void_p
lib.cg_get_knapsack.argtypes = [ctypes.c_char_p]
lib.cg_get_knapsack.restype = ctypes.c_void_p
lib.cg_get_equips.argtypes = [ctypes.c_char_p]
lib.cg_get_equips.restype = ctypes.c_void_p
lib.cg_shutdown.argtypes = []
lib.cg_shutdown.restype = None


def read_cstr(ptr):
    if ptr is None:
        return ""
    result = ctypes.cast(ptr, ctypes.c_char_p).value
    lib.cg_free_string(ptr)
    if result is None:
        return ""
    return result.decode("utf-8", errors="replace")


def process_cmd(msg: str, uid: str, msg_type: str = "2", group: str = "0") -> str:
    result_ptr = lib.cg_process(
        msg.encode("utf-8"),
        msg_type.encode("utf-8"),
        group.encode("utf-8"),
        uid.encode("utf-8"),
    )
    return read_cstr(result_ptr)


# Init engine on startup
init_status = lib.cg_init(DB_PATH.encode("utf-8"))
print(f"Engine init: {init_status}")

# ==================== FastAPI App ====================

app = FastAPI(title="CakeGame API", version="2.0")


@app.get("/api/health")
def health():
    return {"ok": init_status == 0, "status": "ok", "engine_init": init_status}


@app.get("/api/cmd")
def api_cmd(msg: str = Query(""), uid: str = Query("anonymous")):
    if not msg:
        return JSONResponse({"error": "missing msg parameter"}, status_code=400)
    result = process_cmd(msg, uid)
    return {"result": result, "msg": msg, "uid": uid}


@app.get("/api/batch")
def api_batch(cmds: str = Query(""), uid: str = Query("anonymous")):
    if not cmds:
        return JSONResponse({"error": "missing cmds parameter"}, status_code=400)
    cmd_list = cmds.split("|")
    results = []
    for cmd in cmd_list:
        cmd = cmd.strip()
        if cmd:
            result = process_cmd(cmd, uid)
            results.append({"cmd": cmd, "result": result})
    return {"results": results, "uid": uid}


@app.get("/api/user/{user_id}")
def api_user(user_id: str):
    result_ptr = lib.cg_get_user_info(user_id.encode("utf-8"))
    info = read_cstr(result_ptr)
    try:
        return json.loads(info)
    except Exception:
        return {"raw": info}


@app.get("/api/knapsack/{user_id}")
def api_knapsack(user_id: str):
    result_ptr = lib.cg_get_knapsack(user_id.encode("utf-8"))
    info = read_cstr(result_ptr)
    try:
        return json.loads(info)
    except Exception:
        return {"raw": info}


@app.get("/api/equips/{user_id}")
def api_equips(user_id: str):
    result_ptr = lib.cg_get_equips(user_id.encode("utf-8"))
    info = read_cstr(result_ptr)
    try:
        return json.loads(info)
    except Exception:
        return {"raw": info}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=7860)
# Updated Tue Jun  9 08:20:42 UTC 2026
