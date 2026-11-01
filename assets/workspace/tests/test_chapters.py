"""Integração do contrato de execução, usando apenas projetos temporários."""

from pathlib import Path
import json
import shutil
import signal
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent


class ChaptersTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="fiap-tests-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "scripts").mkdir()
        shutil.copy(ROOT / "scripts/chapters.py", self.root / "scripts")
        shutil.copy(ROOT / "Justfile", self.root)
        self.project = self.root / "fase-01/cap-02/projeto com espaços"
        self.project.mkdir(parents=True)
        (self.root / "fase-01/cap-03").mkdir()
        (self.project / "probe.py").write_text(
            "import json, os, sys\n"
            "print(json.dumps([os.getcwd(), sys.argv[1:]]))\n"
            "sys.exit(int(os.environ.get('FIAP_TEST_EXIT', '0')))\n"
        )
        self.config = f'''
[chapters."1.2"]
path = "fase-01/cap-02"
default = "main"
[chapters."1.2".commands.main]
argv = [{json.dumps(sys.executable)}, "probe.py"]
cwd = "projeto com espaços"
[chapters."1.2".commands.alternate]
argv = [{json.dumps(sys.executable)}, "probe.py", "alternate"]
cwd = "projeto com espaços"
[chapters."1.3"]
path = "fase-01/cap-03"
'''
        self.write_config(self.config)

    def write_config(self, text):
        (self.root / "chapters.toml").write_text(text)

    def run_dispatcher(self, *args, env=None):
        return subprocess.run(
            [sys.executable, str(self.root / "scripts/chapters.py"), *args],
            cwd="/tmp", capture_output=True, text=True, env=env,
        )

    def test_default_cwd_and_literal_arguments(self):
        args = ["a b", "", "$(touch unexpected)", "'quoted'", "--flag", "ação"]
        result = self.run_dispatcher("run", "01", "02", "--", *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [str(self.project), args])
        self.assertFalse((self.project / "unexpected").exists())

    def test_target_and_passthrough_task_option(self):
        result = self.run_dispatcher("run", "1", "2", "--task", "alternate", "--", "--task", "app")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)[1], ["alternate", "--task", "app"])

    def test_exit_code(self):
        import os
        result = self.run_dispatcher("run", "1", "2", env={**os.environ, "FIAP_TEST_EXIT": "37"})
        self.assertEqual(result.returncode, 37)

    def test_signal_propagation(self):
        (self.project / "probe.py").write_text(
            "import os, signal\nos.kill(os.getpid(), signal.SIGTERM)\n"
        )
        result = self.run_dispatcher("run", "1", "2")
        self.assertEqual(result.returncode, -signal.SIGTERM)

    def test_listing_and_non_executable(self):
        result = self.run_dispatcher("list")
        self.assertEqual(result.returncode, 0, result.stderr)
        for text in ["1.2", "main (padrão)", "alternate", "cwd:", "1.3", "não executável"]:
            self.assertIn(text, result.stdout)
        result = self.run_dispatcher("run", "1", "3")
        self.assertEqual(result.returncode, 2)
        self.assertIn("não executável", result.stderr)

    def test_clear_usage_errors(self):
        cases = [
            (["run", "2", "3"], "Fase 2 inexistente"),
            (["run", "1", "9"], "Capítulo 9 inexistente"),
            (["run", "0", "2"], "positivos"),
            (["run", "x", "2"], "inteiros"),
            (["run", "1", "2", "--task"], "Informe o alvo"),
            (["run", "1", "2", "--task", "missing"], "alvo válido"),
            (["run", "1"], "Uso:"),
            (["unknown"], "Comando inexistente"),
        ]
        for args, message in cases:
            with self.subTest(args=args):
                result = self.run_dispatcher(*args)
                self.assertEqual(result.returncode, 2)
                self.assertIn(message, result.stderr)
                self.assertNotIn("Traceback", result.stderr)

    def test_multiple_targets_require_selection(self):
        self.write_config(self.config.replace('default = "main"', ""))
        self.assertIn("Selecione um alvo", self.run_dispatcher("run", "1", "2").stderr)
        single = self.config.split('[chapters."1.2".commands.alternate]')[0]
        self.write_config(single.replace('default = "main"', ""))
        self.assertEqual(self.run_dispatcher("run", "1", "2").returncode, 0)

    def test_invalid_registry(self):
        configs = [
            "[invalid",
            '[chapters."1.2"]\npath = "missing"',
            '[chapters."1.2"]\npath = "../"',
            self.config.replace('argv = [', 'argv = [0, ', 1),
            self.config.replace('cwd = "projeto com espaços"', 'cwd = "../../.."'),
            self.config.replace('default = "main"', 'default = ["main"]'),
        ]
        for config in configs:
            with self.subTest(config=config):
                self.write_config(config)
                result = self.run_dispatcher("list")
                self.assertEqual(result.returncode, 2)
                self.assertIn("Erro:", result.stderr)
                self.assertNotIn("Traceback", result.stderr)

    def test_empty_registry_and_help(self):
        self.write_config("[chapters]\n")
        self.assertIn("Nenhum capítulo", self.run_dispatcher("list").stdout)
        self.assertIn("just run FASE CAPITULO", self.run_dispatcher("help").stdout)

    def test_missing_tool(self):
        self.write_config(self.config.replace(json.dumps(sys.executable), '"fiap-tool-that-does-not-exist"'))
        result = self.run_dispatcher("run", "1", "2")
        self.assertEqual(result.returncode, 127)
        self.assertIn("Instale a ferramenta", result.stderr)

    @unittest.skipUnless(shutil.which("just"), "Just não instalado neste ambiente")
    def test_just_end_to_end(self):
        import os
        args = ["with spaces", "", "$(touch unexpected)", "--flag"]
        command = ["just", "--justfile", str(self.root / "Justfile"), "run", "1", "2", "--", *args]
        result = subprocess.run(command, cwd="/tmp", capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), [str(self.project), args])
        result = subprocess.run(command, capture_output=True, env={**os.environ, "FIAP_TEST_EXIT": "37"})
        self.assertEqual(result.returncode, 37)


if __name__ == "__main__":
    unittest.main()
