#!/usr/bin/env python3
"""示例辅助脚本：统计文本行数。"""

import sys


def main() -> int:
    text = sys.stdin.read()
    lines = [line for line in text.splitlines() if line.strip()]
    print(f"非空行数：{len(lines)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
