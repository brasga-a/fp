# Workspace acadêmico FIAP

Projetos e materiais organizados por fases e capítulos. Cada projeto mantém os
manifests e lockfiles do seu ecossistema junto ao código. Não há workspace de
pacotes compartilhado criado automaticamente.

```text
AGENTS.md              orientações para agentes
Justfile               interface de execução independente do FP
chapters.toml          registro declarativo dos capítulos e comandos
pyproject.toml         configuração Python da raiz (uv, sem pacote)
scripts/chapters.py    dispatcher usado pelo Just (Python 3.11+)
tests/test_chapters.py  testes do dispatcher em diretórios temporários
docs/README.md         organização dos materiais acadêmicos
fase-NN/cap-NN/        projetos criados conforme necessário
```

## Comandos

```sh
fp init                 # Completar a estrutura e diagnosticar ferramentas
fp init --check         # Diagnosticar sem alterar arquivos
fp create               # Criar fase ou capítulo com nome automático
fp list                 # Listar capítulos e comandos disponíveis
fp run 1 2              # Executar o alvo padrão registrado
fp delete               # Selecionar uma fase ou capítulo para excluir
```

Esses comandos usam `~/fiap` por padrão. Para trabalhar nesta raiz se ela estiver
em outro local, use `fp --root . COMANDO` ou configure `FP_ROOT`.

Sem FP instalado, use o Just a partir desta pasta:

```sh
just help
just list
just run 1 2
just run 1 2 --task dev -- --host 127.0.0.1
just check
```

Just e Python 3.11+ são necessários para o dispatcher. uv, Vite+, Cargo e
PlatformIO são necessários apenas para os projetos correspondentes. Zed é opcional.

## Registro de execução

`fp create` registra comandos quando o inicializador fornece um entrypoint real.
Python usa `main.py` e `uv run main.py`. Pastas e materiais sem entrypoint não
recebem comandos fictícios. Para registrar um projeto já existente, declare seu
caminho e `commands.ALVO.argv` em `chapters.toml`; `default` seleciona o alvo padrão.
Um alvo pode ter `cwd` relativo ao capítulo. Só registre diretórios e arquivos reais.
Os argumentos são repassados sem interpretação pelo shell.

Guarde PDFs e anotações em `docs/`, preservando os originais. Fases e capítulos de
código são criados por demanda; `fp init` não inventa projetos acadêmicos.

## Inicialização segura

`fp init` só cria arquivos ausentes. Configurações existentes, inclusive
`AGENTS.md`, Justfile e o registro, são preservadas. Ele informa conflitos de
caminho/configuração e ferramentas ausentes no PATH, sem usar sudo nem alterar
o PATH, configurações globais, Git ou permissões do Docker.
