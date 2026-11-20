use std::collections::BTreeMap;

use semver::Version;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct File {
    pub url: String,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    pub programs: BTreeMap<String, BTreeMap<String, File>>,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| format!("tools.json is not valid: {e}"))?;
        version(&manifest.version)?;
        Ok(manifest)
    }

    pub fn file(&self, program: &str, platform: &str) -> Option<&File> {
        self.programs.get(program)?.get(platform)
    }
}

pub fn version(text: &str) -> Result<Version, String> {
    let plain = text.strip_prefix('v').unwrap_or(text);
    Version::parse(plain).map_err(|_| format!("'{text}' is not a version; write it as 0.2.0"))
}

pub fn platform(os: &str, arch: &str) -> Option<String> {
    let known = ["windows", "linux", "macos"].contains(&os) && ["x86_64", "aarch64"].contains(&arch);
    known.then(|| format!("{os}-{arch}"))
}

pub fn manifest_address(releases: &str, version: Option<&Version>) -> String {
    match version {
        Some(version) => format!("{releases}/download/v{version}/tools.json"),
        None => format!("{releases}/latest/download/tools.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = r#"{
        "version": "0.3.0",
        "pub_date": "2026-11-20T12:00:00Z",
        "programs": {
            "fractique-cli": {
                "windows-x86_64": { "url": "https://example.org/fractique-cli_0.3.0_x64.exe", "signature": "AAAA" },
                "linux-x86_64": { "url": "https://example.org/fractique-cli_0.3.0_x86_64-linux.tar.gz", "signature": "BBBB" }
            },
            "fractique-server": {
                "macos-aarch64": { "url": "https://example.org/fractique-server_0.3.0_aarch64-macos.tar.gz", "signature": "CCCC", "later": 1 }
            }
        },
        "later": true
    }"#;

    #[test]
    fn reads_a_manifest() {
        let manifest = Manifest::parse(TEXT.as_bytes()).expect("a manifest");
        assert_eq!(manifest.version, "0.3.0");
        assert_eq!(manifest.pub_date.as_deref(), Some("2026-11-20T12:00:00Z"));
        let file = manifest.file("fractique-cli", "linux-x86_64").expect("an entry");
        assert_eq!(file.url, "https://example.org/fractique-cli_0.3.0_x86_64-linux.tar.gz");
        assert_eq!(file.signature, "BBBB");
        assert_eq!(
            manifest
                .file("fractique-server", "macos-aarch64")
                .map(|f| f.signature.as_str()),
            Some("CCCC")
        );
        assert!(manifest.file("fractique-cli", "macos-aarch64").is_none());
        assert!(manifest.file("fractique-mcp", "linux-x86_64").is_none());
    }

    #[test]
    fn refuses_what_is_not_a_manifest() {
        assert!(Manifest::parse(b"<html>Not Found</html>").is_err());
        assert!(Manifest::parse(br#"{"version": "0.3.0"}"#).is_err());
        assert!(Manifest::parse(br#"{"version": "soon", "programs": {}}"#).is_err());
        assert!(
            Manifest::parse(br#"{"version": "0.3.0", "programs": {"fractique-cli": {"linux-x86_64": {"url": "u"}}}}"#)
                .is_err()
        );
        assert!(Manifest::parse(br#"{"version": "0.3.0", "programs": {}}"#).is_ok());
    }

    #[test]
    fn orders_versions_by_number() {
        let v = |text| version(text).expect("a version");
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("0.3.0") > v("0.3.0-rc.1"));
        assert!(v("0.3.0-rc.2") > v("0.3.0-rc.1"));
        assert_eq!(v("v0.3.0"), v("0.3.0"));
        assert!(version("0.3").is_err());
        assert!(version("newest").is_err());
        assert!(version("").is_err());
    }

    #[test]
    fn names_the_platform_of_each_target() {
        assert_eq!(platform("windows", "x86_64").as_deref(), Some("windows-x86_64"));
        assert_eq!(platform("linux", "x86_64").as_deref(), Some("linux-x86_64"));
        assert_eq!(platform("macos", "aarch64").as_deref(), Some("macos-aarch64"));
        assert_eq!(platform("linux", "aarch64").as_deref(), Some("linux-aarch64"));
        assert_eq!(platform("macos", "x86_64").as_deref(), Some("macos-x86_64"));
        assert_eq!(platform("android", "aarch64"), None);
        assert_eq!(platform("freebsd", "x86_64"), None);
        assert_eq!(platform("linux", "riscv64"), None);
        assert_eq!(
            platform(std::env::consts::OS, std::env::consts::ARCH).is_some(),
            cfg!(any(windows, target_os = "linux", target_os = "macos"))
                && cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
        );
    }

    #[test]
    fn finds_the_manifest_of_a_release() {
        let releases = "https://github.com/tarpetos/fractique/releases";
        assert_eq!(
            manifest_address(releases, None),
            "https://github.com/tarpetos/fractique/releases/latest/download/tools.json"
        );
        assert_eq!(
            manifest_address(releases, Some(&Version::new(0, 2, 0))),
            "https://github.com/tarpetos/fractique/releases/download/v0.2.0/tools.json"
        );
    }
}
