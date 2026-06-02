#!/usr/bin/env python3
from __future__ import annotations

import sys
import time

for _line in sys.stdin:
    time.sleep(5)
    print('{"protocolVersion":"2000m.driver.v3","requestId":"late","ok":false}', flush=True)
