#!/usr/bin/env python3
"""Casos de validação de tags; nenhum acesso ao GitHub."""
import unittest
from unittest.mock import patch
from pathlib import Path
import hashlib
import tarfile
import tempfile
import zipfile
from package_release import package
from check_release import validate


class ReleaseTests(unittest.TestCase):
    def test_matching_version(self):
        validate("v0.1.0", "0.1.0")
        validate("v12.3.45", "12.3.45")

    def test_mismatch(self):
        with self.assertRaises(ValueError):
            validate("v0.2.0", "0.1.0")

    def test_invalid_or_unstable_tag(self):
        for tag in ["0.1.0", "v01.1.0", "v1.0", "v1.0.0-rc.1", "v1.0.0+build", "v1.0.0\n", "v1.0.0;echo bad"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                validate(tag, "1.0.0")


class PackageTests(unittest.TestCase):
    @patch('package_release.subprocess.run')
    @patch('package_release.subprocess.check_output', return_value='fp 1.2.3\n')
    def test_native_archives_and_checksums(self, version, help_command):
        for target, executable in [('x86_64-unknown-linux-gnu', 'fp'), ('x86_64-pc-windows-msvc', 'fp.exe')]:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary)
                (root/'Cargo.toml').write_text('[workspace.package]\nversion="1.2.3"\n')
                binary=root/'target'/target/'release'/executable
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b'native fixture')
                archive=package(target,root)
                if executable.endswith('.exe'):
                    with zipfile.ZipFile(archive) as bundle:
                        self.assertEqual(bundle.namelist(), ['fp.exe'])
                        self.assertEqual(bundle.read('fp.exe'), b'native fixture')
                else:
                    with tarfile.open(archive) as bundle:
                        self.assertEqual(bundle.getnames(), ['fp'])
                with archive.open('rb') as source:
                    digest=hashlib.file_digest(source,'sha256').hexdigest()
                self.assertEqual((root/'dist'/f'SHA256SUMS-{target}').read_bytes(), f'{digest}  {archive.name}\n'.encode('utf-8'))
                self.assertEqual(help_command.call_args.args[0], [str(binary.resolve()), '--help'])

    @patch('package_release.subprocess.check_output', return_value='fp 9.9.9\n')
    def test_wrong_compiled_version_is_refused_before_packaging(self, version):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            (root/'Cargo.toml').write_text('[workspace.package]\nversion="1.2.3"\n')
            with self.assertRaises(ValueError):
                package('x86_64-pc-windows-msvc',root)
            self.assertFalse((root/'dist').exists())


if __name__ == "__main__":
    unittest.main()
