"""复用清单生成器，并按需将路线和清单导出到指定目录。"""
import argparse
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
COPIES = (
    ("docs/ROADMAP.md", "ROADMAP.md"),
    ("docs/ROADMAP.md", "SciCanvas 技术路线（第二阶段）.md"),
    ("docs/SciCanvas 功能完备性分析与技术路线.md", "SciCanvas 功能完备性分析与技术路线.md"),
    ("docs/parity/parity.html", "SciCanvas 对照清单.html"),
)


def sync(check=False, desktop_dir=None):
    command = [sys.executable, str(ROOT / "docs/parity/build_parity.py")]
    if check:
        command.append("--check")
    subprocess.run(command, cwd=ROOT, check=True)
    if desktop_dir is None:
        return
    destination = Path(desktop_dir).resolve()
    if not check:
        destination.mkdir(parents=True, exist_ok=True)
    for source, name in COPIES:
        target = destination / name
        if check:
            if not target.is_file() or target.read_bytes() != (ROOT / source).read_bytes():
                raise SystemExit(f"文档未同步：{target}")
        else:
            shutil.copyfile(ROOT / source, target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="只检查，不写入文档或导出目录")
    parser.add_argument("--desktop-dir", type=Path, help="额外导出两份技术路线及网页清单的目录")
    args = parser.parse_args()
    sync(args.check, args.desktop_dir)


if __name__ == "__main__":
    main()
