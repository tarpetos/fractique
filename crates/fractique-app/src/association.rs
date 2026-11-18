use crate::SCENE_FILE;

const PROG_ID: &str = "Fractique.Scene";
const TYPE_NAME: &str = "Fractique";

pub trait Classes {
    fn read(&self, key: &str, name: &str) -> Option<String>;
    fn write(&mut self, key: &str, name: &str, value: &str) -> bool;
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    key: String,
    name: &'static str,
    value: String,
}

fn open_command(exe: &str) -> String {
    format!("\"{exe}\" \"%1\"")
}

fn icon(exe: &str) -> String {
    format!("\"{exe}\",0")
}

fn entries(exe: &str) -> Vec<Entry> {
    let extension = format!(".{SCENE_FILE}");
    let entry = |key: String, name, value: String| Entry { key, name, value };
    vec![
        entry(extension.clone(), "", PROG_ID.to_owned()),
        entry(format!(r"{extension}\OpenWithProgids"), PROG_ID, String::new()),
        entry(PROG_ID.to_owned(), "", TYPE_NAME.to_owned()),
        entry(format!(r"{PROG_ID}\DefaultIcon"), "", icon(exe)),
        entry(format!(r"{PROG_ID}\shell\open\command"), "", open_command(exe)),
    ]
}

pub fn sync(classes: &mut impl Classes, exe: &str) -> usize {
    let mut written = 0;
    for entry in entries(exe) {
        if classes.read(&entry.key, entry.name).as_deref() != Some(entry.value.as_str())
            && classes.write(&entry.key, entry.name, &entry.value)
        {
            written += 1;
        }
    }
    written
}

#[cfg(windows)]
struct CurrentUser(winreg::RegKey);

#[cfg(windows)]
impl Classes for CurrentUser {
    fn read(&self, key: &str, name: &str) -> Option<String> {
        self.0.open_subkey(key).ok()?.get_value(name).ok()
    }

    fn write(&mut self, key: &str, name: &str, value: &str) -> bool {
        self.0
            .create_subkey(key)
            .and_then(|(key, _)| key.set_value(name, &value))
            .is_ok()
    }
}

#[cfg(windows)]
pub fn register() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

    if !crate::update::is_release() {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe) = exe.to_str() else {
        return;
    };
    let current_user = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let Ok((classes, _)) = current_user.create_subkey(r"Software\Classes") else {
        return;
    };
    if sync(&mut CurrentUser(classes), exe) > 0 {
        // Explorer keeps showing the old icon and command until it is told that associations changed.
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED as _,
                SHCNF_IDLIST as _,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Fake {
        values: HashMap<(String, String), String>,
        writes: usize,
        refuse: bool,
    }

    impl Classes for Fake {
        fn read(&self, key: &str, name: &str) -> Option<String> {
            self.values.get(&(key.to_owned(), name.to_owned())).cloned()
        }

        fn write(&mut self, key: &str, name: &str, value: &str) -> bool {
            if self.refuse {
                return false;
            }
            self.writes += 1;
            self.values.insert((key.to_owned(), name.to_owned()), value.to_owned());
            true
        }
    }

    const EXE: &str = r"C:\Users\Ann Lee\Apps\Fractique_0.1.0_x64.exe";

    fn value<'a>(fake: &'a Fake, key: &str, name: &str) -> &'a str {
        &fake.values[&(key.to_owned(), name.to_owned())]
    }

    #[test]
    fn quotes_the_program_and_the_file() {
        assert_eq!(
            open_command(EXE),
            r#""C:\Users\Ann Lee\Apps\Fractique_0.1.0_x64.exe" "%1""#
        );
        assert_eq!(icon(EXE), r#""C:\Users\Ann Lee\Apps\Fractique_0.1.0_x64.exe",0"#);
    }

    #[test]
    fn registers_the_extension_the_type_and_the_command() {
        let mut fake = Fake::default();
        assert_eq!(sync(&mut fake, EXE), 5);
        assert_eq!(value(&fake, ".frcq", ""), "Fractique.Scene");
        assert_eq!(value(&fake, r".frcq\OpenWithProgids", "Fractique.Scene"), "");
        assert_eq!(value(&fake, "Fractique.Scene", ""), "Fractique");
        assert_eq!(value(&fake, r"Fractique.Scene\DefaultIcon", ""), icon(EXE));
        assert_eq!(
            value(&fake, r"Fractique.Scene\shell\open\command", ""),
            open_command(EXE)
        );
    }

    #[test]
    fn writes_nothing_when_the_path_is_the_stored_one() {
        let mut fake = Fake::default();
        sync(&mut fake, EXE);
        fake.writes = 0;
        assert_eq!(sync(&mut fake, EXE), 0);
        assert_eq!(fake.writes, 0);
    }

    #[test]
    fn rewrites_only_what_names_the_program_when_it_has_moved() {
        let mut fake = Fake::default();
        sync(&mut fake, EXE);
        fake.writes = 0;
        let moved = r"D:\Tools\Fractique.exe";
        assert_eq!(sync(&mut fake, moved), 2);
        assert_eq!(fake.writes, 2);
        assert_eq!(
            value(&fake, r"Fractique.Scene\shell\open\command", ""),
            open_command(moved)
        );
        assert_eq!(value(&fake, r"Fractique.Scene\DefaultIcon", ""), icon(moved));
    }

    #[test]
    fn a_refused_write_changes_nothing() {
        let mut fake = Fake {
            refuse: true,
            ..Fake::default()
        };
        assert_eq!(sync(&mut fake, EXE), 0);
        assert!(fake.values.is_empty());
    }
}
