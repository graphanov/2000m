#!/usr/bin/env python3
from __future__ import annotations

import json
import sys

for line in sys.stdin:
    request = json.loads(line)
    sys.stderr.write("x" * 262144)
    sys.stderr.flush()
    print(json.dumps({
        "protocolVersion": "2000m.driver.v3",
        "requestId": request["requestId"],
        "ok": True,
        "payload": {"status": "ok"}
    }), flush=True)
