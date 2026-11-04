# Workspace acadêmico FIAP

Projetos e materiais organizados por fases e capítulos. Capítulos Python criados
pelo FP são membros do workspace uv da raiz e compartilham `.venv` e `uv.lock`.
Outras tecnologias mantêm os manifests e lockfiles junto ao código.

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
Python usa `main.py` e `uv run --all-packages main.py`. Pastas e materiais sem entrypoint não
recebem comandos fictícios. Para registrar um projeto já existente, declare seu
caminho e `commands.ALVO.argv` em `chapters.toml`; `default` seleciona o alvo padrão.
Um alvo pode ter `cwd` relativo ao capítulo. Só registre diretórios e arquivos reais.
Os argumentos são repassados sem interpretação pelo shell.

## Dependências Python compartilhadas

Na raiz, adicione bibliotecas com `uv add numpy`, substituindo `numpy` pela
biblioteca desejada. De qualquer pasta, use `uv add --project ~/fiap numpy`,
adaptando o caminho à raiz configurada. `fp run FASE CAPITULO` inclui as
dependências da raiz e de todos os membros. Para executar diretamente dentro de
um capítulo, use `uv run --all-packages main.py`; para sincronizar tudo a partir
da raiz, `uv sync --all-packages`.

O FP registra cada novo capítulo Python em `tool.uv.workspace.members`, com
nome único como `fase-01-cap-01`. Não instala dependências durante a criação.
O uv cria ou atualiza `.venv` e `uv.lock` na raiz quando necessário. Requisitos
de versões devem ser compatíveis entre os capítulos. Projetos antigos não são
migrados automaticamente. Ao excluir um capítulo ou fase, o FP remove seus
membros explícitos do manifesto, preservando as dependências compartilhadas.

Os manifestos também indicam a `.venv` da raiz ao Pyright/Basedpyright por
caminhos relativos, permitindo abrir só um capítulo no Zed sem perder a
resolução de imports. Após migrar um projeto já aberto, reinicie o servidor
de linguagem (`editor: restart language server` no Zed). Configurações
personalizadas do analisador são preservadas.

Guarde PDFs e anotações em `docs/`, preservando os originais. Fases e capítulos de
código são criados por demanda; `fp init` não inventa projetos acadêmicos.

## Inicialização segura

`fp init` só cria arquivos ausentes. Configurações existentes, inclusive
`AGENTS.md`, Justfile e o registro, são preservadas. Ele informa conflitos de
caminho/configuração e ferramentas ausentes no PATH, sem usar sudo nem alterar
o PATH, configurações globais, Git ou permissões do Docker.
