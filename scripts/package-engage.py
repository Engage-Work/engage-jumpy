#!/usr/bin/env python3
"""Package a native build in Engage's existing managed-app layout."""
import hashlib
import platform
import shutil
import tarfile
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parent.parent
version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
arch = {"arm64": "aarch64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
system = {"Darwin": "apple-darwin", "Linux": "unknown-linux-gnu", "Windows": "pc-windows-msvc"}[platform.system()]
binary = "jumpy.exe" if platform.system() == "Windows" else "jumpy"
output = root / "target" / "engage-release"
output.mkdir(parents=True, exist_ok=True)
stage = output / "jumpy-v0.12.2"
if stage.exists():
    shutil.rmtree(stage)
stage.mkdir()
shutil.copy2(root / "target" / "release" / binary, stage / binary)
for directory in ("assets", "licenses"):
    shutil.copytree(root / directory, stage / directory)
for name in ("LICENSE", "CREDITS.md", "README.md"):
    shutil.copy2(root / name, stage / name)
name = f"jumpy-v{version}-{arch}-{system}"
if platform.system() == "Windows":
    archive = Path(shutil.make_archive(str(output / name), "zip", output, stage.name))
else:
    archive = output / f"{name}.tar.gz"
    with tarfile.open(archive, "w:gz") as bundle:
        bundle.add(stage, arcname=stage.name)
digest = hashlib.file_digest(archive.open("rb"), "sha256").hexdigest()
archive.with_name(archive.name + ".sha256").write_text(digest + "\n")
print(f"{archive}\nSHA-256: {digest}\nBytes: {archive.stat().st_size}")
