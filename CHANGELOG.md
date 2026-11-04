# Changelog

As mudanças do FP seguem a política de versionamento semântico descrita em
[CONTRIBUTING.md](CONTRIBUTING.md). A versão do executável vem de `Cargo.toml`.

## [Unreleased]

## 0.6.0 — 2026-11-04

### Adicionado

- Implementação nativa Windows x86_64 MSVC nos quatro crates, com descoberta de
  executáveis/PATHEXT, diretório pessoal do perfil e configuração em AppData.
  Suporte completo permanece condicionado à validação nativa Windows.
- Identidade SQLite Windows por volume/ID de 128 bits, preservando registros Unix,
  e credenciais por DPAPI do usuário com aleatoriedade criptográfica do sistema.
- CI nativo Ubuntu/Windows com fmt, check, Clippy, testes, build release e pacotes
  tar.gz/ZIP com checksums, sem publicação em CI comum. Release reutiliza a matriz.
- Testes com executáveis Rust simulados, streams, inicializadores, rollback e
  persistência; testes Windows para batch, DPAPI e falhas Linux-only explícitas.

### Alterado

- Bootstrap Docker rootless/systemd isolado em runtime/linux.rs. Windows init
  prepara o workspace; setup-runtime é recusado com indicação de Docker Desktop.
- Docker Windows utiliza docker.exe e o contexto do usuário, exige Linux
  containers e recusa endpoints Linux incompatíveis sem migrar dados.
- Templates Just usam Python como interpretador direto, com argumentos literais,
  python3 no Linux/python no Windows; dispatcher Windows aguarda o subprocesso.
  Batch com argumentos inseguros é recusado. Templates existentes são preservados.
- Atualizador reconhece ZIP/fp.exe e consulta releases oficiais no Windows.
  Instalação automática Windows permanece recusada, com instruções manuais.

### Corrigido

- Publicação de projetos entre filesystems usa criação exclusiva e rollback por
  identidade, preservando entradas concorrentes. Links externos e tipos especiais
  são recusados e symlinks internos são preservados quando a plataforma permite.
- Locks acadêmicos fecham seus handles antes da remoção no Windows. Operações
  SQLite verificam identidade e não apagam arquivos substituídos ou com hardlinks.
- Segredos ausentes de bancos registrados nunca são regenerados silenciosamente.

## 0.5.0 — 2026-11-04

### Alterado

- Versão mínima do Python elevada de 3.11 para 3.14 no template acadêmico,
  no diagnóstico de `fp init` e no ambiente de CI. Instalações com Python
  anterior a 3.14 precisam ser atualizadas. Manifestos existentes mantêm
  seus requisitos personalizados.

## 0.4.1 — 2026-11-04

### Corrigido

- Manifestos Python novos informam ao Pyright/Basedpyright onde encontrar a
  `.venv` compartilhada, por caminhos relativos à raiz e ao capítulo. Imports
  de dependências como NumPy são resolvidos também ao abrir somente o capítulo
  no Zed. Configurações personalizadas do analisador são preservadas.

## 0.4.0 — 2026-11-04

### Adicionado

- Capítulos Python criados pelo FP entram no workspace uv da raiz e compartilham
  `.venv`, `uv.lock` e dependências. Nomes incluem fase e capítulo para evitar
  colisões entre fases. O novo template declara um workspace vazio.

### Alterado

- Novos alvos Python usam `uv run --all-packages main.py`, incluindo dependências
  da raiz e dos membros. Projetos antigos continuam preservados, sem migração
  automática. Criação não instala pacotes nem cria ambientes ou lockfiles.
- Criação e exclusão atualizam os membros explícitos no manifesto uv com
  preservação de configurações, detecção de edição concorrente e rollback em
  falhas de publicação do registro. Exclusão preserva o ambiente compartilhado.

### Corrigido

- Workflow seleciona Python 3.11 para os testes acadêmicos no Ubuntu 22.04.
- Publicação pode ser retomada manualmente para uma tag existente, sempre
  compilando o conteúdo dessa tag, sem mover tags ou sobrescrever releases.

## 0.3.0 — 2026-11-02

### Adicionado

- `fp init` prepara Docker rootless dedicado, pacotes com SHA-256 fixo, serviço
  systemd de usuário e imagens públicas Oracle/MongoDB sem pedir login.
- `fp init --academic-only`, diagnóstico read-only e `fp db init --setup-runtime`,
  também disponível pela TUI; pré-requisitos administrativos são diagnosticados.
- Senhas aleatórias por recurso em arquivos privados, usadas automaticamente,
  sem valores no registro ou argumentos; variáveis explícitas seguem suportadas.
- Endpoint persistido por recurso rootless, preservando conexões e dados legados.

### Corrigido

- Execução de ferramentas tolera indisponibilidade transitória de executáveis
  recém-escritos (ETXTBSY), sem repetir comandos que já tenham iniciado.

### Alterado

- SQLite embarcado via rusqlite, sem depender da instalação do comando sqlite3.
- Novas instâncias Oracle usam por padrão a imagem pública comunitária
  `gvenzl/oracle-free:23-slim`; imagem e credenciais legadas não são alteradas.

## 0.2.0 — 2026-11-02

Versão preparada localmente; sem tag ou release publicada nesta etapa.

### Alterado

- Repositório convertido em workspace com `fp-cli`, `fp-workspace`, `fp-database`
  e `fp-update`; versão herdada de `workspace.package.version`.
- `fp db` passa a usar núcleo Rust nativo em vez do script externo/FP_DB_BIN.
  `init` agora apenas configura o gerenciador; sintaxe legada recebe orientação.
- Instalação Cargo usa `--path crates/fp-cli`; workflow e validador de releases
  adaptados ao manifesto virtual, preservando o binário e formato dos artefatos.

### Adicionado

- SQLite sem Docker e instâncias Oracle Free/MongoDB com volumes Docker;
  adoção explícita de recursos legados, sem migração automática ou senha padrão.
- Comandos db/database: init, create, run, stop, list, status, logs, exec, delete,
  provision-user e visual; aliases de ações por flags, JSON, no-input e dry-run.
- TUI Cliclack sobre a mesma API, com resumos, cancelamento e confirmação.
- Estado XDG, locks, timeouts, logs, identidade dos recursos e exclusão explícita
  de registro/container/dados, preservando dados importados.

- Distribuição Linux x86_64 GNU em GitHub Releases, com validação de tag,
  testes, empacotamento e SHA-256 antes da publicação.
- `fp update` com confirmação, checksum obrigatório e substituição segura de
  instalações standalone reconhecidas; `fp update --check` somente consulta.
- Seleção exata do artefato por target e validação do formato ELF e da versão
  do binário antes da instalação; rejeição de downloads incompletos e de
  alterações concorrentes na instalação.
- Proteção de instalações Cargo, do sistema ou de origem incerta, com
  atualização direta restrita a instalações standalone declaradas.
- Testes de atualização sem rede, com fixtures e HTTP simulado para falhas de
  download, checksum, permissões, cancelamento e substituição.
- Guia de instalação standalone, integridade e publicação pelo GitHub Actions.
- Rotina no `AGENTS.md` para atualizar este histórico e a versão do CLI em cada
  entrega funcional, mantendo `Cargo.toml`, `Cargo.lock` e changelog consistentes.
  Atualizações locais via Cargo dispensam tags e builds no GitHub Actions;
  mudanças somente de documentação não incrementam a versão do executável.

## [0.1.0] — 2026-11-01

Primeira versão publicada do FP.

### Adicionado

- `fp init`: preparação idempotente da raiz acadêmica, templates embutidos,
  diagnóstico de ferramentas no PATH e modo `--check` sem alterações.
- `fp create`: TUI inline para fases e capítulos com nomes numéricos automáticos,
  inicializadores nativos e registro de entrypoints reais no Just.
- Tecnologias indisponíveis visíveis e desabilitadas com “não configurado”.
- `fp delete`: seleção interativa, confirmação, atualização do registro e
  restauração da pasta quando a publicação do registro falha.
- `fp run`: delegação ao Just com argumentos, terminal e códigos de saída preservados.
- `fp list`: árvore de fases e capítulos, tecnologias e estado executável.
- `fp db`: wrapper do comando db existente, com perfil de execução de usuário.
- Configuração de raiz por `--root`, `FP_ROOT` e padrão `~/fiap`.
- Testes isolados de contratos, bootstrap, exclusão e interação em pseudoterminal.
- Guia de contribuição e instruções AGENTS em inglês, com respostas ao usuário
  em português.

[Unreleased]: https://github.com/brasga-a/fp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/brasga-a/fp/tree/v0.1.0
