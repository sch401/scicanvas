import copy
import unittest

from acceptance import manual_template, missing_evidence, report


class AcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.row = dict(id="F900", feature="验收用例", module="测试模块", pri="P0",
                        status="已实现待验收", needs=["auto", "manual"],
                        evidence={"auto": ["file.rs::test"], "manual": {"result": "待做"}})

    def test_pending_manual_evidence_is_still_missing(self):
        before = copy.deepcopy(self.row)
        self.assertEqual(missing_evidence(self.row), ["manual"])
        text = report([self.row])
        self.assertIn("P0 仍缺证据 1 条", text)
        self.assertIn("manual 1", text)
        self.assertEqual(self.row, before)
        self.row["evidence"]["manual"]["result"] = "通过"
        self.assertEqual(missing_evidence(self.row), [])
        self.assertIn("P0 待状态复核 1 条", report([self.row]))

    def test_manual_template_contains_heading_blank_results_and_record(self):
        text = manual_template(self.row)
        for value in ("### F900", "| # | 操作 | 预期 | 结果 |", "exe 提交号 ____", "结论 通过 / 未通过"):
            self.assertIn(value, text)


if __name__ == "__main__":
    unittest.main()
