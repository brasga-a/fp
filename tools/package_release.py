#!/usr/bin/env python3
"""Empacota o executável nativo validado. Não publica nem altera tags/releases."""
import hashlib
from pathlib import Path
import subprocess
import sys
import tarfile
import tomllib
import zipfile

from check_release import validate


def package(target, root=Path('.')):
    with (root / 'Cargo.toml').open('rb') as source:
        version = tomllib.load(source)['workspace']['package']['version']
    validate(f'v{version}', version)
    names = {
        'x86_64-unknown-linux-gnu': ('fp', 'tar.gz'),
        'x86_64-pc-windows-msvc': ('fp.exe', 'zip'),
    }
    executable, extension = names[target]
    binary = (root / 'target' / target / 'release' / executable).resolve()
    actual = subprocess.check_output([str(binary), '--version'], text=True).strip()
    if actual != f'fp {version}':
        raise ValueError(f'Versão compilada incorreta: {actual}')
    subprocess.run([str(binary), '--help'], check=True, capture_output=True)
    output = root / 'dist'
    output.mkdir(exist_ok=True)
    archive = output / f'fp-v{version}-{target}.{extension}'
    if extension == 'zip':
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as bundle:
            bundle.write(binary, executable)
    else:
        with tarfile.open(archive, 'w:gz') as bundle:
            bundle.add(binary, arcname=executable)
    with archive.open('rb') as source:
        digest = hashlib.file_digest(source, 'sha256').hexdigest()
    (output / f'SHA256SUMS-{target}').write_text(f'{digest}  {archive.name}\n', encoding='utf-8', newline='\n')
    print(f'{archive}: SHA-256 {digest}')
    return archive


if __name__ == '__main__':
    package(sys.argv[1])
