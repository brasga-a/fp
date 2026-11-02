//! Atualizações independentes da raiz acadêmica, restritas à instalação standalone.
use crate::Result;
use self_update::{
    ReleaseAsset,
    backends::github::{Update, UpdateBuilder},
};
use semver::Version;
use std::{
    fs,
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const TARGET: &str = "x86_64-unknown-linux-gnu";
const MARKER: &str = "fp-standalone: https://github.com/brasga-a/fp\n";
const API: &str = "https://api.github.com/repos/brasga-a/fp/releases/";

// Todas as requisições iniciais ficam no namespace oficial. O cliente TLS segue
// os redirecionamentos de assets do GitHub para seu armazenamento de releases.
struct OfficialClient;
impl self_update::http_client::HttpClient for OfficialClient {
    fn get(
        &self,
        url: &str,
        headers: &self_update::http_client::HeaderMap,
        timeout: Option<Duration>,
    ) -> self_update::Result<Box<dyn self_update::http_client::HttpResponse>> {
        if !url.starts_with(API) {
            return Err(self_update::Error::verification_rejected(
                "Origem de download não autorizada",
            ));
        }
        let config = self_update::ureq::Agent::config_builder()
            .https_only(true)
            .timeout_global(timeout)
            .http_status_as_error(false)
            .build();
        self_update::http_client::UreqClient::from(self_update::ureq::Agent::new_with_config(
            config,
        ))
        .get(url, headers, timeout)
    }
}

fn builder() -> UpdateBuilder {
    let mut builder = Update::configure();
    builder
        .repo_owner("brasga-a")
        .repo_name("fp")
        .tag_prefix("v")
        .bin_name("fp")
        .bin_path_in_archive("fp")
        .current_version(env!("CARGO_PKG_VERSION"))
        .target(TARGET)
        .timeout(Duration::from_secs(60))
        .retries(0)
        .show_output(false)
        .show_download_progress(false)
        .no_confirm(true)
        .http_client(Arc::new(OfficialClient));
    builder
}

fn stable_version(tag: &str) -> Result<Version> {
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|_| format!("Versão inválida na release: {tag}"))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(format!("Release fora do canal estável: {tag}").into());
    }
    Ok(version)
}

fn newer(current: &str, latest: &str) -> Result<bool> {
    Ok(stable_version(latest)? > stable_version(current)?)
}

fn artifact(version: &Version, target: &str) -> String {
    format!("fp-v{version}-{target}.tar.gz")
}

fn select_asset(assets: &[ReleaseAsset], name: &str) -> Option<ReleaseAsset> {
    let mut matches = assets
        .iter()
        .filter(|a| a.name() == name && a.download_url().starts_with(API));
    let found = matches.next()?.clone();
    matches.next().is_none().then_some(found)
}

fn explain(error: self_update::Error) -> Box<dyn std::error::Error> {
    use self_update::Error::*;
    let message = match &error {
        RateLimited { .. } => "Limite da API do GitHub atingido; tente novamente mais tarde",
        NotFound { .. } | NoReleaseFound { .. } => {
            "Release ou artefato não encontrado; confira as releases oficiais no GitHub"
        }
        SemVer(_) => "Versão inválida na release; reporte o problema no repositório oficial",
        Transport(_) => "Falha de conexão; verifique a rede e tente novamente",
        ChecksumMismatch { .. } => {
            "SHA-256 inválido; atualização rejeitada. Tente novamente ou reporte a release"
        }
        ChecksumSourceInvalid { .. } => {
            "Checksum ausente ou inválido; atualização rejeitada. Reporte a release"
        }
        InstallPathNotWritable { .. } => {
            "Instalação sem permissão de escrita; atualize pelo método original"
        }
        Unauthorized { .. } | HttpStatus { .. } => {
            "A API do GitHub recusou a consulta; tente novamente mais tarde"
        }
        _ => return format!("Atualização não concluída: {error}").into(),
    };
    message.into()
}

// Um marcador explícito evita inferir a origem de um executável apenas pelo PATH.
// Não admitimos symlinks (inclusive nos ancestrais), hardlinks ou locais de Cargo.
#[cfg(unix)]
fn standalone(exe: &Path, home: &Path, cargo_home: Option<&Path>) -> Result<fs::Permissions> {
    use std::os::unix::fs::MetadataExt;
    let expected = home.join(".local/bin/fp");
    let refuse = "Instalação gerenciada ou de origem incerta; atualize pelo método original. Consulte o README para instalação standalone";
    if exe != expected || fs::canonicalize(&expected)? != expected {
        return Err(refuse.into());
    }
    if cargo_home.is_some_and(|p| exe.starts_with(p))
        || home.join(".local/.crates.toml").exists()
        || home.join(".local/.crates2.json").exists()
    {
        return Err("Instalação Cargo; use cargo install pelo mesmo caminho/repositório, com --force --locked".into());
    }
    let marker = expected.with_file_name(".fp-standalone");
    let uid = fs::metadata(home)?.uid();
    for path in [
        &expected,
        expected.parent().unwrap(),
        &home.join(".local"),
        home,
        &marker,
    ] {
        let meta = fs::symlink_metadata(path).map_err(|_| refuse)?;
        if meta.file_type().is_symlink() || meta.uid() != uid || meta.mode() & 0o6022 != 0 {
            return Err(refuse.into());
        }
        if meta.mode() & 0o200 == 0 {
            return Err(
                "Instalação sem permissão de escrita; atualize pelo método original".into(),
            );
        }
    }
    let meta = fs::metadata(&expected)?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.mode() & 0o100 == 0
        || fs::read_to_string(marker)? != MARKER
    {
        return Err(refuse.into());
    }
    Ok(meta.permissions())
}

#[cfg(not(unix))]
fn standalone(_: &Path, _: &Path, _: Option<&Path>) -> Result<fs::Permissions> {
    Err("Atualização direta disponível somente para Linux x86_64 GNU".into())
}

fn unchanged(exe: &Path, original: &fs::Metadata) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let now = fs::symlink_metadata(exe)?;
    if now.dev() != original.dev()
        || now.ino() != original.ino()
        || now.len() != original.len()
        || now.modified()? != original.modified()?
    {
        return Err(
            "Instalação alterada durante a atualização; execute fp update novamente".into(),
        );
    }
    Ok(())
}

fn install_builder(version: &Version, exe: &Path, permissions: fs::Permissions) -> UpdateBuilder {
    let original = fs::metadata(exe).ok();
    let install_path = exe.to_path_buf();
    let name = artifact(version, TARGET);
    let expected_output = format!("fp {version}\n");
    let mut config = builder();
    config
        .release_tag(format!("v{version}"))
        .bin_install_path(exe)
        .checksum_from_asset("SHA256SUMS")
        .asset_matcher(move |assets| select_asset(assets, &name))
        .verify_binary(move |new_exe| {
            // Não execute um arquivo arbitrário/malformado. Primeiro valide ELF64 LE x86_64.
            let mut header = [0u8; 20];
            let meta = fs::symlink_metadata(new_exe)?;
            if !meta.is_file() || meta.len() < 64 {
                return Err(self_update::Error::verification_rejected(
                    "Binário inválido",
                ));
            }
            fs::File::open(new_exe)?.read_exact(&mut header)?;
            if &header[..6] != b"\x7fELF\x02\x01" || header[18..20] != [62, 0] {
                return Err(self_update::Error::verification_rejected(
                    "Binário incompatível com Linux x86_64",
                ));
            }
            fs::set_permissions(new_exe, permissions.clone())?;
            let output = std::process::Command::new(new_exe)
                .arg("--version")
                .output()?;
            if !output.status.success() || output.stdout != expected_output.as_bytes() {
                return Err(self_update::Error::verification_rejected(
                    "Versão do binário difere da release",
                ));
            }
            if let Some(original) = &original {
                unchanged(&install_path, original)
                    .map_err(|e| self_update::Error::verification_rejected(e.to_string()))?;
            }
            Ok(())
        });
    config
}

// A confirmação e a instalação são injetáveis para testar cancelamento e falhas
// sem jamais substituir o executável dos testes.
fn confirmed_install(
    confirm: impl FnOnce() -> Result<bool>,
    install: impl FnOnce() -> Result<()>,
) -> Result<bool> {
    if !confirm()? {
        return Ok(false);
    }
    install()?;
    Ok(true)
}

pub fn run(check: bool) -> Result<i32> {
    let releases = builder().build()?.get_latest_release().map_err(explain)?;
    let latest = releases
        .latest()
        .ok_or("Nenhuma release estável publicada")?;
    let version = stable_version(latest.version())?;
    let current = env!("CARGO_PKG_VERSION");
    println!("Versão instalada: {current}\nÚltima versão estável: {version}");
    if !newer(current, latest.version())? {
        println!("O FP já está atualizado.");
        return Ok(0);
    }
    println!("Atualização disponível: v{version}");
    if check {
        return Ok(0);
    }
    if self_update::get_target() != TARGET {
        return Err(format!(
            "Sem distribuição compatível com {}",
            self_update::get_target()
        )
        .into());
    }
    if select_asset(latest.assets(), &artifact(&version, TARGET)).is_none() {
        return Err(format!("Release sem artefato compatível: {TARGET}").into());
    }
    let exe = std::env::current_exe()?;
    let home =
        PathBuf::from(std::env::var_os("HOME").ok_or("HOME não definido; atualize manualmente")?);
    let cargo_home = std::env::var_os("CARGO_HOME").map(PathBuf::from);
    let permissions = standalone(&exe, &home, cargo_home.as_deref())?;
    let identity = fs::metadata(&exe)?;
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Err("Execute fp update em um terminal para confirmar a atualização".into());
    }
    cliclack::intro("FP · Atualizar")?;
    let installed = confirmed_install(
        || {
            Ok(
                cliclack::confirm(format!("Atualizar de v{current} para v{version}?"))
                    .initial_value(false)
                    .interact()?,
            )
        },
        || {
            // Revalide após o prompt; nenhuma mudança da instalação antes da confirmação.
            standalone(&exe, &home, cargo_home.as_deref())?;
            unchanged(&exe, &identity)?;
            let spinner = cliclack::spinner();
            spinner.start("Baixando, verificando SHA-256 e instalando…");
            let outcome = install_builder(&version, &exe, permissions)
                .build()?
                .update()
                .map_err(explain);
            match outcome {
                Ok(status) if status.is_updated() => {
                    spinner.stop("Instalação concluída");
                    Ok(())
                }
                Ok(_) => {
                    spinner.stop("Nenhuma instalação realizada");
                    Err("A release mudou; execute fp update novamente".into())
                }
                Err(error) => {
                    spinner.stop("Atualização não concluída");
                    Err(error)
                }
            }
        },
    )?;
    if installed {
        cliclack::outro(format!("FP atualizado para v{version}!"))?;
    } else {
        cliclack::outro_cancel("Atualização cancelada; nenhum arquivo alterado")?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use self_update::http_client::{HeaderMap, HttpClient, HttpResponse};
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::{collections::HashMap, io::Cursor, sync::Mutex};

    struct Response(Vec<u8>, HeaderMap);
    impl HttpResponse for Response {
        fn headers(&self) -> &HeaderMap {
            &self.1
        }
        fn body(self: Box<Self>) -> Box<dyn Read> {
            Box::new(Cursor::new(self.0))
        }
    }
    #[derive(Default)]
    struct Mock {
        bodies: HashMap<String, Vec<u8>>,
        calls: Mutex<Vec<String>>,
    }
    impl HttpClient for Mock {
        fn get(
            &self,
            url: &str,
            _: &HeaderMap,
            _: Option<Duration>,
        ) -> self_update::Result<Box<dyn HttpResponse>> {
            self.calls.lock().unwrap().push(url.into());
            let body = self.bodies.get(url).ok_or_else(|| {
                self_update::Error::Transport(
                    std::io::Error::new(std::io::ErrorKind::NotConnected, "rede simulada").into(),
                )
            })?;
            Ok(Box::new(Response(body.clone(), HeaderMap::new())))
        }
    }
    fn mock(sums: Option<&[u8]>, archive: &[u8]) -> Arc<Mock> {
        let mut mock = Mock::default();
        let mut assets = vec![
            serde_json::json!({"name":artifact(&stable_version("0.2.0").unwrap(), TARGET),"url":format!("{API}assets/1")}),
        ];
        if let Some(sums) = sums {
            assets.push(serde_json::json!({"name":"SHA256SUMS","url":format!("{API}assets/2")}));
            mock.bodies.insert(format!("{API}assets/2"), sums.to_vec());
        }
        let release = serde_json::json!({"tag_name":"v0.2.0","created_at":"2026-11-02T00:00:00Z","draft":false,"prerelease":false,"assets":assets});
        for endpoint in ["latest", "tags/v0.2.0"] {
            mock.bodies.insert(
                format!("{API}{endpoint}"),
                serde_json::to_vec(&release).unwrap(),
            );
        }
        mock.bodies.insert(format!("{API}assets/1"), archive.into());
        Arc::new(mock)
    }
    const ARCHIVE: &[u8] = include_bytes!("../tests/fixtures/update/release.tar.gz");
    const SUMS: &[u8] = include_bytes!("../tests/fixtures/update/SHA256SUMS");

    #[test]
    fn semver_normalization_and_no_downgrades() {
        assert_eq!(
            stable_version("v1.2.3").unwrap(),
            stable_version("1.2.3").unwrap()
        );
        assert!(newer("0.1.9", "v0.2.0").unwrap());
        assert!(newer("1.9.0", "1.10.0").unwrap());
        assert!(!newer("0.2.0", "v0.2.0").unwrap());
        assert!(!newer("0.3.0", "0.2.0").unwrap());
        for tag in ["vv1.0.0", "v01.0.0", "nightly", "1.0.0-rc.1", "1.0.0+build"] {
            assert!(stable_version(tag).is_err());
        }
    }
    #[test]
    fn exact_target_and_official_source_only() {
        let name = artifact(&stable_version("0.2.0").unwrap(), TARGET);
        let assets = [ReleaseAsset::new(name.clone(), format!("{API}assets/1"))];
        assert!(select_asset(&assets, &name).is_some());
        assert!(select_asset(&assets, &name.replace("x86_64", "aarch64")).is_none());
        assert!(select_asset(&[], &name).is_none());
        assert!(
            select_asset(&[ReleaseAsset::new(&name, "https://example.com/fp")], &name).is_none()
        );
        assert!(select_asset(&[assets[0].clone(), assets[0].clone()], &name).is_none());
        assert!(
            OfficialClient
                .get("http://example.com/fp", &HeaderMap::new(), None)
                .is_err()
        );
    }
    #[test]
    fn check_only_fetches_release_metadata() {
        let mock = mock(Some(SUMS), ARCHIVE);
        let releases = builder()
            .http_client(mock.clone())
            .build()
            .unwrap()
            .get_latest_release()
            .unwrap();
        assert!(
            newer(
                env!("CARGO_PKG_VERSION"),
                releases.latest().unwrap().version()
            )
            .unwrap()
        );
        assert_eq!(*mock.calls.lock().unwrap(), [format!("{API}latest")]);
    }
    #[test]
    fn network_errors_are_actionable() {
        let error = builder()
            .http_client(Arc::new(Mock::default()))
            .build()
            .unwrap()
            .get_latest_release()
            .unwrap_err();
        assert!(explain(error).to_string().contains("verifique a rede"));
        let error = self_update::Error::http_status_error(429, "https://api.github.com");
        assert!(explain(error).to_string().contains("Limite"));
        let error = self_update::Error::http_status_error(404, "https://api.github.com");
        assert!(explain(error).to_string().contains("não encontrado"));
    }
    #[test]
    fn checksum_missing_invalid_and_truncated_download_preserve_installation() {
        for (sums, archive) in [
            (None, ARCHIVE),
            (Some(b"invalid".as_slice()), ARCHIVE),
            (Some(SUMS), b"truncated".as_slice()),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let exe = temp.path().join("fp");
            fs::write(&exe, b"original").unwrap();
            let error = install_builder(
                &stable_version("0.2.0").unwrap(),
                &exe,
                fs::Permissions::from_mode(0o755),
            )
            .http_client(mock(sums, archive))
            .build()
            .unwrap()
            .update()
            .unwrap_err();
            assert!(matches!(
                error,
                self_update::Error::ChecksumSourceInvalid { .. }
                    | self_update::Error::ChecksumMismatch { .. }
            ));
            assert_eq!(fs::read(&exe).unwrap(), b"original");
        }
    }
    #[test]
    fn download_failure_preserves_original() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("fp");
        fs::write(&exe, b"original").unwrap();
        let mut mock = mock(Some(SUMS), ARCHIVE);
        Arc::get_mut(&mut mock)
            .unwrap()
            .bodies
            .remove(&format!("{API}assets/1"));
        let error = install_builder(
            &stable_version("0.2.0").unwrap(),
            &exe,
            fs::Permissions::from_mode(0o755),
        )
        .http_client(mock)
        .build()
        .unwrap()
        .update()
        .unwrap_err();
        assert!(matches!(error, self_update::Error::Transport(_)));
        assert_eq!(fs::read(exe).unwrap(), b"original");
    }

    #[test]
    fn concurrent_installation_change_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("fp");
        fs::write(&exe, b"original").unwrap();
        let original = fs::metadata(&exe).unwrap();
        assert!(unchanged(&exe, &original).is_ok());
        let replacement = temp.path().join("replacement");
        fs::write(&replacement, b"newer version").unwrap();
        fs::rename(replacement, &exe).unwrap();
        assert!(unchanged(&exe, &original).is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"newer version");
    }

    #[test]
    fn invalid_extracted_binary_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("fp");
        fs::write(&exe, b"original").unwrap();
        let error = install_builder(
            &stable_version("0.2.0").unwrap(),
            &exe,
            fs::Permissions::from_mode(0o755),
        )
        .http_client(mock(
            Some(include_bytes!(
                "../tests/fixtures/update/INVALID_SHA256SUMS"
            )),
            include_bytes!("../tests/fixtures/update/invalid.tar.gz"),
        ))
        .build()
        .unwrap()
        .update()
        .unwrap_err();
        assert!(matches!(
            error,
            self_update::Error::VerificationRejected { .. }
        ));
        assert_eq!(fs::read(exe).unwrap(), b"original");
    }
    #[test]
    fn isolated_replacement_and_install_failure() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("fp");
        fs::write(&exe, b"original").unwrap();
        // Fixture ELF real: checksum, extração, formato e --version usam o fluxo de produção.
        let status = install_builder(
            &stable_version("0.2.0").unwrap(),
            &exe,
            fs::Permissions::from_mode(0o755),
        )
        .http_client(mock(Some(SUMS), ARCHIVE))
        .build()
        .unwrap()
        .update()
        .unwrap();
        assert!(status.is_updated());
        assert!(fs::read(&exe).unwrap().starts_with(b"\x7fELF"));
        let invalid_destination = temp.path().join("missing/fp");
        assert!(
            install_builder(
                &stable_version("0.2.0").unwrap(),
                &invalid_destination,
                fs::Permissions::from_mode(0o755)
            )
            .verify_binary(|_| Ok(()))
            .http_client(mock(Some(SUMS), ARCHIVE))
            .build()
            .unwrap()
            .update()
            .is_err()
        );
        // Rollback da biblioteca diante de falha no rename após preservar o anterior.
        let backup = temp.path().join("backup");
        let before = fs::read(&exe).unwrap();
        assert!(
            self_update::Move::from_source(temp.path().join("absent"))
                .replace_using_temp(&backup)
                .to_dest(&exe)
                .is_err()
        );
        assert_eq!(fs::read(exe).unwrap(), before);
    }
    #[test]
    fn cancellation_never_calls_install_and_failure_propagates() {
        assert!(!confirmed_install(|| Ok(false), || panic!("instalação após cancelar")).unwrap());
        assert!(
            confirmed_install(|| Ok(true), || Err("falha simulada de substituição".into()))
                .is_err()
        );
        assert!(
            confirmed_install(
                || Err("cancelamento por sinal".into()),
                || panic!("instalação após interrupção")
            )
            .is_err()
        );
    }
    fn installation() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let bin = home.path().join(".local/bin");
        fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("fp");
        fs::write(&exe, b"original").unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(bin.join(".fp-standalone"), MARKER).unwrap();
        (home, exe)
    }
    #[test]
    fn standalone_requires_explicit_origin_and_write_permission() {
        let (home, exe) = installation();
        assert!(standalone(&exe, home.path(), None).is_ok());
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(
            standalone(&exe, home.path(), None)
                .unwrap_err()
                .to_string()
                .contains("permissão")
        );
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        let bin = exe.parent().unwrap();
        fs::set_permissions(bin, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(standalone(&exe, home.path(), None).is_err());
        fs::set_permissions(bin, fs::Permissions::from_mode(0o755)).unwrap();
        fs::remove_file(bin.join(".fp-standalone")).unwrap();
        assert!(standalone(&exe, home.path(), None).is_err());
    }
    #[test]
    fn cargo_system_unknown_symlink_and_hardlink_are_refused() {
        let (home, exe) = installation();
        assert!(standalone(&exe, home.path(), Some(&home.path().join(".local"))).is_err());
        fs::write(home.path().join(".local/.crates2.json"), "{}").unwrap();
        assert!(
            standalone(&exe, home.path(), None)
                .unwrap_err()
                .to_string()
                .contains("Cargo")
        );
        fs::remove_file(home.path().join(".local/.crates2.json")).unwrap();
        assert!(standalone(Path::new("/usr/bin/fp"), home.path(), None).is_err());
        let original = exe.with_file_name("original");
        fs::rename(&exe, &original).unwrap();
        symlink(&original, &exe).unwrap();
        assert!(standalone(&exe, home.path(), None).is_err());
        fs::remove_file(&exe).unwrap();
        fs::hard_link(&original, &exe).unwrap();
        assert!(standalone(&exe, home.path(), None).is_err());
    }
}
