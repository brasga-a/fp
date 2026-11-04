//! DPAPI sem UI, vinculado ao usuário e ao identificador do recurso.
use super::*;
use std::{
    io::{Read, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    ptr,
};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
    Storage::FileSystem::*,
};

fn no_reparse(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    for ancestor in absolute.ancestors() {
        if fs::symlink_metadata(ancestor)?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(
                "E_CREDENTIALS: reparse point no caminho dos segredos; operação recusada".into(),
            );
        }
    }
    Ok(())
}
pub fn directory(dir: &Path, create: bool) -> Result<()> {
    if create && !dir.try_exists()? {
        fs::create_dir(dir)?;
    }
    no_reparse(dir)?;
    if !fs::symlink_metadata(dir)?.is_dir() {
        return Err("E_CREDENTIALS: diretório inválido".into());
    }
    Ok(())
}
fn transform(data: &[u8], id: &str, encrypt: bool) -> Result<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into()?,
        pbData: data.as_ptr().cast_mut(),
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: id.len().try_into()?,
        pbData: id.as_ptr().cast_mut(),
    };
    let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    // SAFETY: entradas permanecem válidas durante a chamada; DPAPI aloca o buffer
    // de saída, que copiamos e liberamos com LocalFree. Nenhuma UI/escopo máquina.
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                ptr::null(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                &entropy,
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err("E_CREDENTIALS: DPAPI recusou o segredo; restaure o backup no perfil original, sem redefinir a senha".into());
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(bytes)
}
pub fn write(path: &Path, id: &str, password: &[u8]) -> Result<()> {
    let encrypted = transform(password, id, true)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    file.write_all(&encrypted)?;
    file.sync_all()?;
    Ok(())
}
pub fn read(path: &Path, id: &str, _: &Path) -> Result<String> {
    no_reparse(path)?;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: handle válido e buffer exigido pela API, mantido aberto até a leitura.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if info.nNumberOfLinks != 1
        || info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
    {
        return Err("E_CREDENTIALS: segredo deve ser arquivo regular sem links".into());
    }
    let mut encrypted = Vec::new();
    file.read_to_end(&mut encrypted)?;
    Ok(String::from_utf8(transform(&encrypted, id, false)?)?)
}
