#!/usr/bin/env python3
"""Casos de validação de tags; nenhum acesso ao GitHub."""
import unittest
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


if __name__ == "__main__":
    unittest.main()
