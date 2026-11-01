#!/usr/bin/env python3
"""Smoke tests da exclusão, exclusivamente em repositórios temporários."""
import errno
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

BINARY = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/fp").resolve()


def snapshot(root):
    return {str(p.relative_to(root)): p.read_bytes() if p.is_file() else None for p in root.rglob("*")}


def scenario(steps, expected, deleted=None):
    with tempfile.TemporaryDirectory(prefix="fp-delete-pty-") as directory:
        root = Path(directory)
        (root / "Justfile").write_text("# preserve\n")
        (root / "chapters.toml").write_text('[chapters."1.1"]\npath="fase-01/cap-01"\n[chapters."1.2"]\npath="fase-01/cap-02"\n[chapters."2.1"]\npath="fase-02/cap-01"\n')
        for path in ["fase-01/cap-01", "fase-01/cap-02", "fase-02/cap-01", "docs/fase-01"]:
            (root / path).mkdir(parents=True)
            (root / path / "original").write_text("keep")
        before = snapshot(root)
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        process = subprocess.Popen([str(BINARY), "--root", directory, "delete"], stdin=slave, stdout=slave, stderr=slave, start_new_session=True, env={**os.environ, "TERM": "xterm-256color"})
        os.close(slave)
        transcript = bytearray()
        pending = bytearray()

        def read():
            if select.select([master], [], [], 0.1)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        return
                    raise
                transcript.extend(data)
                pending.extend(data)

        try:
            for prompt, keys in steps:
                deadline = time.monotonic() + 8
                while prompt.encode() not in pending:
                    read()
                    if time.monotonic() > deadline or process.poll() is not None:
                        raise AssertionError(f"Prompt ausente: {prompt}\n{transcript.decode(errors='replace')}")
                pending.clear()
                time.sleep(0.08)
                os.write(master, keys)
            deadline = time.monotonic() + 8
            while process.poll() is None and time.monotonic() < deadline:
                read()
            assert process.poll() is not None, transcript.decode(errors="replace")
            assert process.returncode == expected, transcript.decode(errors="replace")
            read()
            if deleted is None:
                assert snapshot(root) == before
            else:
                assert not (root / deleted).exists()
                assert (root / "fase-02/cap-01/original").read_text() == "keep"
                assert (root / "docs/fase-01/original").read_text() == "keep"
                text = (root / "chapters.toml").read_text()
                assert 'chapters."1.1"' not in text
                if deleted == "fase-01":
                    assert 'chapters."1.2"' not in text
                else:
                    assert (root / "fase-01/cap-02/original").is_file()
                    assert 'chapters."1.2"' in text
            assert b"\x1b[?1049h" not in transcript
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            os.close(master)


chapter = [("O que deseja excluir", b"\r"), ("Qual fase", b"\r"), ("Qual capítulo", b"\r")]
scenario([("O que deseja excluir", b"\x1b")], 130)
scenario(chapter + [("Excluir permanentemente", b"\x03")], 130)
scenario(chapter + [("Excluir permanentemente", b"\r")], 0)
scenario(chapter + [("Excluir permanentemente", b"y")], 0, "fase-01/cap-01")
scenario([("O que deseja excluir", b"\x1b[B\r"), ("Qual fase", b"\r"), ("Excluir permanentemente", b"y")], 0, "fase-01")
print("5 cenários PTY de exclusão passaram: Esc, Ctrl+C, Não padrão, capítulo e fase.")
