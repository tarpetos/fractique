use std::time::Duration;

// 256 MiB: far above any program of a release, far below what would exhaust a small server.
const LARGEST: u64 = 256 << 20;

#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Missing,
    Other(String),
}

pub trait Fetch: Send + Sync {
    fn get(&self, url: &str, limit: Duration) -> Result<Vec<u8>, Failure>;
}

pub struct Https;

impl Fetch for Https {
    fn get(&self, url: &str, limit: Duration) -> Result<Vec<u8>, Failure> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(limit))
            .user_agent(concat!("fractique/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        let mut response = agent.get(url).call().map_err(|error| match error {
            ureq::Error::StatusCode(404) => Failure::Missing,
            ureq::Error::StatusCode(403 | 429) => {
                Failure::Other("GitHub is limiting requests from this network for now".into())
            }
            ureq::Error::StatusCode(status) => Failure::Other(format!("{url} answered {status}")),
            other => Failure::Other(format!("{url}: {other}")),
        })?;
        response
            .body_mut()
            .with_config()
            .limit(LARGEST)
            .read_to_vec()
            .map_err(|error| Failure::Other(format!("{url}: {error}")))
    }
}
