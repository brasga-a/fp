# Changelog

As mudanças do FP seguem a política de versionamento semântico descrita em
[CONTRIBUTING.md](CONTRIBUTING.md). A versão do executável vem de `Cargo.toml`.

## [Unreleased]

### Adicionado

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
