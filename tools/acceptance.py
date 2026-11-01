"""结构化验收报告；只读清单，不改变功能状态。

python tools/acceptance.py [--check | --manual-template 模块名]
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "docs/parity"))
from build_parity import EVIDENCE_KINDS, missing_evidence, validate  # noqa: E402


def manual_template(row):
    return f"""### {row['id']} {row['feature']}

准备：打开 samples/ 下的示例项目 ____，选中 ____。

| # | 操作 | 预期 | 结果 |
|---|---|---|---|
| 1 | ____ | ____ | |
| 2 | ____ | ____ | |
| 3 | ____ | ____ | |

记录：exe 提交号 ____ ；日期 ____ ；结论 通过 / 未通过；问题 ____
"""


def report(rows):
    pending = [r for r in rows if r["status"] == "已实现待验收"]
    lines = []
    for module in dict.fromkeys(r["module"] for r in pending):
        lines += [f"## {module}", ""]
        for row in pending:
            if row["module"] != module:
                continue
            evidence = row.get("evidence", {})
            lines += [f"{row['id']} {row['feature']} [{row['pri']}]",
                      f"  needs: {', '.join(row.get('needs', []))}",
                      "  evidence: " + json.dumps(evidence, ensure_ascii=False),
                      "  缺少: " + (", ".join(missing_evidence(row)) or "无（等待状态复核）")]
    counts = Counter(k for r in pending for k in missing_evidence(r))
    lines += ["", f"待验收 {len(pending)} 条；P0 仍缺证据 {sum(r['pri'] == 'P0' and bool(missing_evidence(r)) for r in pending)} 条；P0 待状态复核 {sum(r['pri'] == 'P0' and not missing_evidence(r) for r in pending)} 条。",
              "缺少证据：" + "，".join(f"{k} {counts[k]}" for k in EVIDENCE_KINDS) + "。"]
    return "\n".join(lines)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--manual-template", metavar="模块名")
    args = parser.parse_args()
    rows = json.loads((ROOT / "docs/parity/parity.json").read_text(encoding="utf-8"))
    validate(rows)
    if args.check:
        print(f"验收证据一致性通过：{len(rows)} 条。")
    elif args.manual_template:
        matches = [r for r in rows if r["module"] == args.manual_template and "manual" in r.get("needs", [])]
        if not matches:
            parser.error("该模块没有需要人工验收的条目")
        print("\n".join(manual_template(r) for r in matches))
    else:
        print(report(rows))


if __name__ == "__main__":
    main()
