pub const REPOSITORY: &str = "tarpetos/fractique";
// Public half of the release workflow's signing key (`cargo packager signer generate`).
pub const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEY2OUVDRDQxQjEzNjk3NEEKUldSS2x6YXhRYzJlOXM3dGJST3B0QUxtek5mVzI5ZjVRTERiendRYkIvTEFIalFnUDk1bmE3L28K";

#[cfg(feature = "tool")]
mod command;
#[cfg(feature = "tool")]
mod fetch;
#[cfg(feature = "tool")]
mod manifest;
#[cfg(feature = "tool")]
mod place;
#[cfg(feature = "tool")]
mod swap;
#[cfg(feature = "tool")]
mod verify;

#[cfg(feature = "tool")]
pub use command::{CLI, MCP, Options, Outcome, Program, Report, SERVER, Setup, hint, run, start, update};
#[cfg(feature = "tool")]
pub use fetch::{Failure, Fetch, Https};
#[cfg(feature = "tool")]
pub use manifest::{File, Manifest, manifest_address, platform, version};
#[cfg(feature = "tool")]
pub use place::{Refusal, standing};
#[cfg(feature = "tool")]
pub use swap::{aside, replace, tidy};
#[cfg(feature = "tool")]
pub use verify::{unpack, verify};
