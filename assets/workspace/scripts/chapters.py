"""Orquestra comandos explícitos; não detecta linguagens nem gerencia builds."""

from pathlib import Path
import os
import re
import shlex
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent
HELP = """FIAP Academic Monorepo

  just help
  just list
  just run FASE CAPITULO [--task ALVO] [--] [ARGUMENTOS...]
  just check

Exemplos (após registrar capítulos reais em chapters.toml):
  just run 1 2
  just run 2 3 --task dev -- --host 127.0.0.1

Sem --task, usa o alvo default ou o único comando registrado.
Use -- antes dos argumentos do projeto quando eles começarem com --task.
Os argumentos são repassados literalmente, sem interpretação pelo shell.
Capítulos sem commands são listados como não executáveis.
Comandos de hardware só são executados quando solicitados explicitamente.
"""


class ConfigError(ValueError):
    pass


def directory(base, value):
    if not isinstance(value, str) or not value:
        raise ConfigError("O diretório deve ser uma string não vazia.")
    path = (base / value).resolve()
    if not path.is_relative_to(base.resolve()) or not path.is_dir():
        raise ConfigError(f"Diretório inexistente ou fora de {base}: {value}")
    return path


def registry(root):
    with (root / "chapters.toml").open("rb") as source:
        data = tomllib.load(source)
    chapters = data.get("chapters")
    if not isinstance(chapters, dict):
        raise ConfigError("chapters.toml deve conter a tabela [chapters].")
    for key, chapter in chapters.items():
        if not re.fullmatch(r"[1-9][0-9]*\.[1-9][0-9]*", key):
            raise ConfigError(f"Identificador inválido: {key}; use FASE.CAPITULO, como 1.2.")
        if not isinstance(chapter, dict):
            raise ConfigError(f"Capítulo inválido: {key}")
        path = directory(root, chapter.get("path"))
        commands = chapter.get("commands", {})
        if not isinstance(commands, dict):
            raise ConfigError(f"commands deve ser uma tabela: {key}")
        for name, command in commands.items():
            if not isinstance(command, dict):
                raise ConfigError(f"Comando inválido: {key}/{name}")
            argv = command.get("argv")
            if not isinstance(argv, list) or not argv or not all(
                isinstance(arg, str) and "\0" not in arg for arg in argv
            ) or not argv[0]:
                raise ConfigError(f"argv deve ser uma lista não vazia de strings: {key}/{name}")
            directory(path, command.get("cwd", "."))
        if "default" in chapter and (
            not isinstance(chapter["default"], str) or chapter["default"] not in commands
        ):
            raise ConfigError(f"Alvo default inexistente: {key}/{chapter['default']}")
    return chapters


def main(args=None, root=ROOT):
    args = list(sys.argv[1:] if args is None else args)
    try:
        if not args or args == ["help"]:
            print(HELP)
            return 0
        if args[0] not in {"list", "run"}:
            raise ConfigError(f"Comando inexistente: {args[0]}. Use just help.")
        chapters = registry(root)
        if args == ["list"]:
            if not chapters:
                print("Nenhum capítulo registrado em chapters.toml.")
            for key in sorted(chapters, key=lambda key: tuple(map(int, key.split(".")))):
                chapter = chapters[key]
                print(f"{key} — {chapter['path']}")
                commands = chapter.get("commands", {})
                if not commands:
                    print("  não executável (sem entrypoint registrado)")
                for name, command in commands.items():
                    default = chapter.get("default", next(iter(commands)) if len(commands) == 1 else None)
                    suffix = " (padrão)" if name == default else ""
                    print(f"  {name}{suffix}: {shlex.join(command['argv'])}")
                    print(f"    cwd: {chapter['path']}/{command.get('cwd', '.')}")
            return 0
        if len(args) < 3 or args[0] != "run":
            raise ConfigError("Uso: just run FASE CAPITULO [--task ALVO] [--] [ARGUMENTOS...]")
        try:
            phase, number = (int(arg) for arg in args[1:3])
        except ValueError:
            raise ConfigError("Fase e capítulo devem ser números inteiros positivos.") from None
        if phase <= 0 or number <= 0:
            raise ConfigError("Fase e capítulo devem ser números inteiros positivos.")
        if not any(key.split(".")[0] == str(phase) for key in chapters):
            raise ConfigError(f"Fase {phase} inexistente. Use just list.")
        key = f"{phase}.{number}"
        if key not in chapters:
            raise ConfigError(f"Capítulo {number} inexistente na fase {phase}. Use just list.")
        chapter = chapters[key]
        commands = chapter.get("commands", {})
        if not commands:
            raise ConfigError(f"Capítulo {key} não executável: sem entrypoint registrado.")
        extra = args[3:]
        target = chapter.get("default", next(iter(commands)) if len(commands) == 1 else None)
        if extra[:1] == ["--task"]:
            if len(extra) < 2:
                raise ConfigError("Informe o alvo após --task.")
            target, extra = extra[1], extra[2:]
        if target not in commands:
            raise ConfigError(f"Selecione um alvo válido para {key}: {', '.join(commands)}")
        if extra[:1] == ["--"]:
            extra = extra[1:]
        command = commands[target]
        cwd = directory(directory(root, chapter["path"]), command.get("cwd", "."))
        os.chdir(cwd)
        # Substituir o dispatcher preserva exit status, sinais e terminal interativo.
        os.execvp(command["argv"][0], command["argv"] + extra)
    except (ConfigError, OSError, tomllib.TOMLDecodeError) as error:
        print(f"Erro: {error}", file=sys.stderr)
        missing_tool = isinstance(error, FileNotFoundError) and error.filename != str(root / "chapters.toml")
        if missing_tool:
            print("Instale a ferramenta indicada pelo comando registrado.", file=sys.stderr)
        return 127 if missing_tool else 2


if __name__ == "__main__":
    sys.exit(main())
