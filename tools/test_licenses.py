import hashlib
import io
import json
from pathlib import Path
import struct
import tarfile
import tomllib
import unittest

from third_party import font_names

ROOT = Path(__file__).resolve().parents[1]


class LicenseTests(unittest.TestCase):
    def test_mpl_archives_are_offline_complete_sources_matching_the_lockfile(self):
        index = json.loads((ROOT / "licenses/THIRD_PARTY_INDEX.json").read_text(encoding="utf-8"))
        lock = {(p["name"], p["version"]): p for p in tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]}
        self.assertEqual({(s["name"], s["version"]) for s in index["mpl"]}, {(p["name"], p["version"]) for p in index["packages"] if "MPL-2.0" in p["license"]})
        for source in index["mpl"]:
            data = (ROOT / source["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(), lock[(source["name"], source["version"])]["checksum"])
            with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as package:
                files = package.getnames()
                self.assertTrue(any(f.endswith("/Cargo.toml") for f in files))
                self.assertTrue(any(f.endswith("/src/lib.rs") for f in files))
                self.assertTrue(any(f.endswith("/LICENSE.txt") for f in files))

    def test_font_notices_include_all_embedded_fonts_and_inherited_licenses(self):
        index = json.loads((ROOT / "licenses/THIRD_PARTY_INDEX.json").read_text(encoding="utf-8"))
        fonts = next(p for p in index["packages"] if p["name"] == "epaint_default_fonts")
        paths = {f["path"].split("/fonts/")[-1] for f in fonts["files"]}
        self.assertTrue({"Hack-Regular.txt", "OFL.txt", "UFL.txt", "emoji-icon-font-mit-license.txt"}.issubset(paths))
        self.assertTrue(any(f['path'].endswith('/upstream/LICENSE-MIT') for f in fonts['files']))
        text = (ROOT / "licenses/FONT_NOTICES.md").read_text(encoding="utf-8")
        notices = text + "\n".join((ROOT / f['path']).read_text(encoding="utf-8") for f in fonts['files'] if '/fonts/' in f['path'])
        for term in ["Hack-Regular.ttf", "NotoEmoji-Regular.ttf", "Ubuntu-Light.ttf", "emoji-icon-font.ttf", "Bitstream", "Source Foundry", "Google", "Canonical", "John Slegers"]:
            self.assertIn(term, notices)

    def test_font_name_parser_preserves_unicode_copyright_records(self):
        text = "Copyright © 示例作者"
        string = text.encode("utf-16-be")
        table = struct.pack(">HHH6H", 0, 1, 18, 3, 1, 0x409, 0, len(string), 0) + string
        font = struct.pack(">I4H", 0x10000, 1, 0, 0, 0) + struct.pack(">4sIII", b"name", 0, 28, len(table)) + table
        self.assertEqual(font_names(font), [(0, text)])

    def test_windows_renderer_has_complete_cc0_and_upstream_declaration(self):
        index = json.loads((ROOT / "licenses/THIRD_PARTY_INDEX.json").read_text(encoding="utf-8"))
        packages = {p["name"]: p for p in index["packages"]}
        self.assertIn("egui-wgpu", packages)
        self.assertIn("wgpu", packages)
        package = packages["hexf-parse"]
        self.assertEqual(package["version"], "0.2.1")
        self.assertEqual(package["license"], "CC0-1.0")
        files = {Path(f["path"]).name: ROOT / f["path"] for f in package["files"]}
        declaration = tomllib.loads(files["DECLARATION.toml"].read_text(encoding="utf-8"))
        self.assertEqual(declaration["package"]["license"], "CC0-1.0")
        text = files["LICENSE-CC0.txt"].read_text(encoding="utf-8")
        self.assertIn("CC0 1.0 Universal", text)
        self.assertIn("4. Limitations and Disclaimers.", text)
        sources = json.loads((ROOT / "licenses/supplemental/sources.json").read_text(encoding="utf-8"))
        source = next(s for s in sources if s.get("crate") == "hexf-parse-0.2.1")
        self.assertEqual(source["revision"], package["revision"])
        self.assertEqual(hashlib.sha256(files["LICENSE-CC0.txt"].read_bytes()).hexdigest(), source["sha256"])

    def test_png_pdf_and_icon_renderers_share_single_generation(self):
        index = json.loads((ROOT / "licenses/THIRD_PARTY_INDEX.json").read_text(encoding="utf-8"))
        packages = {(p["name"], p["version"]) for p in index["packages"]}
        for name in ["resvg", "usvg"]:
            self.assertIn((name, "0.45.1"), packages)
            self.assertFalse(any(n == name and v != "0.45.1" for n, v in packages))
        self.assertIn(("rustybuzz", "0.20.1"), packages)
        self.assertFalse(any(n == "rustybuzz" and v != "0.20.1" for n, v in packages))

if __name__ == "__main__":
    unittest.main()
