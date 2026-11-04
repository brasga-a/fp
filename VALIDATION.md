# Validação — 2026-11-04 (FP 0.5.0)

O template acadêmico e o diagnóstico de `fp init` exigem Python 3.14+.
O workflow de release usa Python 3.14. O teste de integração confirma que
um workspace novo passa `--python >=3.14` ao uv; o teste com manifesto
personalizado continua usando seu requisito anterior sem sobrescrevê-lo.
O diagnóstico rejeita Python 3.10 a 3.13 e aceita 3.14 e versões posteriores
da série 3 nos testes com executáveis simulados.

- `cargo fmt --all --check` e Clippy com `-D warnings`: passaram.
- `cargo test --workspace --locked --offline`: 98 testes Rust e 11 testes
  Python do dispatcher passaram.
- `cargo build --release --locked --offline`: passou; `target/release/fp
  --version` informou `fp 0.5.0`.
- Verificações PTY: 13 cenários de criação, 5 de exclusão e 5 de bancos passaram.
- `python3 tools/test_release.py`: 3 testes passaram; `check_release.py v0.5.0`
  confirmou a versão.

Validação local com Python 3.14.8, diretórios temporários e inicializadores
simulados. A execução remota no GitHub Actions não foi realizada nesta etapa.

# Validação — 2026-11-04 (FP 0.4.1)

Reproduzido o diagnóstico `Import "numpy" could not be resolved` usando o
Basedpyright 1.40.2 instalado pelo Zed, com apenas a pasta do capítulo como raiz.
O NumPy já estava instalado na `.venv` compartilhada; faltava indicar esse
ambiente ao analisador. A raiz e os capítulos novos recebem `tool.pyright`
com `venvPath` relativo e `venv = ".venv"`, preservando configurações existentes.

- `cargo fmt --all --check`, Clippy com `-D warnings`, 97 testes Rust e 11 testes
  Python do dispatcher passaram, com Cargo offline e lockfile preservado.
- Build release validado; instalação Cargo mantém o prefixo `~/.local`.
- Após o ajuste local autorizado nos dois manifestos, o mesmo Basedpyright
  analisou o `main.py` existente pela raiz e pela pasta do capítulo: zero erros,
  zero avisos e nenhuma regra de diagnóstico desabilitada.
- Não houve alteração no código do exercício ou nas configurações globais do
  editor. Os manifestos anteriores foram preservados em backup no estado do FP.

O editor aberto pode precisar reiniciar o servidor de linguagem para descartar
o ambiente anterior. No Zed, a ação é `editor: restart language server`.

# Validação — 2026-11-04 (FP 0.4.0)

Capítulos Python passam a ser membros explícitos do workspace uv da raiz. A
criação usa nomes distintos por fase; os comandos incluem `--all-packages` para
disponibilizar as dependências da raiz e dos membros no ambiente compartilhado.

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --all --check` | Passou |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passou |
| `cargo test --workspace --locked --offline` | 97 testes Rust passaram; nenhum ignorado; dispatcher gerado passou 11 testes Python |
| `cargo build --release --locked --offline` | Passou |
| `python3 tools/check_tui.py target/release/fp` | 13 cenários PTY passaram |
| `python3 tools/check_delete_tui.py target/release/fp` | 5 cenários PTY passaram |
| `python3 tools/check_database_tui.py target/release/fp` | 5 cenários PTY passaram |
| `target/release/fp --version` e `fp --version` no PATH após instalação Cargo | `fp 0.4.0` |

Os testes usam diretórios temporários e inicializadores simulados. Cobrem nomes
de capítulos repetidos entre fases, preservação de dependências e comentários,
registro de membros Python, cancelamento, configurações inválidas, links,
remoção de membros ao excluir capítulos/fases e rollback em falhas ou edições
concorrentes. Não inicializam projetos reais nem acessam o repositório acadêmico.

Após os testes, a migração explicitamente solicitada foi aplicada ao repositório
acadêmico existente. A sincronização uv offline incluiu raiz e capítulo; a
verificação dentro do capítulo confirmou `sys.prefix` em `~/fiap/.venv`, importou
NumPy 2.5.4 e executou `fp run 3 3` com sucesso. O hash de `main.py` permaneceu
idêntico. Configurações, lockfiles e o ambiente antigo do capítulo foram
preservados em backup no diretório de estado do FP. Isso foi uma migração local
autorizada, separada da suíte automatizada.

Projetos antigos não são migrados automaticamente pelo CLI. Workspaces com
exclusões personalizadas precisam ser revisados antes de criar novos membros
pelo FP. O uv resolve versões em conjunto; dependências incompatíveis entre
capítulos exigem ajustes nos requisitos.

# Validação — 2026-11-01

Ambiente: Linux x86_64, Rust stable 1.99.1. Como Rust não estava instalado,
o toolchain foi obtido do distribuidor oficial e isolado em `/tmp/fp-toolchain`.
O PATH e os atalhos do desktop foram preservados. Na integração posterior do db,
o grupo Docker e o serviço foram configurados com autorização explícita.

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` | Passou |
| `cargo clippy --all-targets -- -D warnings` | Passou |
| `cargo test --locked` | 29 testes passaram; nenhum ignorado |
| `cargo build --release --locked` | Passou |
| `fp --help` | Passou |
| `fp list` em raiz temporária | Árvore, Python e estado executável corretos |
| `fp run` com Just simulado | Cwd, argumentos e código 37 preservados |
| Just real + cópia do dispatcher acadêmico | Cwd, argumentos literais, `-- --task app` e código 37 corretos |
| Falhas de ferramenta e publicação | Destino novo revertido; configurações existentes preservadas |
| Raiz ausente, TOML inválido e argumentos inválidos | Erros claros, código 2 |
| `fp init` sem terminal | Erro explicando necessidade de terminal interativo |
| `python3 tools/check_tui.py target/release/fp` | 6 cenários PTY passaram |
| Instalação Cargo em `~/.local/bin/fp` | Passou |
| Preservação acadêmica | Conjunto de caminhos e hashes SHA-256 dos 14 arquivos acompanhados idênticos ao início |

Os cenários PTY verificam Esc, Ctrl+C, recusa da confirmação, criação de fase,
criação de capítulo com setas e Zed ausente. Verificam também ausência de tela
alternativa e ausência de alterações nos cancelamentos anteriores à confirmação.
Todos usam somente repositórios temporários.

Os 29 testes Cargo cobrem parsing, configuração de raiz, precedência, Unicode,
numeração, convenções, conflitos, cancelamento, registro, rollback, lock,
comandos nativos simulados, detecção de tecnologias, Just, argumentos, sinais
e ferramentas ausentes. Não inicializam projetos reais.

O ensaio com o dispatcher original copiou `Justfile` e `scripts/chapters.py` para
uma raiz temporária com um programa de teste explícito. O original não foi
executado com projetos reais nem modificado. `.venv` e `__pycache__` foram excluídos
da comparação de conteúdo; nenhuma rotina de teste aponta seus destinos para eles.

## Artefato

- Executável: `target/release/fp`.
- Tamanho: **1.612.544 bytes (aproximadamente 1,59 MiB)**.
- Perfil: otimização `s`, LTO thin, símbolos removidos.
- Bibliotecas dinâmicas de sistema: libc, libm, libgcc_s e loader Linux.
- Não depende de Python, Node, shell interativo ou runtime assíncrono para iniciar.

## Reproduzir neste ambiente

O toolchain temporário permanece disponível nesta sessão:

```sh
env RUSTUP_HOME=/tmp/fp-toolchain/rustup CARGO_HOME=/tmp/fp-toolchain/cargo \
  /tmp/fp-toolchain/cargo/bin/cargo test --manifest-path ~/Work/fp/Cargo.toml --locked --offline
~/Work/fp/target/release/fp --help
```

A instalação de teste está em `/tmp/fp-install/bin/fp`. Esses caminhos em `/tmp`
não são uma instalação persistente. Para instalar permanentemente, use um
Rust/Cargo stable no PATH e `cargo install --path ~/Work/fp --locked`.

## Limites da validação

uv e Just foram inspecionados no ambiente. Vite+, PlatformIO e Zed estavam
ausentes; os contratos dessas ferramentas foram exercitados com executáveis
simulados. Não foram baixados templates nem executados inicializadores reais
durante os testes. O FP não implementa recuperação automática após interrupção
brusca nem isolamento dos efeitos externos de inicializadores (como caches).
As limitações e melhorias possíveis estão detalhadas no README.

Referências técnicas consultadas: [API Cliclack](https://docs.rs/cliclack/0.5.7/cliclack/)
e [guia oficial de criação Vite+](https://github.com/voidzero-dev/vite-plus/blob/main/docs/guide/create.md).
O contrato acadêmico foi verificado diretamente em `~/fiap/AGENTS.md`, README,
Justfile, `chapters.toml`, `pyproject.toml`, dispatcher e seus testes.

## Integração db

- `fp db` delega ao script original e define `DB_EXECUTION_PROFILE=user`.
- `db` não executa sudo automaticamente; falhas de Docker encerram com mensagem.
- 29 testes Cargo, 6 testes Python isolados do db, 6 cenários PTY e sintaxe Bash passaram.
- O setup autorizado adicionou o usuário ao grupo `docker` e habilitou/iniciou
  `docker.service` por autenticação gráfica única. Sem alteração de sudoers.
- Serviço verificado como active/enabled; versão do daemon consultada sem sudo.
- `newgrp docker -c '/home/brasga/.local/bin/fp db list'` passou: listou SQLite
  existente e nenhum container ativo, sem iniciar bancos ou baixar imagens.
- A sessão atual precisa carregar o grupo com novo login ou `newgrp docker`.
- Binário atualizado instalado em `~/.local/bin/fp` via Cargo, como usuário comum.
- Os 14 hashes acadêmicos originais continuam idênticos.

## Python com main automático

O template usa `uv init --app --no-package`, registra `default = "main"` e
`argv = ["uv", "run", "main.py"]`. A opção `--no-package` foi confirmada com o
uv 0.12.21 instalado: sem ela, essa versão cria uma aplicação empacotada em `src/`.
A criação é revertida se o inicializador não produzir `main.py`.

Passaram 29 testes Cargo, formatação, Clippy com warnings tratados e build release.
O template nativo foi conferido manualmente em pasta temporária; `fp run 3 3`
foi validado com Just e uv reais em uma cópia temporária do capítulo e dispatcher.
Nenhum teste automatizado modificou o repositório acadêmico.

Por solicitação do usuário, o capítulo existente `fase-03/cap-03` recebeu o
`main.py` ausente e seu registro de execução. O manifest foi preservado. Essa
correção intencional é posterior à comparação original de hashes. O FP instalado
em `~/.local/bin/fp` foi atualizado.

## Exclusão interativa

`fp delete` implementado e instalado: seleção de capítulo/fase inteira,
resumo com caminho completo e registros afetados, confirmação padrão Não.

- `cargo fmt --check`, Clippy com `-D warnings`, **42 testes Cargo** e release passaram.
- 5 cenários PTY de delete: Esc, Ctrl+C, Não padrão, capítulo e fase inteira.
- Os 6 cenários PTY de init continuam passando.
- Cobertura de remoção de registros, preservação de irmãos/docs/Justfile,
  diretórios sem registro, links, conflitos, alteração concorrente, rollback
  e preservação do staging quando o rollback encontra destino ocupado.
- O conjunto de caminhos e hashes do repositório acadêmico permaneceu idêntico
  ao snapshot anterior a esta implementação. Exclusões somente em pastas temporárias.
- Binário atualizado: 1.612.544 bytes. Instalação em `~/.local/bin/fp`.

A exclusão é permanente após confirmação. Interrupções abruptas podem exigir
recuperação manual pelo staging `.fp-delete-*` e pelo registro; veja o README.

## Fluxo create com nomes automáticos

`fp create` substitui `fp init` sem alias. A primeira pergunta é o tipo:
Fase → número → `fase-NN`; Capítulo → fase de destino → número → `cap-NN` →
tecnologia. Não há pergunta de nome livre; registro e inicializadores usam o
nome do diretório gerado. Novos nomes não herdam sufixos descritivos de pastas
antigas, que continuam reconhecidas e preservadas.

- 42 testes Cargo passaram; parsing rejeita `fp init` e nome livre após create.
- Formatação, Clippy com warnings tratados e build release passaram.
- 8 cenários PTY de create passaram, incluindo número 3 escolhido para fase e
  capítulo, nomes `fase-03`/`cap-03` e ausência da antiga pergunta de nome.
- 5 cenários PTY de delete continuam passando.
- Dependência de normalização Unicode removida, pois não há mais nomes livres.
- Executável atualizado em `~/.local/bin/fp`; release com 1.612.544 bytes.
- Nenhum diretório acadêmico foi renomeado ou modificado nesta alteração.

## Init do workspace acadêmico

`fp init` foi reintroduzido com responsabilidade própria: preparar/verificar a raiz
acadêmica. `fp create` continua reservado à criação de fases e capítulos.

- Templates embutidos: AGENTS, README, Justfile, chapters.toml, pyproject.toml,
  gitignore, orientação de docs, dispatcher e testes Python.
- Inicialização de raiz inexistente, reparo aditivo, idempotência e preservação
  byte a byte de configurações existentes; sem criação de fases ou ambientes.
- `--check` não grava arquivos. Diagnóstico de ferramentas obrigatórias e
  opcionais, Python 3.11+, caminhos comuns de instalação e sugestões Fish.
- 58 testes Cargo passaram, incluindo 16 testes de workspace. O dispatcher gerado
  passou seus 11 testes Python numa raiz temporária, independente do FP.
- Formatação, Clippy com `-D warnings` e build release passaram.
- Os 8 cenários PTY de create e 5 de delete continuam passando.
- Listagem do pacote Cargo confirma os templates incluídos, inclusive gitignore.
- Instalado em `~/.local/bin/fp`; release com 1.612.544 bytes.
- `fp init` executado no workspace real: estrutura já completa. Conjunto de
  arquivos e hashes acadêmicos antes/depois idênticos (excluídos caches/venv).
- Na sessão do sistema, Vite+ foi detectado instalado fora do PATH; o comando
  informou `fish_add_path` para seu diretório. PlatformIO ausente e opcional.
  Just, Python, uv, Cargo, Zed, db, Docker e SQLite foram encontrados no PATH.

A inicialização não instala ferramentas, altera PATH, cria configurações globais,
usa sudo ou modifica permissões/serviços. Configurações arbitrárias existentes
são preservadas, sem migração automática. Erros de TOML, tipos de caminhos e
referências inválidas são reportados, sem inventar conteúdo de projetos.

## Tecnologias desabilitadas no create

O menu verifica estritamente os executáveis disponíveis no PATH antes da seleção.
Python requer uv, TypeScript/JavaScript requer vp, Rust requer cargo e IoT requer
pio ou platformio. As opções sem ferramenta aparecem esmaecidas com
`(não configurado)` e são puladas pela navegação; Enter só confirma itens ativos.
SQL e Somente pasta permanecem disponíveis. O caminho resolvido é usado pelo
inicializador, sem recorrer a instalações fora do PATH ou a sudo.

Cliclack 0.5.7 não possui suporte público a itens desabilitados. O seletor específico
de tecnologias reutiliza seu Theme e a crate console, que já era dependência
transitiva; os demais prompts continuam usando Cliclack diretamente. Sem fork,
framework adicional, tela inteira ou novos pacotes transitivos.

Passaram formatação, Clippy com warnings tratados, 62 testes Cargo e release.
Os 13 cenários PTY de create incluem seleção inicial sem ferramentas, bloqueio
de opções indisponíveis, salto sobre TypeScript sem vp, reativação com vp no PATH,
Esc e Ctrl+C. Os 5 cenários PTY de delete continuam passando. Inicializadores
simulados em diretórios temporários; nenhum template remoto foi baixado.
Binário instalado em `~/.local/bin/fp`, com 1.612.544 bytes.

## Primeira publicação e contribuição

Preparada a primeira release `0.1.0`, com Cargo.toml como fonte única da versão,
Cargo.lock consistente, changelog e política SemVer em CONTRIBUTING.md.
`fp --version` e `fp -V` são testados sem workspace nem ferramentas externas.
Os AGENTS do projeto e do template estão em inglês e exigem respostas ao usuário
em português brasileiro. Arquivos acadêmicos existentes não foram sobrescritos.

Passaram cargo fmt --check, Clippy com -D warnings, 64 testes Cargo, os 11 testes
Python do dispatcher gerado, 13 cenários PTY de create, 5 de delete e build release.
Versão conferida: fp 0.1.0. Binário: 1.612.544 bytes.
O histórico remoto inicial e a licença com autoria original foram preservados.

## GitHub Releases e self-update — 2026-11-02

Repositório confirmado pelo remote: `https://github.com/brasga-a/fp`.
Versão preservada: `0.1.0`. A tag `v0.1.0` já existia; não foi alterada.
Não existiam workflows antes desta implementação.

Validações concluídas:

- `cargo fmt --check`.
- `cargo clippy --all-targets --locked -- -D warnings`.
- `cargo test --locked`: 76 testes Cargo, incluindo 12 testes de atualização;
  o teste do dispatcher gerado também executou seus 11 testes Python.
- `cargo build --release --locked`.
- `./target/release/fp --version`: `fp 0.1.0`.
- `./target/release/fp update --help`: flag `--check` disponível.
- `python3 tools/check_tui.py target/release/fp`: 13 cenários PTY.
- `python3 tools/check_delete_tui.py target/release/fp`: 5 cenários PTY.
- `python3 tools/test_release.py`: 3 testes, incluindo tags incompatíveis,
  prereleases, zeros à esquerda e formato inválido.
- Validador real aceitou `v0.1.0` e rejeitou `v9.9.9`, sem criar tags.
- `actionlint` 1.7.12: workflow aprovado. Ferramenta baixada somente em `/tmp`.
- Simulação local do empacotamento: tar contendo somente `fp`, SHA-256 correto,
  permissões executáveis e versão do binário extraído conferida.
- `cargo tree --locked -i tokio`: nenhuma dependência ativa de Tokio.
- `git diff --check`: sem problemas.

Os testes de atualização injetam o transporte HTTP, sem servidor ou rede.
Cobrem comparação SemVer, seleção exata de target/origem, ausência de artefato,
checksum ausente/inválido, download truncado/falho, rede, rate limit, ausência de
release, instalação Cargo/sistema/incerta, falta de escrita, symlink/hardlink,
cancelamento, alteração concorrente, binário inválido e falha com restauração.
A fixture ELF tem fonte auditável e percorre o caminho real de verificação e
instalação da biblioteca, com destino temporário. Nenhum teste substitui o FP
instalado nem seu próprio executável.

Hashes mantidos durante a validação final:

- `~/.local/bin/fp`: `81f94d13feb7b7fe9aff9d42d4b43b841c61d66cf0f08537d53debf4cc85bb4e`.
- `~/.cargo/bin/fp`: `5368c0ae6983c25c12e03370ab37887802b4a2ffa7dfd6e7718c69b015ad152b`.
- `~/fiap/Justfile`: `ca2465a1b34e3ba94d1b5a8e7e87960ff6544033ddffe9b9192db3791d502058`.

Não foram executados comandos de escrita em `~/fiap`, alterados templates ou
Justfiles, instalados novos binários FP, criados commits/tags, enviados pushes
ou publicadas releases. O workflow foi validado localmente, sem execução remota
nem teste de publicação real. A compatibilidade GNU/glibc 2.35 será produzida
pelo runner Ubuntu 22.04 configurado no workflow; o build local usa o sistema
atual e não comprova a execução em todas as distribuições.

Limitações documentadas: atualização direta somente de instalações reconhecidas
em `~/.local/bin/fp` com marcador standalone; Cargo e pacotes usam o método
original. SHA-256 protege integridade, sem assinatura independente de autoria.
A publicação inicial com esta automação exige preparar uma nova versão e tag
(por exemplo, `0.2.0`), conforme README/CONTRIBUTING.

## Cargo workspace e gerenciador nativo — 2026-11-02

Versão preparada: `0.2.0`, única fonte em `[workspace.package]` na raiz;
`fiap-projects` permanece o pacote do executável `fp`. Quatro crates: fp-cli,
fp-workspace, fp-database e fp-update. Os módulos acadêmicos foram migrados com
reexports na biblioteca da CLI para preservar interfaces dos testes; lógica de
exclusão, bootstrap, registro e processos fica em fp-workspace, apresentação na
CLI. O atualizador expõe consulta/plano/instalação sem prompts na biblioteca.

A referência usada foi `~/Work/db`: db, README, tests/test_db.py e
scripts/setup-docker-user estavam disponíveis. `~/Work/db.zip` também existe;
sua listagem foi conferida. Dados, logs e locks de `.db/` não foram tratados
como fontes, copiados ou alterados. Nenhuma credencial de desenvolvimento foi
incorporada ao código. O wrapper externo e FP_DB_BIN foram removidos; três
testes específicos de delegação foram substituídos por testes do gerenciador.

Validações:

- cargo fmt --all --check.
- cargo clippy --workspace --all-targets -- -D warnings.
- cargo test --workspace --locked: 85 testes Rust, incluindo 12 do updater,
  9 da API de bancos e 3 de parsing/automação de bancos. O dispatcher continua
  executando seus 11 testes Python dentro da integração acadêmica.
- cargo build --release --locked; versão resultante `fp 0.2.0`.
- Ajuda principal, db e database; aliases por flags e rejeição de ações conflitantes.
- JSON sem TTY, sem ANSI, consultas sem criar diretórios e visual recusado com no-input.
- PTY: 13 cenários acadêmicos de criação, 5 de exclusão e 5 de bancos.
- python3 tools/test_release.py: 3 testes; check_release.py aceitou v0.2.0.
- actionlint 1.7.12 aprovou o workflow adaptado ao workspace.
- Pacote tar com fp e SHA-256 validado localmente, sem upload.
- Cargo metadata: quatro crates com versão herdada, sem ciclos ou dependência
  de fp-cli/Cliclack/Clap nas bibliotecas funcionais.

Os testes usam apenas temporários e executáveis simulados. SQLite é exercitado
com um cliente simulado baseado na biblioteca sqlite3 do Python, incluindo
execução de SQL. Docker simulado cobre criação sem start, persistência, adoção
legada, identidade, credenciais, autenticação MongoDB, timeout, inicialização
falha, locks, remoção explícita e preservação de dados importados. Nenhum Docker
real, script db legado ou credencial real foi utilizado nos testes.

Limitações operacionais:

- O gerenciador usa sqlite3 instalado no PATH; Oracle/MongoDB exigem Docker e
  clientes internos das imagens. Não foram executados servidores reais.
- run inicia e retorna; --wait consulta health/ping e pode expirar sem parar o
  banco. Pull/create pode exceder timeout e deixar recursos para inspeção manual.
- Não há transação única entre Docker, filesystem e registro; falhas parciais
  preservam dados/registro para diagnóstico e adoção manual, sem limpeza destrutiva.
- Oracle exec usa autenticação local SYSDBA no FREEPDB1 para o script solicitado.
  provision-user só cria/granta quando o usuário ainda não existe. A verificação
  de idempotência SQL deve ser confirmada em uma instância descartável pelo usuário.
- O registro não guarda senhas, mas Docker armazena variáveis dos containers em
  metadados acessíveis a quem controla o daemon. Scripts/logs solicitados podem
  conter dados sensíveis próprios do usuário; revise-os antes de compartilhar.
- Nenhuma migração automática. Importação exige create --existing explícito;
  dados importados não podem ser apagados pelo FP. init legado informa migração.
- Um lock remanescente após interrupção abrupta precisa de inspeção manual.

Não foram criados commits, tags, pushes ou releases. A instalação global não
foi atualizada: os dois executáveis existentes continuam com SHA-256
`0cdc6cc4df947e8208d63697575a9ab7a9e385a641a4b10ccee6c1e2ff9d302f`.
O Justfile acadêmico mantém SHA-256
`ca2465a1b34e3ba94d1b5a8e7e87960ff6544033ddffe9b9192db3791d502058`.

## Bootstrap rootless e credenciais automáticas — 0.3.0

`fp init` agora combina o preparo acadêmico e o setup do runtime de bancos.
`--academic-only` preserva o caminho sem downloads; `--check` apenas consulta.
`fp db init --setup-runtime` e a opção correspondente na TUI preparam somente
os bancos. A versão compartilhada e Cargo.lock foram atualizados para 0.3.0.

O setup fixa Docker/rootless extras 29.9.2 e slirp4netns 1.3.7, com hashes
SHA-256 revisados no código. Os pacotes oficiais foram baixados em `/tmp` para
inspeção e conferência dos hashes, sem executar instalador remoto. Runtime,
dados, configuração do daemon/cliente, socket e serviço systemd são exclusivos
do FP. O cliente usa a conexão persistida por recurso; registros antigos
continuam na conexão anterior. Imagens públicas são baixadas sem docker login;
uma imagem já presente é reutilizada. Nenhum container é criado pelo init.

Novos recursos podem usar credenciais aleatórias de /dev/urandom, com arquivos
regulares 0600 em diretório 0700. Registro, argv e logs do FP não contêm os
valores. Arquivos ausentes/inseguros causam falha, sem redefinir senha existente.
Variáveis explícitas e adoção legada continuam disponíveis. Esses segredos são
protegidos por permissões, não criptografados em disco.

SQLite agora é embarcado por rusqlite com bundled/hooks. SQL foi exercitado
com bancos temporários reais, incluindo consulta de resultados; sqlite3 não é
mais uma dependência de execução. Dependências adicionais do bootstrap são
self_update, sha2, tar e flate2, reutilizando bibliotecas já usadas pelo updater.

Validações concluídas:

- cargo fmt --all --check.
- cargo clippy --workspace --all-targets -- -D warnings.
- cargo test --workspace --locked: 92 testes Rust, incluindo 7 novos testes do
  bootstrap, segredos, roteamento e SQLite embarcado. Sem falhas.
- cargo build --release --locked; saída fp 0.3.0.
- 23 cenários PTY existentes (13 create, 5 delete, 5 bancos).
- 3 testes Python de tags, validação de v0.3.0 e actionlint sem erros.
- Bootstrap simulado: pré-requisito ausente sem escrita/download, checksum
  inválido sem publicação, setup idempotente, serviço alheio preservado,
  imagens baixadas uma vez, nenhuma chamada sudo/login/context use.
- Segredos diferentes por recurso, idempotência, permissões, symlink recusado
  e ausência sem regeneração. JSON/argv não recebem valores secretos.
- Roteamento para socket privado com credenciais geradas, sem alterar o cliente
  legado; SQLite funciona sem executável externo.

Os testes não iniciaram Docker/systemd reais nem tocaram bancos, unidades do
usuário ou o workspace acadêmico real. O Justfile continua com SHA-256
`ca2465a1b34e3ba94d1b5a8e7e87960ff6544033ddffe9b9192db3791d502058`.
Durante os testes, ambas as instalações Cargo 0.2.0 foram preservadas com hash
`5d023f7fa9774445d351e8e468ba06e46462f21232fce6d77414c5e213a46b68`.

Limitações explícitas: Linux x86_64, sessão systemd de usuário, newuidmap,
newgidmap, iptables e subuid/subgid devem estar disponíveis. Esta máquina possui
os mapeamentos e os helpers de UID/GID; faltavam os componentes rootless, agora
obtidos pelo bootstrap. Sistemas sem esses pré-requisitos exigem preparação
administrativa única; o FP não tenta conceder privilégios por conta própria.
Não foi validado um daemon rootless real ou uma inicialização real das imagens
Oracle/MongoDB nesta implementação. Downloads iniciais podem ocupar vários GB
e dependem de rede/espaço/limites anônimos do registry. Nenhum login é solicitado
em caso de falha: o erro orienta repetir o setup. Linger não é configurado.
Não há migração automática nem alteração de credenciais de recursos existentes.

Após a validação, as duas instalações Cargo locais foram atualizadas para
`fp 0.3.0`, preservando seus prefixos. A checagem real read-only
`fp db init --setup-runtime --dry-run --no-input --output json` passou pelos
pré-requisitos desta máquina e retornou `ready: false` para o socket
`/run/user/1000/fp-docker/docker.sock`, confirmando que o runtime ainda não foi
instalado/iniciado. Nenhuma imagem real ou serviço foi iniciado por essa checagem.
O próximo `fp init` do usuário fará o download e a preparação efetiva.

## Preparação da publicação v0.3.0

Na revalidação pré-publicação foi observada uma falha intermitente ao iniciar
um executável simulado recém-escrito. O executor passou a repetir somente spawn
recusado por ETXTBSY, de forma limitada, e a preservar a causa dos demais erros.
Comandos que já iniciaram não são repetidos. O conjunto de testes de bancos
passou em 20 execuções consecutivas; depois passaram novamente os 92 testes do
workspace, fmt, Clippy, build release e validadores de tag/workflow.


Na primeira execução remota da release v0.3.0, os testes acadêmicos detectaram
Python 3.10 no runner Ubuntu 22.04 (tomllib ausente). O workflow foi corrigido em
main para selecionar Python 3.11 explicitamente e permitir retomada manual com
checkout da tag existente. A tag v0.3.0 e seu código foram preservados.
