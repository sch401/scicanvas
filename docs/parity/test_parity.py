import copy
import json
from pathlib import Path
import unittest

import build_parity


class ParityTests(unittest.TestCase):
    def setUp(self):
        rows = json.loads((Path(__file__).parent / "parity.json").read_text(encoding="utf-8"))
        self.row = next(r for r in rows if r["id"] == "F201")

    def test_inconsistent_subitem_counts_are_rejected(self):
        for change in [{"done": 5}, {"total": -1}, {"review": True}, {"done": 3, "review": 2}]:
            with self.subTest(change=change):
                row = copy.deepcopy(self.row)
                next(iter(row["sub_items"].values())).update(change)
                with self.assertRaises(ValueError):
                    build_parity.validate([row])

    def test_generation_preserves_ids_and_prevents_embedded_script_breakout(self):
        row = copy.deepcopy(self.row)
        row["how"] = '</script><script>alert("x")</script>'
        before = copy.deepcopy(row)
        output = build_parity.render([row])
        self.assertEqual(row, before)
        self.assertIn('"id":"F201"', output["parity.html"])
        self.assertNotIn(row["how"], output["parity.html"])
        self.assertIn("\\u003c/script>", output["parity.html"])


    def evidence_row(self):
        row = copy.deepcopy(self.row)
        row["needs"] = ["auto", "reference", "visual", "manual"]
        row["evidence"] = {
            "auto": ["crates/scicanvas-core/src/csv.rs::units_from_header"],
            "reference": ["samples/README.md"],
            "visual": ["samples/README.md"],
            "manual": {"checklist": "docs/acceptance/worksheet.md#F027", "result": "通过", "date": "2026-11-01", "build": "test-fixture"},
        }
        return row

    def test_evidence_schema_positive_and_negative_cases(self):
        # 文件仅用于校验器正例，不作为条目实际参考/视觉证据。
        positive = self.evidence_row()
        positive["status"] = "已完成"
        build_parity.validate([positive])
        optional = copy.deepcopy(self.row)
        optional.pop("needs", None)
        optional.pop("evidence", None)
        build_parity.validate([optional])
        changes = [
            lambda r: r.update(needs=["unknown"]),
            lambda r: r.update(needs=["auto", "auto"]),
            lambda r: r["evidence"].update(other=[]),
            lambda r: r["evidence"].update(auto=["missing.rs::test"]),
            lambda r: r["evidence"].update(auto=["crates/scicanvas-core/src/csv.rs::missing_function"]),
            lambda r: r["evidence"].update(auto=["crates/scicanvas-core/src/csv.rs"]),
            lambda r: r["evidence"].update(reference=["missing.json"]),
            lambda r: r["evidence"].update(visual=["missing.md"]),
            lambda r: r["evidence"]["manual"].update(checklist="missing.md#F027"),
            lambda r: r["evidence"]["manual"].update(checklist="docs/acceptance/worksheet.md#F999"),
            lambda r: r["evidence"]["manual"].update(result="完成"),
            lambda r: r["evidence"]["manual"].update(date=""),
        ]
        for i, change in enumerate(changes):
            with self.subTest(rule=i):
                row = copy.deepcopy(positive)
                change(row)
                with self.assertRaises(ValueError):
                    build_parity.validate([row])
        for kind in build_parity.EVIDENCE_KINDS:
            with self.subTest(incomplete=kind):
                row = copy.deepcopy(positive)
                if kind == "manual":
                    row["evidence"][kind]["result"] = "待做"
                else:
                    row["evidence"][kind] = []
                with self.assertRaises(ValueError):
                    build_parity.validate([row])
        for result in ("待做", "未通过", "通过"):
            row = self.evidence_row()
            row["evidence"]["manual"]["result"] = result
            build_parity.validate([row])

    def test_manual_p0_coverage_pending_needs_and_accepted_evidence(self):
        rows = json.loads((Path(__file__).parent / "parity.json").read_text(encoding="utf-8"))
        build_parity.validate(rows)
        self.assertEqual(sum(r["status"] == "已完成" for r in rows), 81)
        self.assertEqual(sum(r["status"] == "已实现待验收" for r in rows), 102)
        self.assertEqual(sum(r["status"] == "部分实现" for r in rows), 7)
        self.assertEqual(sum(r["status"] == "未开始" for r in rows), 311)
        for row in rows:
            if row["status"] in ("已完成", "已实现待验收"):
                self.assertTrue(row.get("needs"), row["id"])
            if row["pri"] == "P0" and "manual" in row.get("needs", []):
                self.assertIn("manual", row.get("evidence", {}), row["id"])
            self.assertEqual(row["ref"], "")


    def test_document_wording_has_no_damaged_replacements(self):
        import re

        docs = Path(__file__).resolve().parent.parent
        paths = [docs / "parity/parity.json"] + sorted(
            path for path in docs.rglob("*")
            if path.suffix in (".md", ".html") and "_gpt_session" not in path.parts
        )
        damaged = re.compile(r"[ ]科研绘图[ ]|/科[研]绘图|scientific[ ]plotting|来源[：]；|^。", re.MULTILINE)
        for path in paths:
            with self.subTest(path=str(path.relative_to(docs))):
                text = path.read_text(encoding="utf-8")
                match = damaged.search(text)
                if match:
                    line = text.count("\n", 0, match.start()) + 1
                    self.fail(f"Damaged wording: {path.relative_to(docs)}:{line}: {match.group()}")


if __name__ == "__main__":
    unittest.main()
