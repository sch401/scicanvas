import tempfile
import unittest
from pathlib import Path
import xml.etree.ElementTree as ET

import build_msi


class MsiTests(unittest.TestCase):
    def test_payload_registers_three_file_types_with_stable_components(self):
        ns = {"w": build_msi.WIX}
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stage = root / "stage"
            (stage / "samples").mkdir(parents=True)
            (stage / "SciCanvas.exe").write_bytes(b"exe")
            (stage / "samples" / "example.scv").write_bytes(b"sample")
            first, second = root / "first.wxi", root / "second.wxi"
            build_msi.payload(stage, first)
            build_msi.payload(stage, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            xml = ET.parse(first)
            files = xml.findall(".//w:File", ns)
            self.assertEqual({f.get("Name") for f in files}, {"SciCanvas.exe", "example.scv"})
            values = xml.findall(".//w:RegistryValue", ns)
            keys = {v.get("Key") for v in values}
            self.assertTrue({r"Software\Classes\.scv", r"Software\Classes\.scgt", r"Software\Classes\.scfn"} <= keys)
            shortcuts = xml.findall(".//w:Shortcut", ns)
            self.assertEqual(len(shortcuts), 1)
            self.assertEqual(shortcuts[0].get("Icon"), "ProductIcon")
            self.assertEqual(shortcuts[0].get("IconIndex"), "0")
            self.assertEqual(len([key for key in keys if key.endswith(r"\DefaultIcon")]), 3)

    def test_user_data_removal_requires_an_explicit_choice_and_is_skipped_on_upgrade(self):
        ns = {"w": build_msi.WIX}
        xml = ET.parse(build_msi.ROOT / "installer/scicanvas.wxs")
        package = xml.find(".//w:Package", ns)
        self.assertEqual(package.get("Scope"), "perUser")
        choice = xml.find(".//w:Property[@Id='DELETEUSERDATA']", ns)
        self.assertEqual(choice.get("Secure"), "yes")
        action = xml.find(".//w:InstallExecuteSequence/w:Custom", ns)
        self.assertIn('DELETEUSERDATA="1"', action.get("Condition"))
        self.assertIn("NOT UPGRADINGPRODUCTCODE", action.get("Condition"))
        self.assertEqual(action.get("After"), "InstallFinalize")
        check = xml.find(".//w:Control[@Id='DeleteData']", ns)
        self.assertEqual(check.get("Property"), "DELETEUSERDATA")


    def test_standard_wizard_has_path_selection_and_product_branding(self):
        ns = {"w": build_msi.WIX, "ui": build_msi.WIX + "/ui"}
        xml = ET.parse(build_msi.ROOT / "installer/scicanvas.wxs")
        wizard = xml.find(".//ui:WixUI", ns)
        self.assertEqual(wizard.get("Id"), "WixUI_InstallDir")
        self.assertEqual(wizard.get("InstallDirectory"), "INSTALLFOLDER")
        self.assertEqual(xml.find(".//w:MajorUpgrade", ns).get("IgnoreLanguage"), "yes")
        variables = {v.get("Id") for v in xml.findall(".//w:WixVariable", ns)}
        self.assertTrue({"WixUIDialogBmp", "WixUIBannerBmp", "WixUILicenseRtf"} <= variables)
        for name in ["dialog.bmp", "banner.bmp", "product.ico", "license.rtf"]:
            self.assertGreater((build_msi.ROOT / "installer/assets" / name).stat().st_size, 0)


if __name__ == "__main__":
    unittest.main()
