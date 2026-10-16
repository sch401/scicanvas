"""Run the same required Rust checks locally as Windows CI."""
import pathlib
import subprocess


def main():
    root = pathlib.Path(__file__).resolve().parents[1]
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
