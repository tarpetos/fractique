use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn replace(exe: &Path, program: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        over(exe, program)
    }
    #[cfg(not(unix))]
    {
        aside(exe, program)
    }
}

fn put(path: &Path, program: &[u8], like: &Path) -> io::Result<()> {
    let _ = fs::remove_file(path);
    let written = fs::metadata(like).and_then(|old| {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(program)?;
        file.set_permissions(old.permissions())?;
        file.sync_all()
    });
    if written.is_err() {
        let _ = fs::remove_file(path);
    }
    written
}

#[cfg(unix)]
pub fn over(exe: &Path, program: &[u8]) -> io::Result<()> {
    let name = exe.file_name().ok_or(io::ErrorKind::InvalidInput)?.to_string_lossy();
    let new = exe.with_file_name(format!(".{name}.new.{}", std::process::id()));
    put(&new, program, exe)?;
    fs::rename(&new, exe).inspect_err(|_| {
        let _ = fs::remove_file(&new);
    })
}

pub fn aside(exe: &Path, program: &[u8]) -> io::Result<()> {
    let (new, old) = (exe.with_extension("new"), exe.with_extension("old"));
    put(&new, program, exe)?;
    let _ = fs::remove_file(&old);
    // Windows refuses to overwrite a running program but lets it be renamed.
    if let Err(error) = fs::rename(exe, &old) {
        let _ = fs::remove_file(&new);
        return Err(error);
    }
    if let Err(error) = fs::rename(&new, exe) {
        let _ = fs::rename(&old, exe);
        let _ = fs::remove_file(&new);
        return Err(error);
    }
    let _ = fs::remove_file(&old);
    Ok(())
}

pub fn tidy() {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let _ = fs::remove_file(exe.with_extension("old"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .expect("a folder")
            .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[cfg(unix)]
    #[test]
    fn renames_the_new_program_over_the_old_and_keeps_its_mode() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().expect("a folder");
        let exe = folder.path().join("fractique");
        fs::write(&exe, b"old").expect("a file");
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o750)).expect("a mode");
        over(&exe, b"new program").expect("a swap");
        assert_eq!(fs::read(&exe).expect("a file"), b"new program");
        assert_eq!(fs::metadata(&exe).expect("a file").permissions().mode() & 0o7777, 0o750);
        assert_eq!(names(folder.path()), ["fractique"]);
    }

    #[cfg(unix)]
    #[test]
    fn leaves_the_old_program_when_the_new_one_cannot_be_written() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let folder = tempfile::tempdir().expect("a folder");
        let exe = folder.path().join("fractique");
        fs::write(&exe, b"old").expect("a file");
        if fs::metadata(&exe).expect("a file").uid() == 0 {
            return;
        }
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o555)).expect("a mode");
        let swapped = (over(&exe, b"new"), aside(&exe, b"new"));
        fs::set_permissions(folder.path(), fs::Permissions::from_mode(0o755)).expect("a mode");
        assert!(swapped.0.is_err() && swapped.1.is_err());
        assert_eq!(fs::read(&exe).expect("a file"), b"old");
        assert_eq!(names(folder.path()), ["fractique"]);
    }

    #[test]
    fn renames_the_old_program_aside_as_windows_needs() {
        let folder = tempfile::tempdir().expect("a folder");
        let exe = folder.path().join("fractique-cli_0.1.0_x64.exe");
        fs::write(&exe, b"old").expect("a file");
        fs::write(exe.with_extension("old"), b"older").expect("a file");
        fs::write(exe.with_extension("new"), b"half a download").expect("a file");
        aside(&exe, b"new program").expect("a swap");
        assert_eq!(fs::read(&exe).expect("a file"), b"new program");
        assert_eq!(names(folder.path()), ["fractique-cli_0.1.0_x64.exe"]);
    }

    #[test]
    fn leaves_nothing_behind_when_there_is_no_program_to_replace() {
        let folder = tempfile::tempdir().expect("a folder");
        let exe = folder.path().join("fractique.exe");
        assert!(aside(&exe, b"new").is_err());
        assert!(replace(&exe, b"new").is_err());
        assert!(names(folder.path()).is_empty());
    }
}
