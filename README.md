# FP — FIAP Projects

CLI nativa em Rust para criar, listar e executar projetos acadêmicos em um
monorepo poliglota. O executável é `fp`; o pacote Cargo é `fiap-projects`.
A raiz padrão é `~/fiap`. O FP delega a inicialização às ferramentas dos
ecossistemas e a execução ao Just.

## Requisitos e instalação

- Linux, Rust stable recente com Cargo para compilar (edição 2024; validado em 1.99.1).
- Just para `fp run` e para o teste de integração com Just real.
- Terminal interativo para `fp create`; mouse não é necessário.
- uv, Vite+ (`vp`), Cargo ou PlatformIO (`pio`/`platformio`) apenas para os tipos correspondentes.
- Zed (`zed`) opcional.

```sh
cd ~/Work/fp
cargo install --path crates/fp-cli --locked
fp --help
```

Adicione o diretório de executáveis do Cargo ao PATH se necessário. No Fish,
para uma instalação Cargo padrão: `fish_add_path ~/.cargo/bin`.
O FP não modifica o PATH nem configurações globais.

O binário compilado funciona sem Rust, Python ou Node instalados. **Os comandos
do repositório continuam exigindo suas ferramentas**: o dispatcher existente de
`~/fiap`, por exemplo, usa Python 3.11+ e Just. Não é uma dependência do FP para
listar ou criar pastas.

## Uso

```sh
fp init
fp init --check
fp create
fp delete
fp list
fp run 1 2
fp run 2 3 --task dev -- --host 127.0.0.1
fp --root /caminho/academico create
fp --root /caminho/academico run 1 2
fp --root /caminho/academico list
```

A prioridade de configuração é `--root`, depois `FP_ROOT`, depois `~/fiap`.
Caminhos relativos são resolvidos a partir do diretório atual, `~` é expandido e
links/segmentos `..` são normalizados. `fp init` aceita uma raiz ainda inexistente.
Os demais comandos acadêmicos exigem uma raiz existente com `Justfile` e
`chapters.toml` válidos no contrato abaixo.

```fish
set -gx FP_ROOT ~/fiap
fp list
```

Também funciona por comando: `env FP_ROOT=/caminho/academico fp list`.
Nenhum caminho de usuário é fixado no código.

### `fp init`: preparar o workspace

```sh
fp init
fp init --check
fp --root /caminho/novo/workspace init
```

Cria a raiz configurada se necessário e acrescenta somente a estrutura ausente:

```text
AGENTS.md
README.md
Justfile
chapters.toml
pyproject.toml
.gitignore
docs/README.md
scripts/chapters.py
tests/test_chapters.py
```

`AGENTS.md` reúne orientações acadêmicas, pesquisa em materiais e preservação dos
exercícios. O README explica organização e comandos. O Justfile, dispatcher e
seus testes reproduzem o contrato já utilizado no monorepo FIAP; o Just funciona
sem o FP instalado. Os arquivos estão embutidos no executável: não há downloads
de templates em tempo de execução.

A operação é idempotente: mantém arquivos existentes byte a byte e completa
apenas os ausentes. Não cria fases, capítulos, `.venv`, Git, lockfiles de pacotes,
workspaces Cargo/JavaScript ou credenciais. O novo manifesto Python declara um
workspace uv vazio; `fp create` acrescenta os capítulos Python como membros.
Manifestos existentes não são migrados por `fp init`.
Conflitos de tipo, links em caminhos gerenciados, TOML inválido e referências a
capítulos inexistentes são reportados sem sobrescrever nem recriar projetos.
O conteúdo de Justfiles personalizados e instruções existentes é preservado;
o FP não migra receitas nem tenta corrigir configurações arbitrárias.

O diagnóstico verifica Just e Python 3.11+ (necessários ao dispatcher) e indica
uv, Vite+, Cargo, PlatformIO, Zed, db, Docker e SQLite como ferramentas opcionais.
Procura executáveis no PATH e, quando ausentes, verifica diretórios comuns como
`~/.local/bin`, `~/.cargo/bin`, `~/.local/share/vite-plus/bin`,
`~/.platformio/penv/bin`, `/usr/local/bin` e `/usr/bin`. Quando encontra uma
ferramenta fora do PATH, mostra seu caminho e uma sugestão `fish_add_path`.
Não altera PATH, configurações globais, permissões ou serviços e não usa sudo.

`--check` apenas verifica, sem criar arquivos ou locks. Códigos de saída:
0 para estrutura completa e requisitos básicos disponíveis; 1 para estrutura
incompleta na verificação ou ferramenta obrigatória ausente/incompatível; 2
para configuração inválida, conflito de caminho ou falha de escrita. Ferramentas
opcionais ausentes não impedem a inicialização. Mesmo sem os requisitos básicos,
`fp init` pode preparar os arquivos e indicar o que precisa ser instalado.

A escrita usa arquivos temporários e publicação sem substituição. Em caso de
interrupção, arquivos já criados ficam preservados; execute novamente para
completar. O lock é compartilhado com create/delete. Não há reinterpretação do
registro para inventar comandos de projetos antigos.

### `fp create`

Abre uma TUI **inline Cliclack**, com histórico das respostas, menus navegáveis
com setas, Enter para confirmar, Esc/Ctrl+C para cancelar e indicadores de estado.
Não usa tela alternativa nem menus numerados de stdin.

1. Escolha Fase ou Capítulo.
2. Para Fase, informe o número: o sistema gera `fase-NN`.
3. Para Capítulo, selecione a fase e informe o número: o sistema gera `cap-NN`.
4. Para Capítulo, escolha a tecnologia.
5. Confira nome gerado, tipo, fase, tecnologia e caminho antes de confirmar.
6. Ao terminar, escolha se deseja abrir a pasta no Zed.

Não há pergunta de nome livre. Por exemplo, Fase → 3 gera `fase-03`;
Capítulo → fase 3 → 2 gera `fase-03/cap-02`. O registro mantém `cap-02`; o projeto
Python recebe `fase-03-cap-02`, e o projeto Cargo recebe `cap-02`.
Uma fase cria somente uma pasta.
`fp create` cria projetos. `fp init` prepara e verifica o workspace acadêmico.

A numeração sugerida é o maior número existente + 1; você pode informar outro
número positivo disponível. Sem fases, crie uma Fase primeiro. Não há alterações
antes da confirmação. Esc/Ctrl+C retornam 130; recusar a confirmação retorna 0.

A descoberta segue `fase-NN/cap-NN`, com ordenação numérica. Novos nomes contêm
somente prefixo e número, sem sufixos descritivos. A largura existente é preservada
(por exemplo, `cap-009` leva a `cap-010`), com dois dígitos por padrão. Diretórios
antigos com sufixos continuam reconhecidos, sem renomeação. O capítulo recebe o
nome gerado em `chapters.toml`. O nome do projeto Python inclui a fase, por exemplo
`fase-03-cap-02`, para permitir capítulos com o mesmo número em fases diferentes.
Uma fase não ganha metadados extras.
`docs/fase-NN` contém materiais e não conta como fase de projetos.
Numerações duplicadas são reportadas como ambíguas; links de diretórios não são
atravessados na descoberta.

### Tecnologias disponíveis

| Opção | Inicialização | Execução registrada |
| --- | --- | --- |
| Python | `uv init --app --no-package --no-workspace --vcs none …`, seguido de registro no workspace da raiz | `uv run --all-packages main.py` |
| TypeScript / JavaScript | `vp create vite:application --directory . --no-git` | `vp run dev --` se existir script `dev` |
| Rust | `cargo init --bin --vcs none --name cap-01 .` | `cargo run --` se existirem manifest e `src/main.rs` |
| ESP32 / IoT | `pio project init --board BOARD` | Nenhuma operação de hardware automática |
| SQL / Database | Pasta `sql/` vazia | Nenhum |
| Somente pasta | Diretório vazio | Nenhum |

Python cria uma aplicação simples pelo uv (`--app --no-package`), com
`pyproject.toml` e `main.py`. O FP adiciona o caminho exato do capítulo em
`tool.uv.workspace.members` no manifesto da raiz, preservando dependências,
comentários e configurações existentes. Sem manifesto na raiz, cria a configuração
acadêmica padrão. Só capítulos Python são incluídos; fases podem conter outras
tecnologias. Exclusões personalizadas no workspace requerem revisão manual antes
da criação, sem sobrescrever a configuração do usuário.

O `requires-python` da raiz é passado ao uv; downloads implícitos de Python e
pinagem de versão ficam desativados. A criação não instala dependências nem cria
ambientes ou lockfiles. Na execução, uv usa uma única `.venv` e `uv.lock` na raiz.
O alvo `main` usa `uv run --all-packages main.py`, incluindo as dependências da
raiz e de todos os membros, mesmo quando o comando parte da pasta do capítulo.
Se o inicializador não gerar `main.py` ou um manifesto válido, a criação falha
sem publicar um capítulo incompleto.

Para adicionar uma biblioteca disponível aos capítulos Python:

```sh
uv add --project ~/fiap numpy
fp run 3 2
```

Use a raiz configurada em lugar de `~/fiap` quando houver `--root` ou `FP_ROOT`.
Para sincronizar todas as dependências manualmente, use
`uv sync --project ~/fiap --all-packages`; dentro de um capítulo,
`uv run --all-packages main.py`. Requisitos incompatíveis entre capítulos precisam
ser conciliados, pois o workspace tem uma resolução compartilhada. Consulte a
[documentação de workspaces do uv](https://docs.astral.sh/uv/concepts/projects/workspaces/).
Projetos Python antigos e seus comandos não são migrados automaticamente.

Os manifestos novos incluem `tool.pyright.venvPath` e `tool.pyright.venv`, com
caminhos relativos para a `.venv` compartilhada. Assim, Pyright/Basedpyright
resolvem imports mesmo ao abrir só a pasta do capítulo no Zed. Configurações
personalizadas existentes do analisador são preservadas. Se o editor já estava
aberto durante uma migração, reinicie seu servidor de linguagem. No Zed, use
`editor: restart language server` na paleta de comandos.

O Vite+ mantém seus prompts de linguagem, framework e gerenciador de pacotes;
o FP não força npm, pnpm, Bun ou Yarn. O template é uma aplicação independente.
Vite+ e PlatformIO podem baixar pacotes e executar código desses pacotes: o
assistente informa isso **antes da confirmação**. O FP não instala ferramentas
faltantes. O seletor verifica executáveis no PATH: Python depende de `uv`,
TypeScript/JavaScript de `vp`, Rust de `cargo`, IoT de `pio` ou `platformio`.
Sem a ferramenta, a opção aparece esmaecida com **(não configurado)** e não pode
ser selecionada; as setas pulam esses itens. SQL e Somente pasta estão sempre
disponíveis. Para diagnosticar a configuração, execute `fp init --check` e reabra
`fp create` depois de ajustar o PATH. Ferramentas instaladas fora do PATH não
habilitam a opção automaticamente.

PlatformIO pede o board ID e opcionalmente o framework; consulte `pio boards`.
Não presume ESP32 específico, Arduino ou ESP-IDF e não compila, grava nem abre
monitor serial automaticamente. SQL não configura serviços nem credenciais.

### `fp delete`

```sh
fp delete
fp --root /caminho/academico delete
```

Abre uma TUI inline para escolher **Capítulo** ou **Fase inteira**, selecionar a
fase e, quando aplicável, o capítulo. Antes de excluir, mostra o caminho completo,
o tipo e os registros que serão removidos de `chapters.toml`.

A confirmação **Excluir permanentemente?** começa em Não. Enter mantém essa
escolha; Esc/Ctrl+C cancelam o fluxo. Até confirmar Sim, nenhum arquivo é
alterado. Após confirmar, a exclusão é permanente, incluindo arquivos ocultos,
subpastas e todo o conteúdo da fase quando essa opção for selecionada.

O FP remove os registros cujos diretórios estão dentro da pasta excluída,
preservando outros capítulos, o Justfile e materiais em `docs/`. Remove também
os caminhos explícitos dos membros uv dentro do destino; dependências da raiz,
o ambiente compartilhado e o lockfile são preservados. O uv atualiza o lockfile
na próxima sincronização ou execução. Pastas da
convenção existente podem ser excluídas mesmo sem registro. Se um comando de
outro registro usa `cwd` dentro do destino, a exclusão é bloqueada para não
deixar um comando inválido. Links de fase/capítulo não são oferecidos; links
internos são removidos sem percorrer seus destinos.

A operação compartilha o lock de `fp create`, verifica mudanças no registro e
na identidade do diretório após a seleção, move a pasta para staging na raiz e
atualiza o registro antes de apagar os arquivos. Se a atualização falhar,
tenta restaurar a pasta. Se o destino for ocupado ou o rollback falhar,
preserva o staging e informa o caminho de recuperação. Uma interrupção brusca
pode deixar `.fp-init.lock` e `.fp-delete-*`; inspecione os dados e o registro
antes de removê-los. O staging não é uma lixeira nem um backup permanente.

Verificação da TUI exclusivamente em repositórios temporários:
`python3 tools/check_delete_tui.py target/release/fp`.

### `fp run`

Executa exatamente `just run FASE CAPITULO ARGUMENTOS…` na raiz configurada,
independentemente do diretório atual. Stdin, stdout e stderr são herdados, sem
shell intermediário. Os argumentos são encaminhados literalmente. O código de
saída do Just é preservado; término por sinal é representado como `128 + sinal`.
Erros do próprio FP retornam 2. Just ausente gera uma mensagem de instalação.

### `fp list`

Mostra árvore de fases/capítulos, nome, tecnologia e estado `executável` ou
`sem comando`. A tecnologia usa manifests (`pyproject.toml`, `package.json`,
`Cargo.toml`, `platformio.ini`) e estrutura SQL, incluindo manifests nos `cwd`
registrados, e ferramentas explícitas nos comandos quando não há manifest.
Não deduz execução por extensão de arquivo. Entradas explícitas
fora da convenção de diretórios também aparecem como `Registro FASE.CAPITULO`.
Não executa ferramentas externas para listar.

## Integração com Just

`chapters.toml` é a fonte declarativa usada pelo dispatcher existente. O FP
preserva o Justfile e os comandos anteriores. Exemplo de capítulo executável:

```toml
[chapters."2.3"]
path = "fase-02/cap-03"
name = "Exemplo Rust"
default = "main"

[chapters."2.3".commands.main]
argv = ["cargo", "run", "--"]
```

Um alvo pode ter `cwd = "subprojeto"`, relativo ao capítulo. Para Python:
`argv = ["uv", "run", "--all-packages", "main.py"]` para membros do workspace
compartilhado. Só registre esse comando após criar o arquivo.
Capítulos sem comandos possuem somente `path` e `name`. Outros alvos podem ser
adicionados manualmente em `commands`; o dispatcher aceita `--task ALVO`.

A raiz deve fornecer uma receita equivalente ao contrato já existente:

```just
set shell := ["sh", "-cu"]
set positional-arguments
run phase chapter *args:
    @python3 scripts/chapters.py run "$@"
```

`fp init` fornece esse dispatcher em raízes novas, preservando arquivos existentes.
O FP delega a execução ao Just e não reinterpreta seus comandos.
`just run 2 3` continua funcionando sem o FP instalado.

### Integridade da criação

Após confirmação, `.fp-init.lock` impede duas criações simultâneas pelo FP.
Inicializadores trabalham em diretório temporário fora da árvore acadêmica,
evitando detecção de workspaces ancestrais. O destino é reservado com criação
exclusiva; arquivos são movidos, ou copiados entre filesystems. O registro é
editado com preservação de comentários e publicado por substituição atômica,
com verificação de alterações externas. A inclusão no manifesto uv participa do
rollback se a publicação do registro falhar; alterações concorrentes não são
sobrescritas. Falhas normais removem somente o novo destino criado pela operação.
Diretórios existentes nunca são substituídos.

O FP configura apenas o workspace uv dos novos capítulos Python. Não cria
workspaces Cargo/JavaScript, commits, configurações globais ou atalhos do Omarchy.

## Zed

Depois de criar, escolha Sim para executar `zed CAMINHO` diretamente, sem `--wait`.
Se o editor não estiver disponível, a criação permanece válida e uma mensagem é
exibida. Não há dependência de aliases ou funções Fish.

## Organização do código

```text
Cargo.toml / Cargo.lock       manifesto virtual e dependências compartilhadas
crates/fp-cli                 pacote fiap-projects; binário fp, Clap e TUIs
crates/fp-workspace           operações acadêmicas, Just, registro e bootstrap
crates/fp-database            API de bancos; model.rs, manager.rs e process.rs
crates/fp-update              API de consulta e instalação segura de releases
assets/                      templates acadêmicos compartilhados
tests/fixtures/              artefatos simulados de atualização
tools/                       validações e cenários PTY
```

Dependências diretas: Clap, Cliclack, `console`, `toml_edit` (preservar TOML), `serde_json`
(ler scripts JavaScript), `tempfile` (staging), `self_update` (GitHub Releases e
substituição segura) e `semver` (comparação de versões). `console` já é usado pelo Cliclack;
o FP usa a mesma dependência no seletor de tecnologias, pois Cliclack 0.5 não
expõe itens desabilitados. Esse seletor reutiliza o Theme do Cliclack e mantém
renderização inline, histórico e cancelamento. Sem runtime assíncrono ou framework.

Para adicionar uma tecnologia, amplie `Technology` e o seletor, use o
inicializador oficial em `scaffold`, detecte um entrypoint verificável em
`entrypoint` e acrescente a detecção de manifest em `discovery`. Cubra o contrato
com um executável simulado nos testes. Não adicione um sistema genérico de plugins.

## Desenvolvimento e release

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
./target/release/fp --help
python3 tools/check_tui.py target/release/fp
cargo install --path crates/fp-cli --release --locked
```

O perfil release usa `opt-level = "s"`, LTO thin e remoção de símbolos. Entrega um
único ELF nativo, ligado às bibliotecas de sistema normais do Linux. O teste PTY
opcional usa somente a biblioteca padrão Python; Python não faz parte do binário.
Os testes Cargo usam arquivos e inicializadores simulados em diretórios temporários,
sem inicializar projetos reais ou modificar `~/fiap`.

Consulte [VALIDATION.md](VALIDATION.md) para os resultados desta implementação.

## Limitações e melhorias possíveis

- Vite+, PlatformIO e Zed não estavam instalados durante a implementação; seus
  contratos foram testados com executáveis simulados. Fluxos oficiais podem mudar
  conforme a versão instalada; valide-os ao atualizar essas ferramentas.
- Inicializadores podem alterar caches e baixar dependências; esses efeitos não
  são desfeitos. O FP reverte a publicação local em falhas normais, mas não oferece
  transação contra queda de energia ou SIGKILL. Uma interrupção brusca pode deixar
  `.fp-init.lock`, staging ou novo destino; inspecione-os antes de remover manualmente.
- A verificação de edição concorrente protege alterações observadas antes da
  publicação; editores que ignoram o lock ainda podem escrever nessa janela.
- O contrato de integração é o `chapters.toml` FIAP. Receitas Just arbitrárias e
  templates remotos personalizados não são gerenciados.
- Novos capítulos Python compartilham o workspace uv. Projetos e comandos antigos
  são preservados e precisam de migração explícita para aderir a esse modelo.
- Melhorias possíveis: validar versões suportadas dos inicializadores e ampliar
  testes em ambientes com Vite+, hardware PlatformIO e Zed reais.

## Contribuição e versões

Veja [CONTRIBUTING.md](CONTRIBUTING.md) para preparar o ambiente, validar mudanças
e publicar versões. O histórico fica em [CHANGELOG.md](CHANGELOG.md).
A versão vem exclusivamente de `Cargo.toml` e pode ser consultada com
`fp --version` ou `fp -V`. Releases usam tags Git no formato `vMAJOR.MINOR.PATCH`.

Para instalar a primeira versão publicada:

```sh
cargo install --git https://github.com/brasga-a/fp.git --tag v0.1.0 --locked
```

## Distribuição e atualização pelo GitHub

A plataforma inicial é **Linux x86_64**, target `x86_64-unknown-linux-gnu`.
O build roda no Ubuntu 22.04, usando glibc 2.35 como base. Foi escolhido GNU para
integração simples com Linux modernos, especialmente Arch e derivados. Não há
build musl, suporte a Linux sem glibc, ARM, Windows ou macOS nesta etapa.
Rust/Cargo só são necessários para compilar; o executável distribuído não exige Rust.

As releases estáveis oficiais ficam em
[brasga-a/fp](https://github.com/brasga-a/fp/releases). Cada tag `vX.Y.Z` publica:

- `fp-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`, contendo somente `fp`;
- `SHA256SUMS`, no formato padrão de `sha256sum`.

### Instalar um binário standalone

Escolha uma versão **que tenha os artefatos publicados** na página de releases.
Os comandos abaixo usam Bash; ajuste `version` para essa versão. A tag `v0.1.0`
anterior ao workflow não recebe binários retroativamente.

```bash
version=0.2.0 # exemplo: confirme a versão publicada antes de baixar
archive="fp-v$version-x86_64-unknown-linux-gnu.tar.gz"
base="https://github.com/brasga-a/fp/releases/download/v$version"
tmp=$(mktemp -d)
curl --fail --location --proto '=https' --proto-redir '=https' "$base/$archive" -o "$tmp/$archive"
curl --fail --location --proto '=https' --proto-redir '=https' "$base/SHA256SUMS" -o "$tmp/SHA256SUMS"
(cd "$tmp" && sha256sum --check SHA256SUMS) &&
  tar -xzf "$tmp/$archive" -C "$tmp" fp &&
  "$tmp/fp" --version
```

Continue **somente se a verificação e a execução tiverem sucesso**. Se já houver
`~/.local/bin/fp`, confira seu método de instalação antes de substituir. Não copie
sobre um symlink nem use estes passos para sobrescrever uma instalação Cargo ou
de um gerenciador de pacotes. Para uma instalação nova:

```bash
mkdir -p "$HOME/.local/bin"
# Recusa um arquivo ou symlink já existente.
if [ ! -e "$HOME/.local/bin/fp" ] && [ ! -L "$HOME/.local/bin/fp" ]; then
  install -m 755 "$tmp/fp" "$HOME/.local/bin/fp" &&
    printf 'fp-standalone: https://github.com/brasga-a/fp\n' > "$HOME/.local/bin/.fp-standalone"
fi
"$HOME/.local/bin/fp" --version
rm -r -- "$tmp"
```

O arquivo `.fp-standalone` é a declaração explícita da origem da instalação.
Crie-o somente ao instalar um binário verificado das releases oficiais. O FP
não converte instalações existentes em standalone automaticamente.

`~/.local/bin` pode não estar no PATH. Você pode usar o caminho completo ou
adicionar `export PATH="$HOME/.local/bin:$PATH"` à configuração do Bash.
No Fish, use `fish_add_path ~/.local/bin`. Verifique `command -v fp` e
`fp --version` para confirmar qual instalação está sendo executada.

### Consultar e instalar atualizações

```sh
fp --version
fp update --check
fp update
```

`--check` consulta somente os metadados da última release estável pela API do
GitHub: não baixa artefatos, não grava arquivos e não pede confirmação. Funciona
fora de `~/fiap` e em instalações Cargo ou do sistema. Retorna 0 tanto quando há
uma nova versão quanto quando a versão instalada já é igual ou superior; erros
retornam 2. Não requer um token pessoal. Sem conexão, sem release ou com limite
da API atingido, apresenta erro e permite tentar novamente mais tarde.

`fp update` compara versões SemVer e nunca instala uma versão inferior. A API
`releases/latest` do GitHub exclui drafts e prereleases; o FP também rejeita
versões com sufixos de prerelease/build. Seleciona o nome exato do arquivo para
seu target, mostra as versões e solicita confirmação em terminal, com Não como
padrão. Recusar retorna 0; Esc/Ctrl+C retornam 130. Não há opção de confirmação
automática nem canal beta/nightly.

A substituição direta só é permitida no caminho real `~/.local/bin/fp`, com o
marcador acima, arquivos pertencentes ao usuário, permissões adequadas e sem
symlinks nos caminhos ou hardlinks no binário. Metadados Cargo no prefixo e
`CARGO_HOME` também bloqueiam a substituição. Binários manuais em outros locais
ou de origem incerta devem ser atualizados pelo método original. Um gerenciador
que instale nesse prefixo deve remover o marcador standalone ao assumir o arquivo;
nenhuma heurística consegue reconhecer todos os gerenciadores possíveis.

Para Cargo, repita `cargo install --path crates/fp-cli --force --locked` no checkout atualizado,
ou `cargo install --git https://github.com/brasga-a/fp.git --tag vX.Y.Z --force --locked`.
Para pacotes do sistema, use o gerenciador original. O FP não chama sudo, não
muda permissões de diretórios protegidos e não altera Justfile nem `~/fiap`.

O download usa HTTPS no repositório fixo oficial. `self_update` baixa para staging
num diretório temporário, exige o checksum publicado em `SHA256SUMS`, verifica a
integridade antes de extrair e também verifica o digest do asset fornecido pelo
GitHub, quando disponível. O FP valida ELF64 x86_64 e a saída `--version` do
binário extraído. A biblioteca copia o novo executável para um temporário ao lado
da instalação e o publica por rename, preservando permissões. Falhas anteriores
ao rename mantêm o executável anterior; não há backup permanente após sucesso.
Há necessidade de espaço para o download e as cópias temporárias.

SHA-256 detecta corrupção, mas **não comprova autoria**: quem controlar a release
pode trocar o binário e o checksum juntos. A confiança depende do repositório e
do HTTPS do GitHub. Não há assinatura independente nesta etapa. Downloads
interrompidos ou checksums ausentes/divergentes não são instalados.

### Publicar uma nova versão

A única fonte da versão é `workspace.package.version` em `Cargo.toml`; Clap incorpora essa
versão no build. Não existe incremento automático. Edite a versão, atualize o
`Cargo.lock` com `cargo check` e prepare a seção correspondente do CHANGELOG.
Execute as validações de [CONTRIBUTING.md](CONTRIBUTING.md), revise e faça commit.

A tag `v0.1.0` já existe e não deve ser movida. Para a primeira release com esta
automação, prepare uma versão nova — por exemplo `0.2.0` — e, **após commitar o
workflow e a versão correspondente**, publique a tag:

```sh
# Apenas depois de preparar Cargo.toml/Cargo.lock/CHANGELOG para 0.2.0:
python3 tools/check_release.py v0.2.0
cargo build --release --locked
./target/release/fp --version
git tag -a v0.2.0 -m "FP 0.2.0"
git push --atomic origin main v0.2.0
```

O [workflow de release](.github/workflows/release.yml) rejeita tags que não sejam
`vX.Y.Z` ou que discordem do Cargo. Instala Rust stable e Just para os testes
existentes, roda fmt/Clippy/testes, compila release, confere `--version`, empacota
apenas `fp` e gera SHA-256. Somente após tudo passar, outro job publica usando
`GITHUB_TOKEN`, com `contents: write` restrito à publicação. As actions são
fixadas por hash de commit. Não exige servidores, domínios, PATs, crates.io ou
ferramentas locais além das já usadas no desenvolvimento.

A publicação começa em rascunho e só fica pública depois dos uploads. O workflow
não sobrescreve releases existentes nem cria/envia tags. Se uma publicação
falhar deixando rascunho, inspecione-o no GitHub antes de concluir ou removê-lo
manualmente e reexecutar. Nunca mova uma tag publicada para corrigir binários;
prepare outra versão. Não reutilize um número de versão já lançado.

Para testar sem rede nem instalações reais:

```sh
cargo test -p fp-update --locked
python3 tools/test_release.py
python3 tools/check_release.py v0.2.0 # ajuste se a versão mudar
```

Os testes usam HTTP injetado e fixtures, inclusive checksum ausente/inválido,
download truncado, rede indisponível, cancelamento, instalações protegidas e
falha de substituição. A fixture de sucesso é um ELF mínimo, com fonte em `tests/fixtures/update/stub.S`,
que imprime uma versão simulada e encerra. O teste percorre checksum, extração,
validação de formato/versão e instalação somente em diretório temporário.
Uma segunda fixture, em formato de script, testa a rejeição de binário inválido.
A biblioteca foi consultada na [documentação de self_update](https://docs.rs/self_update/1.3.3/self_update/).
Dependências diretas novas: `self_update` (cliente síncrono ureq, rustls, GitHub,
tar/gzip e checksums) e `semver`. Não há runtime assíncrono no build do FP.

## Gerenciador nativo de bancos (0.3.0)

`fp db` e `fp database` são aliases. Não dependem do script `db` nem de
`FP_DB_BIN`. O núcleo em Rust usa somente Docker e seus clientes internos para
Oracle/MongoDB. SQLite está embarcado no executável e não exige Docker nem sqlite3.
Os comandos não dependem de `~/fiap` e não modificam projetos acadêmicos.

| Comando | Semântica |
| --- | --- |
| `init` | Prepara configuração/dados/estado; não cria nem inicia bancos |
| `create` | Cria arquivo SQLite ou container parado com volume persistente |
| `run` | Inicia recurso registrado; nunca cria implicitamente |
| `stop` | Para o container, preservando os dados; SQLite não tem servidor |
| `list` | Lista registros sem chamar Docker ou gravar arquivos |
| `list --discover` | Consulta containers legados fiap-oracle/fiap-mongodb |
| `status` | Consulta arquivo ou estado/health do container |
| `logs` | Retorna últimos 100 registros Docker ou eventos locais do SQLite |
| `exec` | Executa o script explicitamente indicado por `--file` |
| `provision-user` | Cria usuário Oracle somente se não existe |
| `delete` | Remove por padrão só o registro |
| `visual` | Abre o menu Cliclack sobre a mesma API |

A sintaxe canônica é `fp db create --engine sqlite --name estudos`.
Clap também reconhece `--init`, `--create`, `--delete`, `--run` e `--visual`
como nomes alternativos dos respectivos subcomandos, com os mesmos argumentos.
Duas ações na mesma chamada são recusadas. Opções globais podem aparecer antes
ou depois da ação. Não há `--force` para operações de bancos.

### Automação e SQLite

```sh
fp db init --no-input --output json
fp db create --engine sqlite --name estudos --no-input --output json
fp db list --output json
fp db status --name estudos --output json
fp db exec --name estudos --file /caminho/consulta.sql --no-input --output json
fp db create --engine sqlite --name importado --file /caminho/existente.db --existing --no-input
```

Exemplo de listagem vazia: `{"resources":[]}`. JSON de consultas/criação inclui
identificadores, engine e parâmetros não sensíveis. `exec` retorna
`{"id":"estudos","output":"..."}`. JSON não contém ANSI, banners ou spinners.
Diagnósticos ficam em stderr, com identificadores como `E_NOT_FOUND`,
`E_CONFIRM`, `E_CONFLICT`, `E_CREDENTIALS`, `E_BUSY`, `E_TOOL` e `E_TIMEOUT`;
falhas retornam 2, sucesso retorna 0, cancelamento por Esc/Ctrl+C retorna 130.
Erros de parsing são reportados pelo Clap em stderr e retornam 2.

Todos os comandos diretos são não interativos. `--no-input` também impede abrir
a interface visual acidentalmente. Sem ação, `fp db` abre o menu somente quando
stdin e stderr são terminais; fora de TTY retorna orientação sem bloquear.
Agentes devem sempre fornecer operação e parâmetros explícitos, usando
`--no-input --output json`, sem navegar pela TUI.

`--dry-run` está disponível para init/create/run/stop/exec/delete/provision-user:
valida parâmetros e descreve a operação sem gravar arquivos nem executar scripts.
Não garante espaço/permissão nem sucesso futuro de Docker. Operações sobre
recursos existentes exigem registro mesmo em dry-run. Scripts SQL/JavaScript
são código solicitado pelo usuário, com os privilégios do cliente; revise-os.

### Configuração e estado

Os padrões são `~/.config/fp/databases.json`, `~/.local/share/fp/` e
`~/.local/state/fp/`. Variáveis `XDG_CONFIG_HOME`, `XDG_DATA_HOME` e
`XDG_STATE_HOME` absolutas têm prioridade. Para isolamento/testes use
`--config-dir`, `--data-dir` e `--state-dir`, com diretórios distintos dos reais.
Nenhum banco novo é criado junto do executável ou automaticamente no código-fonte.

`database.lock` serializa operações mutáveis. `database.log` contém eventos,
sem senhas ou conteúdo dos scripts. Após queda abrupta, inspecione o processo
indicado no lock e os recursos antes de remover um lock remanescente manualmente.
O registro é salvo por temporário e rename. Arquivos SQLite e IDs Docker são
conferidos para recusar substituições de identidade. Consultas não criam diretórios.

### Oracle Free e MongoDB

Execute `fp init` para preparar o Docker rootless e baixar as imagens públicas.
Novos bancos recebem senha aleatória local e usuário MongoDB `fp`, sem pedir login.
Opcionalmente, `--password-env VAR` e `--username` permitem configuração avançada
ou adoção de recursos existentes, cujas credenciais nunca são redefinidas.

```sh
fp db init
fp db create --engine oracle --name oracle-a --no-input
fp db run --name oracle-a --no-input
fp db status --name oracle-a --output json
fp db run --name oracle-a --wait --timeout 600 --no-input
fp db provision-user --name oracle-a --username estudante --password-env FP_STUDENT_PASSWORD --no-input
fp db exec --name oracle-a --file /caminho/exercicios.sql --no-input
fp db stop --name oracle-a --no-input

fp db create --engine mongodb --name mongo-a --no-input
fp db run --name mongo-a --no-input
fp db exec --name mongo-a --file /caminho/consulta.js --no-input --output json
```

Imagem Oracle padrão: `gvenzl/oracle-free:23-slim` (pública, mantida pela comunidade),
volume em `/opt/oracle/oradata`, porta 1521 e serviço `FREEPDB1`. MongoDB usa
`mongo:8.0`, `/data/db` e porta 27017. Novos nomes são `fp-NOME` e
`fp-NOME-data`; configure `--container`, `--volume`, `--image`, `--port` e
`--host` explicitamente quando necessário. Apenas bindings de loopback
`127.0.0.1` ou `::1` são aceitos. Não há alteração de Docker context/host,
permissão de socket, grupos ou sudo automático.

`create` pode baixar a imagem e tem timeout; não inicia o container.
`run` retorna após iniciar, enquanto o servidor inicializa no Docker. Não há
daemon FP ou worker Bash. `--wait` aguarda healthcheck Oracle/ping MongoDB,
com prazo explícito; timeout não remove ou para o banco. Consulte status e logs.
Um pull ou criação parcialmente concluído pode deixar imagem/volume/container;
inspecione e adote explicitamente, sem apagar dados automaticamente.

As senhas são geradas em arquivos privados separados do registro, ou vêm de
variáveis explícitas. São encaminhadas sem aparecer no argv.
Docker mantém variáveis do container em seus próprios metadados; usuários com
acesso ao daemon podem inspecioná-las. Esse acesso equivale a privilégios
administrativos. O FP não implementa um serviço de segredos nem copia credenciais
legadas. `exec` MongoDB autentica automaticamente com o segredo local ou a
variável explicitamente registrada.
`exec` Oracle usa autenticação local SYSDBA **somente para o script solicitado**,
no PDB `FREEPDB1`; use com cuidado. `provision-user` é explícito e idempotente:
cria com quota de 100 MB e grants CREATE SESSION/CREATE TABLE apenas quando o
usuário não existe, sem mudar senha ou grants de usuário existente.

### Legado e exclusão

Nenhuma importação acontece automaticamente. Para adotar Oracle antigo:

```sh
fp db list --discover --output json
fp db create --engine oracle --name oracle-legado --existing \
  --container fiap-oracle --volume fiap-oracle-data \
  --image container-registry.oracle.com/database/free:latest --no-input
```

Para MongoDB, use engine mongodb, `fiap-mongodb`, `fiap-mongodb-data`, `mongo:8.0`,
usuário existente e o nome da variável da senha existente. A adoção exige imagem
e volume compatíveis, registra o ID Docker e não inicia, recria ou provisiona
usuários. SQLite legado pode ser registrado com `--existing --file` apontando
para `~/Work/db/.db/sqlite.db` (expanda `~` no shell). Não é sobrescrito nem migrado.

`init --oracle`, `init --mongodb` e `init --sqlite` agora retornam mensagem de
migração para create/run. `connect ID` consulta logs do registro Docker; para
SQLite retorna orientação para `exec`. `-f ENGINE up` deve ser substituído por
create/run explícitos. Não há fallback ao script original.

```sh
# Esquecer o registro, mantendo arquivo/container/volume:
fp db delete --name estudos --confirm estudos --no-input
# Remover container parado, mantendo seu volume:
fp db delete --name oracle-a --confirm oracle-a --remove-container --no-input
# Apagar dados criados pelo FP, de forma permanente:
fp db delete --name mongo-a --confirm mongo-a --remove-container --delete-data --no-input
fp db delete --name estudos --confirm estudos --delete-data --dry-run --no-input
```

`--confirm` deve ser exatamente o identificador, inclusive em dry-run.
`--delete-data` nunca é implícito e, em Docker, exige `--remove-container`.
O FP recusa apagar dados importados, volumes sem sua label ou containers em
execução; pare-os antes. Não seleciona todos os bancos. Se a remoção Docker
falhar parcialmente, o registro é preservado para diagnóstico; consulte Docker
antes de repetir. Não há transação global entre filesystem, registro e Docker.

### Interface visual

`fp db`, `fp db visual`, `fp db --visual` e `fp database` abrem o mesmo menu
inline Cliclack. Inclui criar/iniciar/parar/listar/status/scripts/excluir,
configuração inicial, logs, descoberta e provisionamento de usuário Oracle.
A criação coleta engine, nome, arquivo ou container/volume/imagem/porta/host,
usuário e **nome da variável da senha**; mostra resumo e confirmação padrão Não.
A exclusão mostra a persistência, pergunta separadamente sobre container/dados e
exige digitar o identificador. Esc/Ctrl+C cancelam. Toda execução chama a mesma
API usada pela CLI declarativa; o núcleo não depende de terminal ou Cliclack.

O gerenciador usa serde, rusqlite com SQLite embarcado, self_update para download,
SHA-256 e tar/gzip. Não há runtime assíncrono.


## `fp init`: bancos sem sudo ou login repetido

A partir de 0.3.0, `fp init` prepara os arquivos acadêmicos e também o runtime
de bancos. Baixa os pacotes estáticos oficiais Docker 29.9.2 e rootless extras,
além de slirp4netns 1.3.7, verificando SHA-256 fixado no código antes de extrair.
Não baixa nem executa um instalador remoto. O launcher local é o oficial do
Docker, incluído no pacote verificado.

O FP cria somente seu serviço `fp-docker.service` em systemd **do usuário**,
habilita-o para as sessões futuras e inicia-o. Não altera o daemon do sistema,
contexto Docker global, grupos, permissões do socket ou shell/PATH do usuário.
Os pacotes ficam em `$XDG_DATA_HOME/fp/runtime/`; imagens e volumes em
`$XDG_DATA_HOME/fp/docker/`. O socket privado fica em
`$XDG_RUNTIME_DIR/fp-docker/docker.sock`. A configuração do daemon é própria,
assim como o diretório de configuração do cliente, sem consultar logins globais.
Os padrões XDG continuam sendo os documentados acima.

O setup baixa antecipadamente `gvenzl/oracle-free:23-slim` e `mongo:8.0`, sem
criar bancos nem containers. Imagens já presentes são reutilizadas, inclusive
sem conexão. O download inicial pode ocupar vários GB; falhas de rede, falta de
espaço ou limites anônimos do registry retornam erro, sem pedir login ou informar
sucesso. Execute `fp init` novamente para continuar.

```sh
fp init
fp db create --engine mongodb --name estudos
fp db run --name estudos
fp db exec --name estudos --file /caminho/consulta.js
fp db stop --name estudos
```

Nenhuma senha precisa ser digitada nesses comandos. O FP gera 32 bytes aleatórios
por recurso e guarda sua representação hexadecimal em arquivos 0600 no diretório
0700 `$XDG_CONFIG_HOME/fp/secrets/`. Os valores não aparecem no registro JSON,
argv, resumos ou logs do FP. **Esses arquivos são protegidos por permissões, não
criptografados em disco**. Inclua-os no backup privado junto aos dados; apagá-los
não redefine a senha no banco. Credenciais também constam nos metadados internos
do Docker, acessíveis a quem controla esse daemon.

Novos recursos registram o socket dedicado. Registros anteriores sem esse campo
continuam usando a conexão Docker anterior e são protegidos pela checagem de ID.
Não há migração automática de dados, troca de senhas ou parada de containers
legados. Dois daemons não podem publicar simultaneamente a mesma porta; use
`--port` para o novo recurso se um banco anterior já ocupar a porta padrão.

O runtime requer Linux x86_64, sessão systemd de usuário, user namespaces,
newuidmap/newgidmap, iptables e ao menos 65536 IDs subordinados em `/etc/subuid`
e `/etc/subgid`. Se um pré-requisito faltar, o FP interrompe **antes dos downloads**
e informa a preparação administrativa única necessária. Nenhum programa em uma
pasta de usuário consegue conceder esses privilégios por conta própria; o FP
não executa sudo, não altera /etc e não enfraquece políticas do kernel.

`fp init --check` verifica sem instalar/iniciar serviços ou baixar imagens.
`fp init --academic-only` mantém o preparo apenas acadêmico. Para preparar bancos
sem tocar no workspace acadêmico, use `fp db init --setup-runtime`; essa operação
também aparece na TUI. `fp db init` isoladamente continua preparando só diretórios.
`--dry-run` com `--setup-runtime` faz apenas a verificação. SQLite funciona até
sem esse setup, pois agora faz parte do binário; aceita SQL, sem metacomandos do
antigo shell sqlite3 como `.shell` ou `.read`.

Para diagnóstico: `systemctl --user status fp-docker.service` e
`journalctl --user -u fp-docker.service`. O serviço acompanha a sessão de usuário;
execução antes do login/apos logout exigiria linger, que o FP não configura.
A arquitetura rootless segue a [documentação Docker](https://docs.docker.com/engine/security/rootless/).
A imagem Oracle pública e suas variáveis seguem a
[documentação gvenzl](https://github.com/gvenzl/oci-oracle-free).

Para adotar explicitamente um container já existente no daemon dedicado, use
`fp db create --existing --rootless --engine ENGINE --name NOME` com imagem,
container e volume corretos. Sem `--rootless`, adoções continuam consultando
a conexão Docker anterior, preservando a compatibilidade com os bancos legados.
