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
cargo install --path . --locked
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
workspaces Cargo/uv/JavaScript ou credenciais. Use `fp create` para criar projetos.
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
Capítulo → fase 3 → 2 gera `fase-03/cap-02`. O nome registrado e o nome passado
a uv/Cargo vêm do diretório gerado. Uma fase cria somente uma pasta.
`fp create` cria projetos. `fp init` prepara e verifica o workspace acadêmico.

A numeração sugerida é o maior número existente + 1; você pode informar outro
número positivo disponível. Sem fases, crie uma Fase primeiro. Não há alterações
antes da confirmação. Esc/Ctrl+C retornam 130; recusar a confirmação retorna 0.

A descoberta segue `fase-NN/cap-NN`, com ordenação numérica. Novos nomes contêm
somente prefixo e número, sem sufixos descritivos. A largura existente é preservada
(por exemplo, `cap-009` leva a `cap-010`), com dois dígitos por padrão. Diretórios
antigos com sufixos continuam reconhecidos, sem renomeação. O capítulo recebe o
nome gerado em `chapters.toml`. Uma fase não ganha metadados extras.
`docs/fase-NN` contém materiais e não conta como fase de projetos.
Numerações duplicadas são reportadas como ambíguas; links de diretórios não são
atravessados na descoberta.

### Tecnologias disponíveis

| Opção | Inicialização | Execução registrada |
| --- | --- | --- |
| Python | `uv init --app --no-package --no-workspace --vcs none …` | `uv run main.py` |
| TypeScript / JavaScript | `vp create vite:application --directory . --no-git` | `vp run dev --` se existir script `dev` |
| Rust | `cargo init --bin --vcs none --name cap-01 .` | `cargo run --` se existirem manifest e `src/main.rs` |
| ESP32 / IoT | `pio project init --board BOARD` | Nenhuma operação de hardware automática |
| SQL / Database | Pasta `sql/` vazia | Nenhum |
| Somente pasta | Diretório vazio | Nenhum |

Python cria uma aplicação simples pelo uv (`--app --no-package`), com `pyproject.toml` e `main.py`, sem criar
`.venv` ou workspace durante a inicialização. O `requires-python` do
`pyproject.toml` da raiz é passado ao uv; downloads implícitos de Python e
pinagem de versão ficam desativados. O alvo `main` é registrado automaticamente
como `uv run main.py`, permitindo `fp run FASE CAPITULO` imediatamente após a
criação. Se o inicializador terminar sem gerar `main.py`, o FP cancela a
publicação com erro, sem registrar um capítulo incompleto.

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
preservando outros capítulos, o Justfile e materiais em `docs/`. Pastas da
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
`argv = ["uv", "run", "main.py"]`. Só registre esse comando após criar o arquivo.
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
com verificação de alterações externas. Falhas normais removem somente o novo
destino criado pela operação. Diretórios existentes nunca são substituídos.

O FP não cria workspaces de pacotes, commits, configurações globais ou atalhos do Omarchy.

## Zed

Depois de criar, escolha Sim para executar `zed CAMINHO` diretamente, sem `--wait`.
Se o editor não estiver disponível, a criação permanece válida e uma mensagem é
exibida. Não há dependência de aliases ou funções Fish.

## Organização do código

```text
Cargo.toml / Cargo.lock       dependências e release reproduzível com --locked
src/main.rs                  entrada e códigos de saída
src/cli.rs                   argumentos Clap
src/root.rs                  resolução de caminhos e validação da raiz
src/workspace.rs             bootstrap e diagnóstico de PATH
src/update.rs                consulta e atualização standalone pelo GitHub
assets/workspace/            arquivos portáveis embutidos no binário
src/discovery.rs             numeração, nomes e manifests
src/tui.rs                   fluxo Cliclack e listagem
src/technology_menu.rs       disponibilidade e seletor com itens desabilitados
src/create.rs                  plano, inicializadores e publicação
src/delete.rs                TUI e exclusão com atualização do registro
src/registry.rs              edição preservando chapters.toml
src/processes.rs             processos, Just e Zed
tests/contracts.rs           testes Cargo isolados
tests/delete.rs              exclusão, cancelamento e preservação de arquivos
tests/workspace.rs           bootstrap, idempotência e diagnóstico
tools/check_tui.py            smoke tests opcionais em pseudoterminal
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
cargo install --path . --release --locked
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
- O contrato de integração é o `chapters.toml` FIAP. Receitas Just arbitrárias,
  templates remotos personalizados e workspaces não são gerenciados.
- A execução Python existente pode usar a `.venv` compartilhada do uv conforme o
  dispatcher; o FP não muda essa política. Projetos independentes mantêm manifests
  locais e não ganham ambiente até sua ferramenta precisar dele.
- Melhorias possíveis: validar versões suportadas dos inicializadores e ampliar
  testes em ambientes com Vite+, hardware PlatformIO e Zed reais.

## Wrapper do banco local: `fp db`

```sh
fp db init --oracle
fp db init --mongodb
fp db init --sqlite
fp db list
fp db connect sqlite
fp db connect oracle
fp db --help
```

Executa o `db` disponível no PATH, herdando terminal, diretório atual e código
de saída. Não exige uma raiz acadêmica válida. Se necessário, `FP_DB_BIN` pode
apontar para outro executável `db`; não usa aliases ou funções do shell.
`fp db --help` mostra a ajuda original do db.

O wrapper define `DB_EXECUTION_PROFILE=user`. O db atual usa as permissões do
usuário, respeita o contexto/host Docker existente e não chama sudo
automaticamente. A configuração única do grupo e serviço Docker está documentada
em [~/Work/db/README.md](../db/README.md). É preciso entrar novamente na sessão
para carregar o grupo Docker. Containers, volumes e SQLite continuam gerenciados
pelo db original. Bash é uma dependência do script db; o executável FP continua
nativo e não incorpora outro executor de bancos.

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

Para Cargo, repita `cargo install --path . --force --locked` no checkout atualizado,
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

A única fonte da versão é `package.version` em `Cargo.toml`; Clap incorpora essa
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
cargo test --locked update::tests
python3 tools/test_release.py
python3 tools/check_release.py v0.1.0 # ajuste se a versão mudar
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
