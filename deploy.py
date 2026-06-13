#!/usr/bin/env python3
"""Deploy libcakegame.so to HuggingFace Space"""
import os
os.environ["HF_ENDPOINT"] = "https://hf-mirror.com"

from huggingface_hub import HfApi

api = HfApi(token=open(os.path.expanduser("~/.cache/huggingface/token")).read().strip())

# Upload the binary
api.upload_file(
    path_or_fileobj="/opt/data/cakegame-rs/cakegame-v2/target/release/libcakegame.so",
    path_in_repo="libcakegame.so",
    repo_id="smithjacki/hermes",
    repo_type="space",
)
print("✅ Uploaded libcakegame.so")

# Restart the space
api.restart_space("smithjacki/hermes")
print("✅ Space restart triggered")
