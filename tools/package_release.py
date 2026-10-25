"""Build a Windows x64 release candidate with dependency notices and licenses."""
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zipfile

import third_party

ROOT = Path(__file__).resolve().parents[1]


def main():
    generated = third_party.outputs()
    for relative, data in generated.items():
        if (ROOT / relative).read_bytes() != data:
            raise SystemExit(f"Attribution out of date: {relative}; run python tools/third_party.py")
    output = subprocess.check_output(
        ["cargo", "build", "--release", "--locked", "-p", "scicanvas", "--message-format=json-render-diagnostics"], cwd=ROOT,
    ).decode("utf-8")
    artifacts = [json.loads(line) for line in output.splitlines() if line.strip()]
    executable = next(Path(a["executable"]) for a in artifacts if a.get("reason") == "compiler-artifact" and a.get("executable") and a["target"]["name"] == "scicanvas")
    binary = executable.read_bytes()
    pe = struct.unpack_from("<I", binary, 0x3c)[0]
    if binary[pe:pe+4] != b"PE\0\0" or struct.unpack_from("<H", binary, pe+4)[0] != 0x8664:
        raise SystemExit("Release artifact must be a Windows x64 PE executable")
    version = third_party.tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    name = f"SciCanvas-{version}-windows-x64"
    destination = ROOT / "target/releases"
    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / f"{name}.zip"
    with tempfile.TemporaryDirectory(prefix="package-", dir=destination) as temporary:
        stage = Path(temporary).resolve() / name
        assert stage.is_relative_to(destination.resolve())
        stage.mkdir()
        (stage / "SciCanvas.exe").write_bytes(binary)
        for relative, data in generated.items():
            path = stage / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        index = json.loads(generated["licenses/THIRD_PARTY_INDEX.json"])
        extras = ["LICENSE-MIT", "LICENSE-APACHE", "docs/COPYRIGHT_AUDIT.md", "licenses/supplemental/sources.json"]
        if index["mpl"]:
            extras.append("MPL_SOURCE.md")
            extras.extend(source["path"] for source in index["mpl"])
        supplements = json.loads((ROOT / "licenses/supplemental/sources.json").read_text(encoding="utf-8"))
        extras += [r["path"] for r in supplements]
        for relative in extras:
            path = stage / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, path)
        sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT).decode().strip())
        rust_docs = sysroot / "share/doc/rust"
        runtime = stage / "licenses/rust-runtime"
        runtime.mkdir()
        shutil.copyfile(rust_docs / "COPYRIGHT-library.html", runtime / "COPYRIGHT-library.html")
        shutil.copytree(rust_docs / "licenses", runtime / "licenses")
        info = dict(
            status="release-candidate", version=version, target=third_party.TARGET,
            git_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip(),
            working_tree_dirty=bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)),
            rustc=subprocess.check_output(["rustc", "-vV"], cwd=ROOT).decode().strip(),
            exe_sha256=hashlib.sha256(binary).hexdigest(),
            cargo_lock_sha256=third_party.sha((ROOT / "Cargo.lock").read_bytes()),
        )
        (stage / "BUILD_INFO.json").write_text(json.dumps(info, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
        readme = [
            "SciCanvas Windows x64 发布候选包\n\n双击 SciCanvas.exe 运行。\n",
            "第三方版权与许可：THIRD_PARTY_NOTICES.md、licenses/\n",
            "代码与素材来源检查、仍需确认的记录：docs/COPYRIGHT_AUDIT.md\n",
            "本程序会读取系统字体，未在包中复制 Windows 字体或系统 DLL。\n",
            "向他人分发时请保留整个包，附带的第三方声明与源码适用于对应组件。\n",
        ]
        if index["mpl"]:
            readme.insert(2, "MPL 对应源码离线获取：MPL_SOURCE.md、licenses/mpl-sources/*.crate\n")
        (stage / "README.txt").write_text("".join(readme), encoding="utf-8")
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    package.write(path, path.relative_to(stage.parent).as_posix())
    print(archive)
    print("SHA-256: " + third_party.sha(archive.read_bytes()))


if __name__ == "__main__":
    main()
