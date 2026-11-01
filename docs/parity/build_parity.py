"""Render parity.json without changing IDs or audited metadata.

python docs/parity/build_parity.py [--check | --desktop]
The optional --desktop also updates the user's copy next to this checkout.
"""
import argparse
from collections import Counter
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
STATUSES = ["已完成", "已实现待验收", "部分实现", "未开始", "不做"]
MILESTONES = {"P0": "M1 日常够用", "P1": "M2 论文全流程", "P2": "M3 高级功能", "不做": "—"}


EVIDENCE_KINDS = ("auto", "reference", "visual", "manual")
ROOT = HERE.parents[1]


def validate_evidence(row):
    feature_id = row["id"]
    needs = row.get("needs", [])
    if not isinstance(needs, list) or any(k not in EVIDENCE_KINDS for k in needs) or len(set(needs)) != len(needs):
        raise ValueError(f"Invalid needs: {feature_id}")
    evidence = row.get("evidence", {})
    if not isinstance(evidence, dict) or any(k not in EVIDENCE_KINDS for k in evidence):
        raise ValueError(f"Invalid evidence: {feature_id}")
    for kind in ("auto", "reference", "visual"):
        entries = evidence.get(kind, [])
        if not isinstance(entries, list) or any(not isinstance(v, str) or not v for v in entries):
            raise ValueError(f"Invalid {kind} evidence: {feature_id}")
        for entry in entries:
            filename, separator, name = entry.partition("::") if kind == "auto" else (entry, "", "")
            path = ROOT / filename
            if not path.is_file():
                raise ValueError(f"Missing evidence file: {feature_id}/{entry}")
            if kind == "auto" and (not separator or not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name) or not re.search(r"\bfn\s+" + re.escape(name) + r"\b", path.read_text(encoding="utf-8"))):
                raise ValueError(f"Missing test function: {feature_id}/{entry}")
    if "manual" in evidence:
        manual = evidence["manual"]
        if not isinstance(manual, dict) or manual.get("result") not in ("通过", "未通过", "待做"):
            raise ValueError(f"Invalid manual result: {feature_id}")
        filename, separator, anchor = manual.get("checklist", "").partition("#")
        path = ROOT / filename
        if not separator or not path.is_file() or not re.search(r"^### " + re.escape(anchor) + r"(?:\s|$)", path.read_text(encoding="utf-8"), re.M):
            raise ValueError(f"Missing manual heading: {feature_id}")
        if manual["result"] != "待做" and (not manual.get("date") or not manual.get("build")):
            raise ValueError(f"Missing manual date/build: {feature_id}")
    if row["status"] == "已完成":
        for kind in needs:
            if not evidence.get(kind) or (kind == "manual" and evidence[kind]["result"] != "通过"):
                raise ValueError(f"Incomplete accepted evidence: {feature_id}/{kind}")


def missing_evidence(row):
    evidence = row.get("evidence", {})
    return [k for k in row.get("needs", []) if not evidence.get(k) or (k == "manual" and evidence[k]["result"] != "通过")]


def validate(rows):
    ids = set()
    for row in rows:
        feature_id = row["id"]
        if feature_id in ids or not feature_id.startswith("F") or not feature_id[1:].isdigit():
            raise ValueError(f"Duplicate or invalid ID: {feature_id}")
        ids.add(feature_id)
        validate_evidence(row)
        if row["status"] not in STATUSES or row["milestone"] != MILESTONES[row["pri"]]:
            raise ValueError(f"Invalid status or milestone: {feature_id}")
        for field in ["feature", "module", "ref", "how", "verify"]:
            if not isinstance(row[field], str):
                raise ValueError(f"Invalid {field}: {feature_id}")
        for name, progress in row.get("sub_items", {}).items():
            counts = [progress["total"], progress["done"], progress.get("review", 0), progress.get("partial", 0)]
            if not name or any(type(n) is not int or n < 0 for n in counts) or counts[0] == 0 or sum(counts[1:]) > counts[0]:
                raise ValueError(f"Invalid sub-item counts: {feature_id}/{name}")
            if not isinstance(progress["note"], str) or not progress["note"]:
                raise ValueError(f"Missing sub-item scope: {feature_id}/{name}")


def sub_text(row):
    return "；".join(
        f"{name}：已完成 {p['done']}/{p['total']}，待验收 {p.get('review', 0)}，部分实现 {p.get('partial', 0)}；{p['note']}"
        for name, p in row.get("sub_items", {}).items()
    )


def render(rows):
    validate(rows)
    priorities, statuses = Counter(r["pri"] for r in rows), Counter(r["status"] for r in rows)
    md = [
        "# SciCanvas 功能对照清单", "",
        "条目按 Origin 帮助文档的目录整理，只引用章节标题；实现按公开说明和观察到的行为编写。",
        "数据源为 `parity.json`，页面模板为 `tracker_template.html`。运行 `python docs/parity/build_parity.py` 生成文档与页面；`--check` 检查同步，`--desktop` 同步桌面副本。新增条目追加稳定编号，已有编号保留。", "",
        f"共 {len(rows)} 项：" + "，".join(f"{k} {priorities[k]} 项" for k in MILESTONES) + "。",
        "状态：" + "，".join(f"{k} {statuses[k]} 项" for k in STATUSES) + "。", "",
        "- **P0 / M1 日常够用**：日常数据处理和论文作图的最小集合。",
        "- **P1 / M2 论文全流程**：统计、多面板、信号处理等。",
        "- **P2 / M3 高级功能**：3D、专业图与高级分析。",
        "- **部分实现**：范围内仍有明确缺口；**已实现待验收**：已接入工作流，专项或参考对照待完成。",
        "- **已完成**：完成当前条目的实现与既定验证；**未开始**按桌面代码判断，旧网页演示单独说明。",
        "- **子项**：`sub_items` 按名称记录 `total`、`done`、可选 `review` / `partial`、`note`。已完成与待验收分别计数；未开始为总数减去以上计数。",
        "- 子项独立于共享存储中的条目状态，修改条目状态不会自动增加子项完成数。规划目标与路线参考基数在备注中标明；重叠目录分别查看，避免重复加总。",
        "- F489 起补充第三阶段需求。F514 为 F148–F156 的目录总览，239 是路线参考数；当前通用线性/多项式回归单独计数。",
        "- Origin 私有格式与脚本兼容保留原范围；扩展包采用 SciCanvas 自有格式。", "",
        "验证原则：分析与 NIST StRD、SciPy / R 对照；图形与 matplotlib 同数据出图比较数据位置与刻度。", "",
    ]
    for module in dict.fromkeys(r["module"] for r in rows):
        md += [f"## {module}", "", "| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |", "|---|---|---|---|---|---|---|---|"]
        for row in rows:
            if row["module"] == module:
                cells = [row[k] for k in ["id", "feature", "ref", "pri", "status", "how", "verify"]] + [sub_text(row)]
                md.append("| " + " | ".join(str(c or "—").replace("|", "\\|").replace("\n", " ") for c in cells) + " |")
        md.append("")
    template = (HERE / "tracker_template.html").read_text(encoding="utf-8")
    if template.count("__DATA__") != 1:
        raise ValueError("Expected exactly one __DATA__ placeholder")
    data = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).replace("<", "\\u003c")
    return {"PARITY.md": "\n".join(md), "parity.html": template.replace("__DATA__", data)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--desktop", action="store_true")
    args = parser.parse_args()
    outputs = render(json.loads((HERE / "parity.json").read_text(encoding="utf-8")))
    for name, content in outputs.items():
        path = HERE / name
        if args.check:
            if path.read_text(encoding="utf-8") != content:
                raise SystemExit(f"Out of sync: {path}; run python docs/parity/build_parity.py")
        else:
            path.write_text(content, encoding="utf-8")
    if args.desktop:
        (HERE.parents[2] / "SciCanvas 对照清单.html").write_text(outputs["parity.html"], encoding="utf-8")


if __name__ == "__main__":
    main()
