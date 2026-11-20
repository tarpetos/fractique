use std::io::Read;
use std::sync::{Arc, LazyLock};

use axum::extract::State;
use axum::http::header::{
    ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY, VARY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::{Config, Shared};

const SPEC: &str = include_str!("../openapi.json");
const SCALAR_GZ: &[u8] = include_bytes!("../vendor/scalar/standalone.js.gz");
pub(crate) const ICONS: [(&str, &[u8]); 3] = [
    ("/docs/icon-32.png", include_bytes!("../../../assets/icon/icon-32.png")),
    ("/docs/icon-64.png", include_bytes!("../../../assets/icon/icon-64.png")),
    ("/docs/icon.png", include_bytes!("../../../assets/icon/icon-256.png")),
];

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; worker-src 'self' blob:; base-uri 'none'; \
form-action 'none'; frame-ancestors 'none'";

const PAGE: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#06080d">
<title>Fractique API</title>
<link rel="icon" type="image/png" sizes="32x32" href="/docs/icon-32.png">
<link rel="icon" type="image/png" sizes="64x64" href="/docs/icon-64.png">
<link rel="apple-touch-icon" href="/docs/icon.png">
<style>html, body { margin: 0; background: #06080d; }</style>
</head>
<body>
<div id="app"></div>
<script src="/docs/scalar.js"></script>
<script src="/docs/init.js"></script>
</body>
</html>
"##;

// Scalar's online extras (fonts, telemetry, AI agent, MCP, its hosted client) are off; the CSP blocks them anyway.
const INIT: &str = r##"Scalar.createApiReference('#app', {
  url: '/openapi.json',
  darkMode: true,
  forceDarkModeState: 'dark',
  hideDarkModeToggle: true,
  customCss: `
    .light-mode, .dark-mode {
      --scalar-background-1: #06080d;
      --scalar-background-2: #0d111a;
      --scalar-background-3: #161c29;
      --scalar-background-accent: rgba(126, 178, 255, 0.12);
      --scalar-color-1: #f4f6fb;
      --scalar-color-2: #aab3c5;
      --scalar-color-3: #717b90;
      --scalar-color-accent: #8ab8ff;
      --scalar-border-color: rgba(255, 255, 255, 0.1);
      --scalar-sidebar-background-1: #080b12;
      --scalar-sidebar-color-active: #f4f6fb;
      --scalar-sidebar-item-active-background: rgba(126, 178, 255, 0.14);
      --scalar-button-1: #f4f6fb;
      --scalar-button-1-color: #06080d;
      --scalar-button-1-hover: #d5dbe8;
    }
    .scalar-app aside::before, .t-doc__header header::after {
      content: ''; flex: none; background: url(/docs/icon.png) no-repeat left center / contain;
    }
    .scalar-app aside::before {
      content: 'Fractique API'; display: flex; align-items: center; height: 40px; margin: 14px 12px 4px;
      padding-left: 50px; background-size: 40px 40px; font-size: 16px; font-weight: 600;
      color: var(--scalar-color-1);
    }
    .t-doc__header header::after { width: 28px; height: 28px; margin-right: 8px; }
  `,
  withDefaultFonts: false,
  telemetry: false,
  agent: { disabled: true },
  mcp: { disabled: true },
  hideClientButton: true,
  showDeveloperTools: 'never',
  persistAuth: false,
});
"##;

static SCALAR_JS: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut script = Vec::new();
    flate2::read::GzDecoder::new(SCALAR_GZ)
        .read_to_end(&mut script)
        .expect("the vendored Scalar is gzip");
    script
});

pub(crate) fn spec(config: &Config) -> String {
    let mut spec: Value = serde_json::from_str(SPEC).expect("openapi.json is JSON");
    spec["info"]["version"] = json!(env!("CARGO_PKG_VERSION"));
    let limits = format!(
        "\n\n**This server** renders up to {} pixels a side and {} pixels in all, with up to {} samples per pixel \
         and a supersample up to {}, {} at once and {} waiting, each for at most {} s.",
        config.max_side,
        config.max_pixels,
        config.max_samples,
        config.max_supersample,
        config.jobs,
        config.queue,
        config.timeout.as_secs_f64()
    );
    if let Some(Value::String(description)) = spec.pointer_mut("/info/description") {
        description.push_str(&limits);
    }
    let fields = [
        ("width", config.max_side),
        ("height", config.max_side),
        ("samples", config.max_samples),
        ("supersample", config.max_supersample),
    ];
    for (field, most) in fields {
        spec["components"]["schemas"]["RenderRequest"]["properties"][field]["maximum"] = json!(most);
    }
    if !config.mcp
        && let Some(paths) = spec["paths"].as_object_mut()
    {
        paths.remove("/mcp");
    }
    spec.to_string()
}

pub(crate) async fn openapi(State(shared): State<Arc<Shared>>) -> Response {
    ([(CONTENT_TYPE, "application/json")], shared.spec.clone()).into_response()
}

pub(crate) async fn page() -> Response {
    let headers = [
        (CONTENT_TYPE, "text/html; charset=utf-8"),
        (CONTENT_SECURITY_POLICY, CSP),
        (REFERRER_POLICY, "no-referrer"),
        (X_FRAME_OPTIONS, "DENY"),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ];
    (headers, PAGE).into_response()
}

pub(crate) fn icon(png: &'static [u8]) -> Response {
    ([(CONTENT_TYPE, "image/png"), (X_CONTENT_TYPE_OPTIONS, "nosniff")], png).into_response()
}

fn script(body: &'static [u8], encoding: Option<&'static str>) -> Response {
    let kind = [
        (CONTENT_TYPE, "text/javascript; charset=utf-8"),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
    ];
    let mut response = (kind, body).into_response();
    if let Some(encoding) = encoding {
        response
            .headers_mut()
            .insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }
    response
}

pub(crate) async fn init_js() -> Response {
    script(INIT.as_bytes(), None)
}

pub(crate) async fn scalar_js(headers: HeaderMap) -> Response {
    let gzip = headers
        .get_all(ACCEPT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(accepts_gzip);
    let mut response = if gzip {
        script(SCALAR_GZ, Some("gzip"))
    } else {
        script(&SCALAR_JS, None)
    };
    response
        .headers_mut()
        .insert(VARY, HeaderValue::from_static("accept-encoding"));
    response
}

fn accepts_gzip(value: &str) -> bool {
    value.split(',').any(|item| {
        let mut parts = item.split(';').map(str::trim);
        let name = parts.next().unwrap_or_default();
        let refused = parts.any(|part| part.strip_prefix("q=").and_then(|q| q.parse::<f32>().ok()) == Some(0.0));
        (name.eq_ignore_ascii_case("gzip") || name == "*") && !refused
    })
}

#[cfg(test)]
mod tests {
    use fractique_core::Quality;
    use fractique_core::wallpaper::{Layout, Material};

    use super::*;
    use crate::MAX_BODY_BYTES;

    fn file() -> Value {
        serde_json::from_str(SPEC).expect("openapi.json is JSON")
    }

    fn refs<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
        match value {
            Value::Object(object) => {
                found.extend(object.get("$ref").and_then(Value::as_str));
                object.values().for_each(|value| refs(value, found));
            }
            Value::Array(list) => list.iter().for_each(|value| refs(value, found)),
            _ => {}
        }
    }

    fn names(schema: &Value) -> Vec<&str> {
        let listed = schema["enum"].as_array().expect("an enum");
        listed.iter().filter_map(Value::as_str).collect()
    }

    #[test]
    fn the_file_is_openapi_3_1_and_its_refs_resolve() {
        let spec = file();
        assert_eq!(spec["openapi"], "3.1.0");
        let mut found = Vec::new();
        refs(&spec, &mut found);
        assert!(found.len() > 20);
        for name in found {
            let target = spec.pointer(name.trim_start_matches('#'));
            assert!(target.is_some_and(Value::is_object), "{name} does not resolve");
        }
    }

    #[test]
    fn the_file_states_the_defaults_and_the_names_of_the_code() {
        let spec = file();
        let config = Config::default();
        let schemas = &spec["components"]["schemas"];
        let render = &schemas["RenderRequest"]["properties"];
        assert_eq!(render["width"]["maximum"], config.max_side);
        assert_eq!(render["height"]["maximum"], config.max_side);
        assert_eq!(render["samples"]["maximum"], config.max_samples);
        assert_eq!(render["supersample"]["maximum"], config.max_supersample);

        let limits = &schemas["Limits"]["properties"];
        assert_eq!(limits["jobs"]["default"], config.jobs);
        assert_eq!(limits["queue"]["default"], config.queue);
        assert_eq!(limits["max_side"]["default"], config.max_side);
        assert_eq!(limits["max_pixels"]["default"], config.max_pixels);
        assert_eq!(limits["max_samples"]["default"], config.max_samples);
        assert_eq!(limits["max_supersample"]["default"], config.max_supersample);
        assert_eq!(limits["timeout_seconds"]["default"], config.timeout.as_secs());
        assert_eq!(limits["max_body_bytes"]["const"], MAX_BODY_BYTES);
        assert_eq!(limits["mcp"]["default"], config.mcp);
        assert_eq!(limits["docs"]["default"], config.docs);
        assert_eq!(MAX_BODY_BYTES, 1 << 20, "openapi.json says 1 MiB in words");

        let quality = &render["quality"];
        assert_eq!(names(quality), Quality::ALL.map(|(name, _)| name));
        assert_eq!(quality["default"], "standard");
        let said = quality["description"].as_str().expect("a description");
        for (name, quality) in Quality::ALL {
            let (samples, supersample) = quality.settings();
            let line = format!("{name} {samples} and {supersample}");
            assert!(said.contains(&line), "{line}");
        }
        let wallpaper = &schemas["WallpaperRequest"]["properties"];
        assert_eq!(names(&wallpaper["layout"]), Layout::ALL.map(|(name, _)| name));
        assert_eq!(names(&wallpaper["material"]), Material::ALL.map(|(name, _)| name));

        for method in ["post", "delete"] {
            let headers = spec["paths"]["/mcp"][method]["parameters"].as_array().expect("headers");
            let version = headers.iter().find(|header| header["name"] == "MCP-Protocol-Version");
            assert_eq!(names(&version.expect("the header")["schema"]), crate::mcp::PROTOCOLS);
        }
    }

    #[test]
    fn the_served_description_has_the_limits_of_its_server() {
        let config = Config {
            max_side: 512,
            max_samples: 40,
            max_supersample: 1,
            mcp: false,
            ..Config::default()
        };
        let spec: Value = serde_json::from_str(&spec(&config)).expect("JSON");
        assert_eq!(spec["info"]["version"], env!("CARGO_PKG_VERSION"));
        let render = &spec["components"]["schemas"]["RenderRequest"]["properties"];
        assert_eq!(render["width"]["maximum"], 512);
        assert_eq!(render["samples"]["maximum"], 40);
        assert_eq!(render["supersample"]["maximum"], 1);
        assert!(spec["paths"].get("/mcp").is_none());
        let said = spec["info"]["description"].as_str().expect("a description");
        assert!(said.contains("up to 512 pixels a side"), "{said}");
    }

    #[test]
    fn the_page_uses_every_icon_and_only_this_server() {
        for (path, png) in ICONS {
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "{path}");
            assert!(PAGE.contains(path) || INIT.contains(path), "{path} is not used");
        }
        for text in [PAGE, INIT] {
            assert!(!text.contains("http://") && !text.contains("https://") && !text.contains("//cdn"));
        }
    }

    #[test]
    fn gzip_is_sent_to_clients_that_take_it() {
        for value in ["gzip", "gzip, deflate, br", "br;q=1.0, GZIP;q=0.5", "*"] {
            assert!(accepts_gzip(value), "{value}");
        }
        for value in ["", "identity", "br", "gzip;q=0", "gzip; q=0.0"] {
            assert!(!accepts_gzip(value), "{value}");
        }
        assert!(SCALAR_JS.len() > SCALAR_GZ.len());
    }
}
