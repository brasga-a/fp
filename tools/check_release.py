#!/usr/bin/env python3
"""Valida a tag estável contra a única fonte de versão, sem alterar arquivos."""
import re
import json
import subprocess
import sys
from pathlib import Path


def validate(tag, version):
    if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag):
        raise ValueError("A tag deve ter o formato estável vX.Y.Z")
    if tag != f"v{version}":
        raise ValueError(f"Tag {tag} difere de Cargo.toml ({version})")


if __name__ == "__main__":
    try:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version=1", "--locked"], text=True))
        manifest = str(Path("crates/fp-cli/Cargo.toml").resolve())
        version = next(p["version"] for p in metadata["packages"] if p["manifest_path"] == manifest)
        validate(sys.argv[1], version)
    except (ValueError, IndexError) as error:
        sys.exit(str(error))
    print(f"Versão validada: {version}")
