"""Install the pinned official Windows audit tool inside ignored target/."""
import hashlib
import io
from pathlib import Path
import tarfile
import urllib.request

VERSION = "0.20.2"
PACKAGE = f"cargo-deny-{VERSION}-x86_64-pc-windows-msvc"
SHA256 = "975a22143262fd27476d19ee00c7af67978426e40e1dee94eed6bbade1cf87dc"


def install():
    directory = Path(__file__).resolve().parents[1] / "target/tools/cargo-deny"
    directory.mkdir(parents=True, exist_ok=True)
    archive = directory / f"{PACKAGE}.tar.gz"
    if not archive.exists():
        url = f"https://github.com/EmbarkStudios/cargo-deny/releases/download/{VERSION}/{PACKAGE}.tar.gz"
        with urllib.request.urlopen(url, timeout=60) as response:
            archive.write_bytes(response.read())
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise ValueError(f"cargo-deny archive checksum mismatch: {archive}")
    executable = directory / "cargo-deny.exe"
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as package:
        with package.extractfile(f"{PACKAGE}/cargo-deny.exe") as binary:
            executable.write_bytes(binary.read())
    return executable


if __name__ == "__main__":
    print(install())
