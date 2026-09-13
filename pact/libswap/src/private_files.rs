//! Restrict wallet data before creating secrets, including existing installs.
use anyhow::{Context, Result};
use std::path::Path;

pub fn private_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
        anyhow::ensure!(
            !std::fs::symlink_metadata(path)?.file_type().is_symlink(),
            "wallet data directory must not be a symbolic link"
        );
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        // The private parent protects newly created sidecars too. Repair modes
        // of existing immediate files without following links outside this dir.
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                private_dir(&entry.path())?;
            } else if entry.file_type()?.is_file() {
                std::fs::set_permissions(entry.path(), std::fs::Permissions::from_mode(0o600))?;
            }
        }
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(path)?;
    Ok(())
}

pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path).context("open private file")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn repairs_existing_directory_and_cookie_modes() {
        let dir = std::env::temp_dir().join(format!("pact-permissions-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cookie = dir.join(".cookie");
        std::fs::write(&cookie, "old").unwrap();
        std::fs::set_permissions(&cookie, std::fs::Permissions::from_mode(0o644)).unwrap();
        private_dir(&dir).unwrap();
        write_private(&cookie, b"new").unwrap();
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&cookie).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_file(cookie).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
