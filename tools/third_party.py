"""Generate checked-in Windows third-party notices from Cargo.lock and cached crates.

Run after cargo fetch: python tools/third_party.py [--check]. No network is used.
Includes normal and build dependencies; build tools are retained conservatively.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import struct
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-pc-windows-msvc"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def cargo(*args):
    return subprocess.check_output(["cargo", *args], cwd=ROOT).decode("utf-8-sig")


def font_names(data):
    """Read original copyright/trademark/license name records from SFNT fonts."""
    for i in range(struct.unpack_from(">H", data, 4)[0]):
        tag, _, offset, _ = struct.unpack_from(">4sIII", data, 12 + i * 16)
        if tag != b"name":
            continue
        _, count, strings = struct.unpack_from(">HHH", data, offset)
        records = set()
        for j in range(count):
            platform, _, _, name_id, length, start = struct.unpack_from(">6H", data, offset + 6 + j * 12)
            if name_id in (0, 7, 8, 9, 13, 14):
                value = data[offset + strings + start:offset + strings + start + length]
                records.add((name_id, value.decode("utf-16-be" if platform in (0, 3) else "mac_roman")))
        return sorted(records)
    raise ValueError("Font is missing its name table")


def outputs():
    metadata = json.loads(cargo("metadata", "--format-version", "1", "--locked", "--offline", "--all-features", "--filter-platform", TARGET))
    tree = cargo("tree", "--locked", "--offline", "--target", TARGET, "--all-features", "--edges", "normal,build", "--prefix", "none", "--format", "{p}")
    active = set(re.findall(r"^([\w-]+) v([^\s]+)", tree, re.M))
    packages = sorted((p for p in metadata["packages"] if p["source"] and (p["name"], p["version"]) in active), key=lambda p: (p["name"], p["version"]))
    lock_bytes = (ROOT / "Cargo.lock").read_bytes()
    locked = {(p["name"], p["version"]): p for p in tomllib.loads(lock_bytes.decode())["package"] if "source" in p}
    supplements = json.loads((ROOT / "licenses/supplemental/sources.json").read_text(encoding="utf-8"))
    for record in supplements:
        if sha((ROOT / record["path"]).read_bytes()) != record["sha256"]:
            raise ValueError(f"Supplement checksum mismatch: {record['path']}")
    result, inventory, mpl = {}, [], []
    notices = ["# SciCanvas 第三方版权与许可证", "", "本文件随 Windows x64 发行包一起提供。应用自有部分沿用 MIT OR Apache-2.0；下列第三方代码与字体保留各自许可。", "",
        "清单来自 Cargo.lock 和 Windows normal/build 完整特性依赖树，保守包含构建期工具；不是对每个包都被链接进 exe 的判断。原始版权、许可和 NOTICE 文本保存在对应目录，许可 OR/AND 表达式保留原样。", "",
        "第三方软件包的原始许可和版权声明按 Cargo.lock 一并提供。egui 内嵌字体另保留原始字体许可和字体 name 表声明，包含 Hack 的 Bitstream Vera 声明。", "",
        "Windows 系统字体通过用户电脑读取，本发行包不附加微软雅黑/Arial 字体文件；系统 DLL 同样不随包复制。Rust 标准库版权材料在发行包的 licenses/rust-runtime 中，按实际构建工具链附带。", "",
        "## 包清单", "", "| 包 / 版本 | 声明许可 | 原始声明文件 |", "| --- | --- | --- |"]
    font_lines = ["# SciCanvas / egui 内嵌字体原始声明", "", "以下内容直接读取 epaint_default_fonts 包中四个字体的 name 表，保留版权、商标和许可文字；另附 fonts/*.txt 的原始完整声明。系统字体不在此清单中。", ""]
    for p in packages:
        name, version = p["name"], p["version"]
        key = f"{name}-{version}"
        directory = Path(p["manifest_path"]).parent
        expected = locked[(name, version)]["checksum"]
        archive = directory.parents[2] / "cache" / directory.parent.name / f"{key}.crate"
        archive_data = archive.read_bytes()
        if sha(archive_data) != expected:
            raise ValueError(f"Crate archive checksum mismatch: {key}")
        with tarfile.open(fileobj=io.BytesIO(archive_data), mode="r:gz") as source:
            original = {m.name.removeprefix(key + "/"): source.extractfile(m).read() for m in source.getmembers() if m.isfile()}
        selected = {}
        vcs = json.loads(original.get(".cargo_vcs_info.json", b"{}"))
        revision = vcs.get("git", {}).get("sha1")
        for relative, data in original.items():
            file = Path(relative)
            if re.search(r"license|licence|copying|notice|copyright|unlicense", file.name, re.I) or (name == "epaint_default_fonts" and file.parent.as_posix() == "fonts" and file.suffix == ".txt"):
                if (directory / relative).read_bytes() != data:
                    raise ValueError(f"Modified attribution file: {key}/{relative}")
                selected[relative] = data
        candidates = [r for r in supplements if revision and (
            f"/{revision}/" in r["url"]
            or (r.get("crate") == key and r.get("revision") == revision and r.get("license") == p["license"])
        )]
        # A nested font license does not replace the containing crate's code license.
        if not candidates and not any(re.search(r"license|licence|copying|unlicense", f, re.I) for f in selected):
            raise ValueError(f"Missing original license text: {key}; fetch its exact upstream revision into licenses/supplemental")
        for record in candidates:
            relative = record["path"].split(f"/{record['group']}/", 1)[1]
            selected[f"upstream/{relative}"] = (ROOT / record["path"]).read_bytes()
        # Preserve source-level notices as well, including REUSE file copyrights.
        headers = set()
        for relative, data in original.items():
            if Path(relative).suffix not in (".rs", ".toml", ".c", ".h", ".xml", ".idl"):
                continue
            lines = data.decode("utf-8", errors="replace").splitlines()[:45]
            for i, line in enumerate(lines):
                if re.search(r"copyright|SPDX-FileCopyrightText", line, re.I):
                    # Keep the complete leading notice context, with the source path.
                    headers.add(f"[{relative}]\n" + "\n".join(lines[max(0, i-1):min(len(lines), i+12)]))
        if headers:
            selected["SOURCE_COPYRIGHT_HEADERS.txt"] = ("\n\n".join(sorted(headers)) + "\n").encode()
        files = []
        for relative, data in sorted(selected.items()):
            destination = f"licenses/third-party/{key}/{relative}"
            result[destination] = data
            files.append(dict(path=destination, sha256=sha(data)))
        if not p["license"]:
            raise ValueError(f"Missing declared license: {key}")
        inventory.append(dict(name=name, version=version, license=p["license"], repository=p["repository"], crate_sha256=expected, revision=revision, files=files))
        notices.append(f"| {name} {version} | {p['license']} | " + ", ".join(f"[{Path(f['path']).name}]({f['path']})" for f in files) + " |")
        if "MPL-2.0" in p["license"]:
            # .crate is the original complete published source, verified against Cargo.lock.
            for relative, data in original.items():
                if (directory / relative).read_bytes() != data:
                    raise ValueError(f"MPL source locally modified: {key}/{relative}; distribute the modified source")
            source_path = f"licenses/mpl-sources/{key}.crate"
            result[source_path] = archive_data
            mpl.append(dict(name=name, version=version, path=source_path, sha256=expected, url=f"https://static.crates.io/crates/{name}/{key}.crate"))
        if name == "epaint_default_fonts":
            for relative in sorted(original):
                if relative.endswith(".ttf"):
                    font_lines += [f"## {Path(relative).name}", ""]
                    data = original[relative]
                    if (directory / relative).read_bytes() != data:
                        raise ValueError(f"Modified embedded font: {relative}")
                    records = font_names(data)
                    if not records:
                        font_lines += [f"此字体未提供相关 name 记录，原始版权与许可见 [{key}/fonts](third-party/{key}/fonts/emoji-icon-font-mit-license.txt)。", ""]
                    for id, text in records:
                        font_lines += [f"nameID {id}:", "", "```text", text, "```", ""]
    notices += ["", "## 算法参考与代码来源复核", "", "统计实现中的 AS R94 来源复核见 [版权检查记录](docs/COPYRIGHT_AUDIT.md)。保守保留 [SciPy BSD 声明](licenses/supplemental/scipy-1.17.1/LICENSE.txt)，该声明不代表 SciPy/NumPy 作为运行依赖被打包。", ""]
    notices += ["## 验收数据来源", "", "NIST/ITL StRD 原始认证数据为美国政府作品，仅用于离线测试；来源：[NIST StRD](https://www.itl.nist.gov/div898/strd/)。文件与 SHA-256 / 下载日期见 [非线性清单](crates/scicanvas-core/tests/nist/nls/MANIFEST.txt) 和 [线性清单](crates/scicanvas-core/tests/nist/lls/MANIFEST.txt)，按 supplemental/sources.json 校验原始字节。", ""]
    result["THIRD_PARTY_NOTICES.md"] = ("\n".join(notices)).encode()
    result["licenses/FONT_NOTICES.md"] = ("\n".join(font_lines)).encode()
    index = dict(target=TARGET, scope="normal,build; all workspace features", cargo_lock_sha256=sha(lock_bytes), packages=inventory, mpl=mpl)
    result["licenses/THIRD_PARTY_INDEX.json"] = (json.dumps(index, ensure_ascii=False, indent=2) + "\n").encode()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    for relative, data in generated.items():
        path = ROOT / relative
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f"Attribution out of date: {relative}; run python tools/third_party.py")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print(f"Third-party attribution {'checked' if args.check else 'generated'}: {len(generated)} files")


if __name__ == "__main__":
    main()
