//! Identidade dos diretórios confirmados para exclusão.
use crate::Result;
use std::path::Path;

#[cfg(unix)]
pub(crate) type Identity = (u64, u64);
#[cfg(windows)]
pub(crate) type Identity = (u64, [u8; 16]);

#[cfg(unix)]
pub(crate) fn identify(path: &Path) -> Result<Identity> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(path)?;
    Ok((meta.dev(), meta.ino()))
}

#[cfg(windows)]
pub(crate) fn identify(path: &Path) -> Result<Identity> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FileIdInfo,
        GetFileInformationByHandleEx,
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
    // SAFETY: handle válido e buffer com tamanho/tipo exigidos por FileIdInfo.
    if unsafe {
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
    Ok((info.VolumeSerialNumber, info.FileId.Identifier))
}

pub(crate) fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    }
    #[cfg(unix)]
    {
        meta.is_symlink()
    }
}
