use serde_json::Value;

const MAGIC: &[u8; 4] = b"FRCQ";
pub const FILE_VERSION: u8 = 1;
const LARGEST: usize = 64 << 20;

pub fn is_packed(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

pub fn pack(document: &Value) -> Vec<u8> {
    // Numbers of a scene are shorter as text than as eight-byte floats: squeezed JSON came out a third smaller
    // than squeezed CBOR.
    let mut out = MAGIC.to_vec();
    out.push(FILE_VERSION);
    out.extend(miniz_oxide::deflate::compress_to_vec(
        document.to_string().as_bytes(),
        9,
    ));
    out
}

pub fn unpack(bytes: &[u8]) -> Result<Value, String> {
    let Some(rest) = bytes.strip_prefix(MAGIC) else {
        return Err("not a Fractique file".into());
    };
    let Some((&version, packed)) = rest.split_first() else {
        return Err("the file is cut short".into());
    };
    if version != FILE_VERSION {
        return Err(format!(
            "the file is of version {version}; this program reads version {FILE_VERSION}"
        ));
    }
    let body = miniz_oxide::inflate::decompress_to_vec_with_limit(packed, LARGEST)
        .map_err(|_| "the file is damaged".to_owned())?;
    serde_json::from_slice(&body).map_err(|_| "the file is damaged".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets;

    #[test]
    fn a_packed_file_comes_back_the_same_and_smaller() {
        let scene = presets::get("solar-mandala").expect("the preset exists");
        let document = serde_json::json!({
            "fractique": 1,
            "view": [0.25, -1.5e-7, 123456.789],
            "seed": u64::MAX,
            "scene": serde_json::to_value(&scene).expect("a scene is JSON"),
        });
        let packed = pack(&document);
        assert!(is_packed(&packed));
        assert_eq!(unpack(&packed).expect("it unpacks"), document);
        assert!(packed.len() * 2 < document.to_string().len());
    }

    #[test]
    fn a_damaged_file_is_refused() {
        assert!(unpack(b"{}").is_err());
        assert!(unpack(b"FRCQ").is_err());
        assert!(unpack(b"FRCQ\x09").is_err());
        assert!(unpack(b"FRCQ\x01nonsense").is_err());
    }
}
