# FP contributor instructions

## Language

- **Always respond to the user in Brazilian Portuguese (pt-BR)**, even though
  these instructions are written in English, unless the user explicitly requests
  another language.
- Keep user-facing CLI messages and project documentation in Portuguese unless
  a task explicitly requires another language. Keep AGENTS.md instructions in English.

## Scope and architecture

- Read README.md and CONTRIBUTING.md before making changes.
- FP is a small synchronous Rust CLI. Use Clap for arguments, Cliclack for inline
  prompts and std::process::Command for external tools. Keep modules explicit.
- `fp init` prepares and diagnoses the academic repository; `fp create` creates
  numbered phases and chapters; `fp delete` requires interactive confirmation.
- Just and chapters.toml remain the source of execution commands. Do not implement
  a second project runner, package manager, build system or plugin framework.
- Preserve stdin/stdout/stderr and external exit codes. Pass arguments directly,
  without interpolating them into shell commands.
- New phases and chapters have generated numeric names. Unavailable technologies
  remain visible but disabled in the create menu.

## Data and configuration

- Preserve existing academic files, PDFs, manifests, lockfiles and custom settings.
  Bootstrap only missing support files; never overwrite existing AGENTS.md files.
- The academic root defaults to ~/fiap, with --root taking precedence over FP_ROOT.
  Do not hardcode user names or absolute home paths.
- Do not modify the real academic repository during automated tests. Use temporary
  directories and simulated executables; never initialize real ecosystem projects
  during automated tests or access real databases/hardware.
- Keep workspace templates in assets/workspace portable and self-contained.
  Preserve their dispatcher contract and its tests.
- Do not change global configuration, shell shortcuts, Docker permissions or
  credentials unless the user explicitly requests that work. Do not use sudo
  automatically or download/run remote scripts silently.

## Validation and releases

- Run cargo fmt --check, cargo clippy --all-targets -- -D warnings,
  cargo test --locked and cargo build --release --locked for code changes.
- Exercise tools/check_tui.py and tools/check_delete_tui.py after changing prompts.
  Report what was validated and any remaining limitations.
- Cargo.toml is the single source of the CLI version; Clap reads that version.
  Keep Cargo.lock, CHANGELOG.md and Git release tags consistent. Follow the
  SemVer policy in CONTRIBUTING.md and never move an existing release tag.
- Do not commit or push unless explicitly requested. Never force-push, discard
  unrelated work or replace remote history to publish a change.
