#!/usr/bin/env python3
"""Smoke tests opcionais Linux da TUI real. Só utiliza repositórios temporários.
Uso: python3 tools/check_tui.py [target/debug/fp]
Python é necessário apenas para esta verificação de desenvolvimento.
"""
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


def scenario(steps, expected, chapter=False, number=1, tools=None, technology=None, disabled=None):
    with tempfile.TemporaryDirectory(prefix="fp-pty-") as directory:
        root = Path(directory)
        (root / "Justfile").write_text("run phase chapter *args:\n    @exit 0\n")
        (root / "chapters.toml").write_text("[chapters]\n")
        if chapter:
            (root / "fase-01").mkdir()
        tool_path = root / "tools"
        if tools is not None:
            tool_path.mkdir()
            for name, body in tools.items():
                executable = tool_path / name
                executable.write_text("#!/bin/sh\n" + body + "\n")
                executable.chmod(0o755)
        before = sorted(str(p.relative_to(root)) for p in root.rglob("*"))
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        process = subprocess.Popen([str(BINARY), "--root", directory, "create"], stdin=slave, stdout=slave, stderr=slave, env={**os.environ, "TERM": "xterm-256color", "PATH": str(tool_path)}, start_new_session=True)
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
                # Permite ao console concluir a renderização antes de receber teclas.
                time.sleep(0.08)
                os.write(master, keys)
            deadline = time.monotonic() + 8
            while process.poll() is None and time.monotonic() < deadline:
                read()
            assert process.poll() is not None, transcript.decode(errors="replace")
            assert process.returncode == expected, (process.returncode, transcript.decode(errors="replace"))
            read()
            if any(prompt == "Deseja abrir" and keys == b"\x1b[A\r" for prompt, keys in steps):
                assert b"zed" in transcript and b"ausente" in transcript, transcript.decode(errors="replace")
            after = sorted(str(p.relative_to(root)) for p in root.rglob("*"))
            if expected == 130 or any(keys == b"n" for _, keys in steps):
                assert before == after, (before, after)
                assert (root / "chapters.toml").read_text() == "[chapters]\n"
            elif chapter:
                assert (root / f"fase-01/cap-{number:02}").is_dir()
                assert f'[chapters."1.{number}"]' in (root / "chapters.toml").read_text()
                assert f'name = "cap-{number:02}"' in (root / "chapters.toml").read_text()
            else:
                assert (root / f"fase-{number:02}").is_dir()
            if technology:
                assert f"Tecnologia: {technology}".encode() in transcript, transcript.decode(errors="replace")
            for label in disabled or []:
                assert f"{label} (não configurado)".encode() in transcript, transcript.decode(errors="replace")
            if technology == "SQL / Database" and expected == 0:
                assert (root / f"fase-01/cap-{number:02}" / "sql").is_dir()
            assert "Qual é o nome".encode() not in transcript
            assert b"\x1b[?1049h" not in transcript, "Não usar tela alternativa"
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            os.close(master)


kind_phase = ("O que deseja criar", b"\r")
number = ("Número da fase", b"\r")
chapter = [("O que deseja criar", b"\x1b[B\r"), ("Em qual fase", b"\r"), ("Número do capítulo", b"\r"), ("Qual tecnologia", b"\x1b[B\x1b[B\x1b[B\x1b[B\x1b[B\r")]
scenario([("O que deseja criar", b"\x1b")], 130)
scenario([("O que deseja criar", b"\x03")], 130)
scenario([kind_phase, number, ("Deseja continuar", b"n")], 0)
scenario([kind_phase, number, ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0)
scenario(chapter + [("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, chapter=True)
scenario([kind_phase, number, ("Deseja continuar", b"y"), ("Deseja abrir", b"\x1b[A\r")], 0)
scenario([kind_phase, ("Número da fase", b"\x7f3\r"), ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, number=3)
scenario(chapter[:2] + [("Número do capítulo", b"\x7f3\r"), chapter[3], ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, chapter=True, number=3)

# Nenhuma ferramenta: Enter seleciona SQL; as setas não ativam opções indisponíveis.
prefix = chapter[:3]
scenario(prefix + [("Qual tecnologia", b"\x1b[A\x1b[A\r"), ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, chapter=True, technology="SQL / Database", disabled=["Python", "TypeScript / JavaScript", "Rust", "ESP32 / IoT"])
# Cargo e uv disponíveis: descer de Python pula TypeScript sem vp e seleciona Rust.
fakes = {"uv": "exit 99", "cargo": "/bin/mkdir src\nprintf '[package]\\n' > Cargo.toml\nprintf 'fn main() {}\\n' > src/main.rs"}
scenario(prefix + [("Qual tecnologia", b"\x1b[B\r"), ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, chapter=True, tools=fakes, technology="Rust (Cargo)", disabled=["TypeScript / JavaScript", "ESP32 / IoT"])
# vp no PATH reativa TypeScript; inicializador simulado, sem download de templates.
scenario(prefix + [("Qual tecnologia", b"\r"), ("Deseja continuar", b"y"), ("Deseja abrir", b"\r")], 0, chapter=True, tools={"vp": "printf '%s' '{\"scripts\":{\"dev\":\"vite\"}}' > package.json"}, technology="TypeScript / JavaScript (Vite+)", disabled=["Python", "Rust", "ESP32 / IoT"])
scenario(prefix + [("Qual tecnologia", b"\x1b")], 130, chapter=True)
scenario(prefix + [("Qual tecnologia", b"\x03")], 130, chapter=True)
print("13 cenários PTY de create passaram, incluindo tecnologias desabilitadas, navegação e cancelamento.")
