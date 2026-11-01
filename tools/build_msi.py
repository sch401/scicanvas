"""Build a per-user Windows MSI from the checked release ZIP with WiX 5."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid
import xml.etree.ElementTree as ET
import zipfile

import third_party

ROOT = Path(__file__).resolve().parents[1]
WIX = "http://wixtoolset.org/schemas/v4/wxs"
ET.register_namespace("", WIX)
NAMESPACE = uuid.UUID("b23a1965-9d4c-45e7-9549-71e074996f32")


def element(parent, name, **attrs):
    return ET.SubElement(parent, f"{{{WIX}}}{name}", attrs)


def identifier(prefix, value):
    return prefix + hashlib.sha256(value.encode("utf-8")).hexdigest()[:24]


def payload(stage, output):
    include = ET.Element(f"{{{WIX}}}Include")
    fragment = element(include, "Fragment")
    local = element(fragment, "StandardDirectory", Id="LocalAppDataFolder")
    programs = element(local, "Directory", Id="UserPrograms", Name="Programs")
    root = element(programs, "Directory", Id="INSTALLFOLDER", Name="SciCanvas")
    menu = element(fragment, "StandardDirectory", Id="ProgramMenuFolder")
    element(menu, "Directory", Id="SciCanvasMenu", Name="SciCanvas")
    directories = {Path("."): ("INSTALLFOLDER", root)}
    for path in sorted(stage.rglob("*")):
        if not path.is_dir():
            continue
        relative = path.relative_to(stage)
        parent = directories[relative.parent][1]
        ident = identifier("D", relative.as_posix())
        directories[relative] = (ident, element(parent, "Directory", Id=ident, Name=path.name))
    group = element(fragment, "ComponentGroup", Id="Payload")
    for path in sorted(stage.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(stage)
        key = relative.as_posix()
        ident = identifier("C", key)
        component = element(group, "Component", Id=ident, Guid=str(uuid.uuid5(NAMESPACE, key)),
                            Directory=directories[relative.parent][0])
        file_id = "MainExe" if key == "SciCanvas.exe" else identifier("F", key)
        element(component, "File", Id=file_id, Source=str(path), Name=path.name)
        element(component, "RegistryValue", Root="HKCU", Key=r"Software\SciCanvas\InstalledFiles",
                Name=ident, Type="integer", Value="1", KeyPath="yes")
        if file_id == "MainExe":
            element(component, "Shortcut", Id="StartMenuShortcut", Directory="SciCanvasMenu",
                    Name="SciCanvas", Target="[#MainExe]", WorkingDirectory="INSTALLFOLDER")
            element(component, "RemoveFolder", Id="RemoveSciCanvasMenu", Directory="SciCanvasMenu", On="uninstall")
            for extension, progid in [("scv", "SciCanvas.Project"), ("scgt", "SciCanvas.GraphTemplate"),
                                      ("scfn", "SciCanvas.FitFunction")]:
                element(component, "RegistryValue", Root="HKCU", Key=rf"Software\Classes\.{extension}",
                        Type="string", Value=progid)
                element(component, "RegistryValue", Root="HKCU", Key=rf"Software\Classes\{progid}\shell\open\command",
                        Type="string", Value='"[#MainExe]" "%1"')
    ET.ElementTree(include).write(output, encoding="utf-8", xml_declaration=True)


def main():
    if sys.platform != "win32":
        raise SystemExit("MSI builds require Windows")
    wix = shutil.which("wix")
    if wix is None:
        raise SystemExit("Install WiX 5.0.2 and add wix to PATH")
    installed = subprocess.check_output([wix, "--version"], text=True).strip()
    if not installed.startswith("5.0.2+"):
        raise SystemExit(f"WiX 5.0.2 required; found {installed}")
    subprocess.run([sys.executable, "tools/package_release.py"], cwd=ROOT, check=True)
    version = third_party.tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    destination = ROOT / "target/releases"
    archive = destination / f"SciCanvas-{version}-windows-x64.zip"
    target = destination / f"SciCanvas-{version}-windows-x64.msi"
    with tempfile.TemporaryDirectory(prefix="msi-", dir=destination) as temp:
        temporary = Path(temp)
        with zipfile.ZipFile(archive) as z:
            z.extractall(temporary)
        stage = temporary / f"SciCanvas-{version}-windows-x64"
        shutil.copytree(ROOT / "samples", stage / "samples")
        generated = temporary / "payload.wxi"
        payload(stage, generated)
        subprocess.run([wix, "build", str(ROOT / "installer/scicanvas.wxs"), "-arch", "x64",
                        "-d", f"Version={version}", "-d", f"Payload={generated}", "-o", str(target)],
                       cwd=ROOT, check=True)
    print(target)
    print("SHA-256: " + hashlib.sha256(target.read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
