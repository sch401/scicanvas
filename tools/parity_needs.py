"""一次性初始建议：python tools/parity_needs.py；核对后手工写回 parity.json。

保留本脚本以说明分类来源；不自动写回或改变功能状态。
"""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def suggest(row):
    text = row["verify"]
    needs = ["auto"]
    if re.search(r"NIST|SciPy|NumPy|numpy|参考值|\bR\b", text):
        needs.append("reference")
    if re.search(r"绘图|图类型|作图|导出与发布", row["module"]) and re.search(r"目视|Origin 同数据|视觉|对照", text):
        needs.append("visual")
    if re.search(r"人工|手工|手动操作|实际拖放|粘贴到 Word|原生文件选择", text):
        needs.append("manual")
    return needs


def main():
    rows = json.loads((ROOT / "docs/parity/parity.json").read_text(encoding="utf-8"))
    print("| 编号 | 功能 | needs |")
    print("|---|---|---|")
    for row in rows:
        if row["status"] == "已实现待验收":
            print(f"| {row['id']} | {row['feature']} | {', '.join(suggest(row))} |")


if __name__ == "__main__":
    main()
