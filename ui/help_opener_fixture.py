#!/usr/bin/python3
"""Owned test-only opener; never opens a URI"""
import json
import os
import sys
import time
with open(os.environ["BGR_HELP_OPENER_LOG"], "a", encoding="utf-8") as stream:
    stream.write(json.dumps({"pid": os.getpid(), "argv": sys.argv[1:]}) + "\n")
time.sleep(float(os.environ.get("BGR_HELP_OPENER_DELAY", "0")))
raise SystemExit(int(os.environ.get("BGR_HELP_OPENER_STATUS", "0")))
