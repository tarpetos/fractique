use std::io::Read;
use std::path::{Component, Path};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use minisign_verify::{PublicKey, Signature};

// 256 MiB: far above any program of a release, far below what would exhaust a small server.
const LARGEST: u64 = 256 << 20;

fn text(encoded: &str, what: &str) -> Result<String, String> {
    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|_| format!("the {what} is not base64"))?;
    String::from_utf8(bytes).map_err(|_| format!("the {what} is not text"))
}

pub fn verify(bytes: &[u8], signature: &str, pubkey: &str, name: &str) -> Result<(), String> {
    let key = PublicKey::decode(&text(pubkey, "public key")?).map_err(|e| format!("the public key: {e}"))?;
    let signature = Signature::decode(&text(signature, "signature")?).map_err(|e| format!("the signature: {e}"))?;
    key.verify(bytes, &signature, false)
        .map_err(|e| format!("the signature does not fit the file: {e}"))?;
    // cargo-packager signs "timestamp:<seconds>\tfile:<name>" along with the bytes.
    let signed = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("file:"));
    if signed != Some(name) {
        return Err(format!(
            "the signature is for {}, not for {name}",
            signed.unwrap_or("a file without a name")
        ));
    }
    Ok(())
}

pub fn unpack(bytes: Vec<u8>, name: &str, binary: &str) -> Result<Vec<u8>, String> {
    if !name.ends_with(".tar.gz") {
        return Ok(bytes);
    }
    let fail = |e: std::io::Error| format!("{name} could not be unpacked: {e}");
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(&bytes[..]));
    for entry in archive.entries().map_err(fail)? {
        let entry = entry.map_err(fail)?;
        let path = entry.path().map_err(fail)?.into_owned();
        if !entry.header().entry_type().is_file() || !is_program(&path, binary) {
            continue;
        }
        let mut program = Vec::new();
        entry.take(LARGEST).read_to_end(&mut program).map_err(fail)?;
        if program.is_empty() {
            break;
        }
        return Ok(program);
    }
    Err(format!("{name} holds no {binary}"))
}

fn is_program(path: &Path, binary: &str) -> bool {
    let mut parts = path.components().filter(|part| !matches!(part, Component::CurDir));
    matches!((parts.next(), parts.next()), (Some(Component::Normal(only)), None) if only == binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = include_str!("../tests/fixtures/release.pub");
    const OTHER_KEY: &str = include_str!("../tests/fixtures/other.pub");
    const ARCHIVE_NAME: &str = "fractique-cli_9.9.9_x86_64-linux.tar.gz";
    const ARCHIVE: &[u8] = include_bytes!("../tests/fixtures/fractique-cli_9.9.9_x86_64-linux.tar.gz");
    const ARCHIVE_SIGNATURE: &str = include_str!("../tests/fixtures/fractique-cli_9.9.9_x86_64-linux.tar.gz.sig");
    const BARE_NAME: &str = "fractique-cli_9.9.9_x64.exe";
    const BARE: &[u8] = include_bytes!("../tests/fixtures/fractique-cli_9.9.9_x64.exe");
    const BARE_SIGNATURE: &str = include_str!("../tests/fixtures/fractique-cli_9.9.9_x64.exe.sig");
    const MCP_SIGNATURE: &str = include_str!("../tests/fixtures/fractique-mcp_9.9.9_x86_64-linux.tar.gz.sig");

    #[test]
    fn accepts_what_cargo_packager_signed() {
        assert_eq!(verify(ARCHIVE, ARCHIVE_SIGNATURE, KEY, ARCHIVE_NAME), Ok(()));
        assert_eq!(verify(BARE, BARE_SIGNATURE, KEY, BARE_NAME), Ok(()));
        assert_eq!(verify(BARE, &format!("{BARE_SIGNATURE}\n"), KEY, BARE_NAME), Ok(()));
    }

    #[test]
    fn refuses_a_changed_byte() {
        for at in [0, ARCHIVE.len() / 2, ARCHIVE.len() - 1] {
            let mut changed = ARCHIVE.to_vec();
            changed[at] ^= 1;
            assert!(
                verify(&changed, ARCHIVE_SIGNATURE, KEY, ARCHIVE_NAME).is_err(),
                "byte {at}"
            );
        }
        let mut longer = BARE.to_vec();
        longer.push(0);
        assert!(verify(&longer, BARE_SIGNATURE, KEY, BARE_NAME).is_err());
    }

    #[test]
    fn refuses_a_truncated_file() {
        assert!(verify(&ARCHIVE[..ARCHIVE.len() - 1], ARCHIVE_SIGNATURE, KEY, ARCHIVE_NAME).is_err());
        assert!(verify(&ARCHIVE[..ARCHIVE.len() / 2], ARCHIVE_SIGNATURE, KEY, ARCHIVE_NAME).is_err());
        assert!(verify(&[], ARCHIVE_SIGNATURE, KEY, ARCHIVE_NAME).is_err());
    }

    #[test]
    fn refuses_another_key() {
        assert!(verify(ARCHIVE, ARCHIVE_SIGNATURE, OTHER_KEY, ARCHIVE_NAME).is_err());
        assert!(verify(ARCHIVE, ARCHIVE_SIGNATURE, crate::PUBKEY, ARCHIVE_NAME).is_err());
    }

    #[test]
    fn refuses_the_signature_of_another_file() {
        assert!(verify(ARCHIVE, BARE_SIGNATURE, KEY, ARCHIVE_NAME).is_err());
        assert!(verify(ARCHIVE, MCP_SIGNATURE, KEY, ARCHIVE_NAME).is_err());
        let renamed = verify(
            ARCHIVE,
            ARCHIVE_SIGNATURE,
            KEY,
            "fractique-cli_10.0.0_x86_64-linux.tar.gz",
        );
        assert!(renamed.is_err_and(|e| e.contains(ARCHIVE_NAME)));
    }

    #[test]
    fn refuses_what_is_not_a_signature() {
        assert!(verify(BARE, "", KEY, BARE_NAME).is_err());
        assert!(verify(BARE, "not base64 at all", KEY, BARE_NAME).is_err());
        assert!(verify(BARE, &STANDARD.encode("one line"), KEY, BARE_NAME).is_err());
        assert!(verify(BARE, BARE_SIGNATURE, "", BARE_NAME).is_err());
        assert!(verify(BARE, BARE_SIGNATURE, BARE_SIGNATURE, BARE_NAME).is_err());
    }

    #[test]
    fn the_key_of_the_releases_is_a_key() {
        assert!(PublicKey::decode(&text(crate::PUBKEY, "public key").expect("base64")).is_ok());
    }

    #[test]
    fn unpacks_the_program() {
        let program = unpack(ARCHIVE.to_vec(), ARCHIVE_NAME, "fractique").expect("the program");
        assert_eq!(program, b"#!/bin/sh\necho \"fractique 9.9.9\"\n");
        assert_eq!(unpack(BARE.to_vec(), BARE_NAME, "fractique").as_deref(), Ok(BARE));
        assert!(unpack(ARCHIVE.to_vec(), ARCHIVE_NAME, "fractique-server").is_err());
        assert!(unpack(BARE.to_vec(), ARCHIVE_NAME, "fractique").is_err());
    }

    fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let packer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(packer);
        for (path, content) in files {
            let mut header = tar::Header::new_ustar();
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, path, *content).expect("an entry");
        }
        builder.into_inner().expect("a tar").finish().expect("a gzip")
    }

    #[test]
    fn takes_only_the_program_from_an_archive() {
        // macOS tar puts a "._name" file with the attributes of each file before it.
        let mac = archive(&[("./._fractique", b"attributes"), ("./fractique", b"program")]);
        assert_eq!(unpack(mac, "a.tar.gz", "fractique").as_deref(), Ok(&b"program"[..]));
        let nested = archive(&[("other/fractique", b"wrong"), ("fractique-mcp", b"wrong")]);
        assert!(unpack(nested, "a.tar.gz", "fractique").is_err());
        let empty = archive(&[("fractique", b"")]);
        assert!(unpack(empty, "a.tar.gz", "fractique").is_err());
    }
}
