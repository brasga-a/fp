//! Identidade persistida de SQLite. IDs Windows nunca reutilizam o campo Unix.
use crate::{Resource, Result};
use std::{
    fs::{self, File},
    path::Path,
};

pub(crate) fn record(r: &mut Resource) -> Result<()> {
    let path = r.file.as_ref().ok_or("E_CONFIG: arquivo SQLite ausente")?;
    let file = open(path, false)?;
    #[cfg(unix)]
    {
        r.file_identity = Some(unix_id(&file)?);
    }
    #[cfg(windows)]
    {
        r.windows_file_identity = Some(windows_id(&file)?);
    }
    Ok(())
}
pub(crate) fn checked(r: &Resource, deleting: bool) -> Result<File> {
    let path = r.file.as_ref().ok_or("E_CONFIG: arquivo SQLite ausente")?;
    let file = open(path, deleting)?;
    #[cfg(unix)]
    let matches = Some(unix_id(&file)?) == r.file_identity;
    #[cfg(windows)]
    let matches = Some(windows_id(&file)?) == r.windows_file_identity;
    if !matches {
        return Err("E_CONFLICT: identidade do arquivo SQLite mudou ou não pode ser comprovada nesta plataforma".into());
    }
    Ok(file)
}
#[cfg(unix)]
fn open(path: &Path, _: bool) -> Result<File> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err("E_CONFLICT: SQLite deve ser arquivo regular sem links".into());
    }
    let file = File::open(path)?;
    use std::os::unix::fs::MetadataExt;
    let meta = fs::symlink_metadata(path)?;
    if (meta.dev(), meta.ino()) != unix_id(&file)? || !meta.is_file() {
        return Err("E_CONFLICT: arquivo SQLite alterado".into());
    }
    Ok(file)
}
#[cfg(unix)]
fn unix_id(file: &File) -> Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 {
        return Err("E_CONFLICT: SQLite deve ser arquivo regular sem hardlinks".into());
    }
    Ok((meta.dev(), meta.ino()))
}
#[cfg(unix)]
pub(crate) fn remove(r: &Resource) -> Result<()> {
    let _guard = checked(r, true)?;
    Ok(fs::remove_file(r.file.as_ref().unwrap())?)
}
#[cfg(windows)]
fn open(path: &Path, deleting: bool) -> Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::*;
    // Nega compartilhamento para exclusão/substituição enquanto validamos/executamos.
    let access = FILE_GENERIC_READ | if deleting { DELETE } else { 0 };
    Ok(fs::OpenOptions::new()
        .access_mode(access)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?)
}
#[cfg(windows)]
fn windows_id(file: &File) -> Result<(u64, [u8; 16])> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::*;
    let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    let mut basic: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: handle válido, buffers correspondem às estruturas solicitadas.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut basic) } == 0
        || unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if basic.nNumberOfLinks != 1
        || basic.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
    {
        return Err("E_CONFLICT: SQLite deve ser arquivo regular sem links/reparse points".into());
    }
    Ok((info.VolumeSerialNumber, info.FileId.Identifier))
}
#[cfg(windows)]
pub(crate) fn remove(r: &Resource) -> Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::*;
    let file = checked(r, true)?;
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: exclusão no próprio handle validado, sem reabrir um caminho mutável.
    if unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&disposition as *const FILE_DISPOSITION_INFO).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
