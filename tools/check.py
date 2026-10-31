"""运行与 Windows CI 对应的完整检查。"""
import pathlib
import subprocess
import sys
from install_cargo_deny import install


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
    subprocess.run([str(install()), "--locked", "check", "--hide-inclusion-graph"], cwd=root, check=True)
    subprocess.run([sys.executable, "tools/third_party.py", "--check"], cwd=root, check=True)
    subprocess.run([sys.executable, "-m", "unittest", "discover", "-s", "tools", "-p", "test_*.py"], cwd=root, check=True)
    subprocess.run([sys.executable, "tools/sync_docs.py", "--check"], cwd=root, check=True)
    subprocess.run([sys.executable, "-m", "unittest", "discover", "-s", "docs/parity", "-p", "test_*.py"], cwd=root, check=True)
    subprocess.run(["node", "--test", "docs/parity/parity.test.cjs"], cwd=root, check=True)
    checks = [
        ["fmt", "--all", "--", "--check"],
        ["test", "--profile", "ci", "--workspace", "--all-features", "--locked"],
        ["test", "--profile", "ci", "-p", "scicanvas-core", "--no-default-features", "--locked"],
        ["clippy", "--profile", "ci", "--workspace", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"],
    ]
    for args in checks:
        subprocess.run(["cargo", *args], cwd=root, check=True)


if __name__ == "__main__":
    main()
