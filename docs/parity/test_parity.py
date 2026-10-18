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


if __name__ == "__main__":
    unittest.main()
