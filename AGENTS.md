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

- Run cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings,
  cargo test --workspace --locked and cargo build --release --locked for code changes.
- Exercise tools/check_tui.py and tools/check_delete_tui.py after changing prompts.
  Report what was validated and any remaining limitations.
- workspace.package.version in Cargo.toml is the single source of the CLI version; Clap reads that version.
  Keep Cargo.lock, CHANGELOG.md and Git release tags consistent. Follow the
  SemVer policy in CONTRIBUTING.md and never move an existing release tag.
- Do not commit or push unless explicitly requested. Never force-push, discard
  unrelated work or replace remote history to publish a change.

## Changelog and CLI version routine

- For every meaningful change, update CHANGELOG.md in the same task. Write in
  Portuguese, describe the resulting behavior and group entries under Added,
  Changed, Fixed or Removed (using Portuguese headings). Preserve past entries.
- Record ongoing work under `[Unreleased]`. Before delivering a new CLI version,
  move its entries into a dated `[X.Y.Z]` section and retain `[Unreleased]` for
  subsequent work. A version entry does not require a Git tag or GitHub Release.
- For user-visible CLI features or fixes, increment `workspace.package.version` in
  Cargo.toml according to CONTRIBUTING.md: PATCH for compatible fixes, MINOR for
  new features (and breaking changes before 1.0), MAJOR for breaking changes
  from 1.0 onward. Group related changes into one version increment per delivery.
- Documentation-only and test-only changes do not require a CLI version bump.
  Describe them in the changelog when they affect the contributor workflow.
- After changing the version, run `cargo check` to refresh the package entry in
  Cargo.lock without broadly updating dependencies. Keep the manifest, lockfile
  and dated changelog section consistent; maintain changelog comparison links
  using existing refs only. Do not duplicate the CLI version in Rust constants.
- Run the required code validations above and check
  `./target/release/fp --version` against Cargo.toml before delivering a changed
  executable. Keep version tests and fixtures independent of the current package
  version so future increments do not break simulated update tests.
- Local Cargo updates do not require GitHub Actions. When installation is
  requested, use `cargo install --path crates/fp-cli --force --locked`, preserving the
  original install prefix (`--root ~/.local` when applicable), and verify the
  actual `fp` resolved through PATH. Do not replace Cargo-managed files manually.
- Incrementing a CLI version or updating its changelog does not authorize
  commits, pushes, tags or releases. Create and push `vX.Y.Z` tags only when
  explicitly requested; routine development pushes must not trigger releases.

## Workspace and native database manager

- Keep exactly the four functional crates in crates/: fp-cli (package
  fiap-projects), fp-workspace, fp-database and fp-update. Functional libraries
  must not depend on fp-cli, Cliclack or terminal prompts. UI belongs in fp-cli.
- Prefer explicit database commands with --no-input --output json for agents.
  Never instruct automation to navigate the TUI. Both interfaces must call the
  same fp-database API, with structured arguments and explicit results.
- init only configures directories; create provisions, run only starts existing
  resources. Never invoke the legacy db script or FP_DB_BIN as a fallback.
- Keep Linux XDG directories; Windows uses APPDATA for config and LOCALAPPDATA
  for SQLite/state. Tests must override Paths and simulate Docker/clients.
  Never touch real containers, volumes, legacy .db files or installed fp in tests.
- Require --confirm ID for deletion. Removing a registry entry, removing a
  container and deleting persistent data are separate choices. Imported data
  must be preserved. Validate resource identity before destructive operations.
- Credentials use generated private per-resource files or explicit environment variables, never registry plaintext.
  Do not modify Docker context, socket permissions, groups or invoke sudo.
- Check all three PTY scripts after UI changes; test release paths from the
  virtual manifest and keep workspace-inherited versions consistent.

## Rootless bootstrap

- On Linux, fp init bootstraps the dedicated database runtime unless --academic-only
  is selected. init --check is read-only. Database init without --setup-runtime
  still only prepares directories. Reflect this distinction in help and tests.
- Download only pinned official Docker/rootless packages and slirp4netns with
  SHA-256 verification before extraction. Never pipe remote installers to a shell.
- Use only fp-docker.service in the user manager, with dedicated data/config,
  client configuration and runtime socket. Never change global Docker context,
  system daemon, groups, subuid/subgid, kernel policy or invoke sudo automatically.
- Generate credentials for new resources only; keep secrets in a private 0700
  directory and 0600 regular files, never JSON registry, argv or diagnostics.
  Missing secrets must fail, never rotate credentials in an existing database.
- Persist the endpoint for new rootless resources; preserve the original daemon
  for legacy records. Never migrate existing volumes or passwords automatically.
- SQLite is bundled through rusqlite; tests must not rely on a sqlite3 executable.
  Bootstrap tests inject SetupHost and temporary RuntimeLocation, never real
  systemd, downloads, databases or global user configuration.
- Missing host prerequisites require a clear one-time administrative instruction;
  never bypass namespaces/security policy to make bootstrap appear successful.

## Native Windows portability

- Keep one Cargo workspace with exactly four functional crates and one final
  executable: fp on Linux, fp.exe on Windows x86_64 MSVC. Version 0.6.0 introduces
  the Windows implementation, validated in native CI. Keep native Windows CI
  mandatory for platform changes and report outstanding manual validations.
- Prefer standard cross-platform APIs. Use cfg attributes around real platform
  differences: target_os = "linux" for rootless/systemd, windows for Win32/DPAPI,
  unix only for shared Unix APIs. Do not use cfg! to hide invalid code.
- Credentials live in credentials.rs/credentials/windows.rs, independently of
  runtime/linux.rs. Preserve POSIX protection and user-scoped DPAPI protection.
  Never regenerate an existing database's missing password. No plaintext in
  registry, process arguments or diagnostics.
- Preserve legacy Unix file_identity serialization. Windows uses a separate
  optional windows_file_identity (volume and 128-bit file ID). Refuse operations
  when identity cannot be proven. Never substitute filename-only comparisons.
- Windows init prepares academic files without Docker bootstrap; init --check
  stays read-only. setup-runtime must fail clearly and suggest Docker Desktop.
  Respect user Docker contexts and require Linux containers for Oracle/MongoDB.
  Never translate persisted unix:// endpoints or migrate volumes automatically.
- Windows updater checks official release metadata and selects ZIP/fp.exe, but
  automatic installation is explicitly unavailable. Preserve safe Linux updates
  and Cargo-managed installations. Document manual Windows installation.
- Templates use Just with Python as recipe interpreter (python3 on Linux,
  python on Windows). Preserve literal argv, stdin/out/err and exit codes.
  Unix dispatcher keeps execvp; Windows waits for subprocess completion.
  Reject unsafe batch arguments; never interpolate them into shell strings.
- Gate POSIX tests individually, keep shared tests active, and use native mock
  executables. Only rootless-bootstrap tests are a Linux-only suite. Never run
  real Docker or initialize real ecosystem projects in automated tests.
- Run CI natively on Ubuntu and Windows MSVC. Cross-compilation only proves
  compilation, not execution. Report unexecuted Windows checks explicitly.
  Configuring a workflow does not mean it passed. Do not commit/push merely to
  trigger CI without authorization. Ordinary CI must not publish releases.
