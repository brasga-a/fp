# Contribuindo com o FP

O FP é uma CLI pequena para produtividade acadêmica. Mudanças devem preservar
os projetos do usuário e aproveitar as ferramentas existentes de cada ecossistema.
Leia o [README](README.md) e as [instruções para agentes](AGENTS.md).

## Preparar o ambiente

Use Rust stable recente, Cargo, rustfmt, Clippy, Just e Python 3.14+.
Use Just 1.39+ (o CI fixa 1.59.0). Python e Just são usados nos testes do dispatcher e nos testes de terminal;
não são runtimes do executável FP. Ferramentas como uv, vp, PlatformIO e Zed
são simuladas nos testes: não é necessário instalar todos os ecossistemas.

```sh
git clone https://github.com/brasga-a/fp.git
cd fp
cargo build --locked
cargo run -- --help
```

## Desenvolver e testar

Faça mudanças focadas, com módulos pequenos e sem introduzir um sistema de
plugins, package manager ou executor próprio. A execução de capítulos continua
sendo responsabilidade do Just e de `chapters.toml`.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --release --locked
python3 tools/check_tui.py target/release/fp
python3 tools/check_delete_tui.py target/release/fp
```

Use apenas repositórios temporários nos testes. Não modifique `~/fiap`, execute
projetos acadêmicos reais, inicialize projetos com ferramentas reais, acesse
hardware ou inicie bancos como parte dos testes automatizados. Cubra mudanças
em argumentos, registro e processos com executáveis simulados.

Ao modificar o bootstrap, mantenha os arquivos de `assets/workspace/` portáveis
para qualquer home e confirme que o Just funciona sem o FP instalado. Ao mudar
prompts, valide setas, Enter, Esc, Ctrl+C, cancelamento sem alterações e opções
desabilitadas. As mensagens da CLI são em português. Os arquivos `AGENTS.md`
são em inglês e devem instruir explicitamente respostas ao usuário em português.

## Propor uma mudança

Descreva o problema, o comportamento final e como foi validado. Para correções,
inclua um exemplo reproduzível. Explique dependências novas e limitações reais.
Preserve alterações não relacionadas, configurações existentes e autoria.
Não inclua credenciais, dumps, ambientes virtuais ou artefatos de build.

## Versionamento

O projeto usa versionamento semântico: `MAJOR.MINOR.PATCH`.

- `PATCH`: correções compatíveis, como ajuste de diagnóstico ou tratamento de erro.
- `MINOR`: funcionalidades novas compatíveis. Antes de 1.0, mudanças incompatíveis
  também incrementam MINOR e devem ser destacadas no changelog.
- `MAJOR`: mudanças incompatíveis a partir da versão 1.0.

A única fonte da versão do executável é `workspace.package.version` em `Cargo.toml`.
Clap usa essa versão automaticamente em `fp --version` e `fp -V`.
Não duplique uma constante de versão em `src/`. Versione `Cargo.lock`.
Registre mudanças em `[Unreleased]` no [CHANGELOG.md](CHANGELOG.md); ao preparar
uma release, mova-as para uma seção com número e data.

Para atualizar a versão, edite `Cargo.toml` e execute `cargo check` para atualizar
a entrada do pacote em `Cargo.lock`. Não atualize todas as dependências apenas
para mudar a versão do FP. Atualize o changelog e rode as validações acima.

## Publicar uma versão

Depois de revisar e commitar os arquivos da release:

```sh
cargo build --release --locked
./target/release/fp --version
```

Confira que a saída, `Cargo.toml`, a entrada `fiap-projects` em `Cargo.lock` e a
seção do changelog têm a mesma versão. A tag Git deve ter o prefixo `v`:

```sh
# Exemplo para uma release 0.1.0; não recrie se a tag já existir.
git tag -a v0.1.0 -m "FP 0.1.0"
git push --atomic origin main v0.1.0
```

Nunca mova ou sobrescreva tags publicadas. Se precisar corrigir uma release,
publique outra versão. O workflow [release.yml](.github/workflows/release.yml) valida a tag contra
`Cargo.toml`, executa fmt/Clippy/testes e publica os binários e `SHA256SUMS` no
GitHub Releases. Ele não publica no crates.io. `v0.1.0` já existe: não reutilize
essa tag para incluir o novo workflow; prepare uma nova versão (por exemplo,
`0.2.0`, por adicionar comandos) antes de publicar esses recursos.

Valide a automação localmente com `python3 tools/test_release.py` e
`python3 tools/check_release.py vVERSAO` (substitua pela versão do Cargo).
Não há incremento automático, criação de tags ou push pelo workflow.
O token padrão do Actions recebe escrita somente no job de publicação.
Se um upload falhar, a release fica em rascunho; inspecione e conclua ou remova
o rascunho manualmente antes de reexecutar. Releases existentes não são sobrescritas.

## Instalar uma versão publicada

```sh
cargo install --git https://github.com/brasga-a/fp.git --tag v0.1.0 --locked
```

Para desenvolvimento local, use `cargo install --path crates/fp-cli --locked`.

## Workspace e bancos

Os quatro crates ficam em `crates/`; só `fp-cli` depende das bibliotecas
funcionais. Bibliotecas não dependem de Cliclack nem da CLI. A versão é herdada
de `[workspace.package]`, incluindo o pacote `fiap-projects`. A raiz é virtual;
instalação local: `cargo install --path crates/fp-cli --root ~/.local --force --locked`
quando esse for o prefixo original. Não instale durante testes.

Testes de bancos usam `Manager::new(Paths { ... })` e executáveis simulados,
sem Docker real ou credenciais reais. Execute também
`python3 tools/check_database_tui.py target/release/fp`. Agentes devem usar
subcomandos completos com `--no-input --output json` e diretórios isolados;
nunca navegar pela TUI. Novas operações devem compartilhar o mesmo núcleo nas
duas interfaces. Preserve códigos `E_*`, stdout JSON limpo, dry-run e confirmação
por identificador. Não execute o script legado nem altere seus dados.


Em 0.3.0, `fp init` também prepara o runtime rootless. Nos testes acadêmicos use
`--academic-only`; testes do bootstrap devem injetar `SetupHost` e caminhos
`RuntimeLocation` temporários. Nunca chamem systemd/Docker reais ou baixem imagens.
SQLite é embarcado: teste SQL real em arquivos temporários, sem mock de sqlite3.
Ao atualizar pacotes rootless, revise a origem e fixe os novos hashes em
`crates/fp-database/src/runtime/linux.rs`; não use URLs ou versões fornecidas por entrada
arbitrária. Credenciais locais são privadas, não criptografadas: não as inclua
em fixtures, snapshots, commits ou logs. Apenas novas instâncias podem receber
segredos automáticos; adoções preservam as credenciais e o daemon existentes.


Para retomar uma publicação após corrigir apenas o workflow, abra Actions →
Release → Run workflow na branch main e informe a tag já existente. A execução
manual faz checkout de `refs/tags/TAG`, valida Cargo e compila o código exato da
tag. Não mova tags para corrigir infraestrutura de CI. Python 3.14 é selecionado
explicitamente no runner Ubuntu 22.04, cujo Python de sistema é 3.10.

## Portabilidade Linux / Windows

Preserve os quatro crates e use a biblioteca padrão multiplataforma primeiro.
Isole APIs Unix compartilhadas com `#[cfg(unix)]`, bootstrap Linux/systemd com
`#[cfg(target_os = "linux")]` e Win32/DPAPI com `#[cfg(windows)]`. `cfg!` não
esconde imports ou tipos inválidos. Não crie uma camada genérica de plataforma.
As únicas dependências diretas adicionais desta implementação são `getrandom`
(já transitiva; aleatoriedade do sistema) e `windows-sys` sob target Windows
(já transitiva; APIs estáveis de identidade e DPAPI). O SQLite continua sendo
`rusqlite` com `bundled`; `self_update` usa HTTPS/rustls sem OpenSSL do sistema.
Não há novos crates funcionais ou bibliotecas substituídas.

No Windows, instale Rust MSVC, Build Tools C++/Windows SDK, Just e Python 3.14+
com o comando `python` no PATH. O executável usa VCRUNTIME140/UCRT padrão do
MSVC; verifique também uma instalação standalone com o redistributable x64
apropriado antes da primeira release. Não use WSL como prova de execução Windows.
Execute nativamente:

```powershell
cargo fmt --all --check
cargo check --workspace --locked --target x86_64-pc-windows-msvc
cargo clippy --workspace --locked --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo test --workspace --locked --target x86_64-pc-windows-msvc
cargo build --release --locked --target x86_64-pc-windows-msvc
python -m unittest discover -s assets/workspace/tests -v
```

O workflow `ci.yml` executa esses comandos nativamente nos runners Ubuntu e
Windows e guarda os dois pacotes com checksums por target, sem publicar releases.
`release.yml` reutiliza a matriz para uma tag existente validada, combina os
checksums e mantém publicação em rascunho, recusa de releases existentes e
permissão de escrita apenas no job de publicação. CI manual valida a revisão
já disponível no GitHub; não consegue executar alterações ainda apenas locais.
Nunca faça commit/push apenas para dispará-lo sem autorização.

Testes que usam Bash, sinais ou permissões POSIX têm atributos individuais.
As suítes compartilhadas mantêm parsing, diretórios, criação, rollback, SQLite,
identidade persistida e JSON sem TTY. Executáveis Rust simulados cobrem
inicializadores nativos, streams e códigos de saída. Só a suíte do bootstrap
rootless é inteiramente Linux, por testar exclusivamente esse domínio. Testes
Windows adicionais cobrem batch, DPAPI, registros incompatíveis e recusa de
bootstrap. Não execute Docker real. Os três testes PTY são Linux, pois usam PTYs
POSIX; isso não comprova a experiência visual num console Windows.

Não declare suporte Windows completo antes de registrar a execução nativa em
VALIDATION.md, com link do Actions e resultados. Antes da primeira distribuição,
valide também um terminal Windows, os inicializadores opcionais reais e Docker
Desktop com Linux containers em recursos descartáveis autorizados. A atualização
Windows permanece manual; não implemente substituição insegura para obter paridade.

As APIs Win32 usadas são documentadas pela Microsoft:
[GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex)
e [DPAPI](https://learn.microsoft.com/en-us/windows/win32/seccrypto/data-protection).
O Just usa a [configuração de interpretador](https://just.systems/man/en/settings.html)
e argumentos posicionais, sem interpolar os argumentos do usuário em código.
