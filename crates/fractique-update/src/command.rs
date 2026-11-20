use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use semver::Version;

use crate::fetch::{Failure, Fetch, Https};
use crate::manifest::{Manifest, manifest_address, platform, version};
use crate::place::standing;
use crate::swap::{replace, tidy};
use crate::verify::{unpack, verify};

const MANIFEST_LIMIT: Duration = Duration::from_secs(30);
const DOWNLOAD_LIMIT: Duration = Duration::from_secs(300);
const HINT_LIMIT: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Program {
    pub key: &'static str,
    pub binary: &'static str,
}

pub const CLI: Program = Program {
    key: "fractique-cli",
    binary: "fractique",
};
pub const MCP: Program = Program {
    key: "fractique-mcp",
    binary: "fractique-mcp",
};
pub const SERVER: Program = Program {
    key: "fractique-server",
    binary: "fractique-server",
};

#[derive(clap::Args, Clone, Debug, Default)]
pub struct Options {
    /// Only say whether a newer version is out: exit code 0 when this is the newest, 10 when a newer one is out
    #[arg(long, conflicts_with = "version")]
    pub check: bool,
    /// Install this version instead of the newest, an older one too
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    Current,
    Newer,
    Installed,
    Refused,
    Failed,
}

impl Outcome {
    pub fn code(self) -> u8 {
        match self {
            Self::Current | Self::Installed => 0,
            Self::Failed => 1,
            Self::Refused => 3,
            Self::Newer => 10,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub outcome: Outcome,
    pub text: String,
}

pub struct Setup {
    pub program: Program,
    pub current: String,
    pub release: bool,
    pub platform: Option<String>,
    pub exe: PathBuf,
    pub appdir: Option<PathBuf>,
    pub pubkey: String,
    pub releases: String,
    pub fetch: Arc<dyn Fetch>,
}

impl Setup {
    pub fn here(program: Program) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| format!("where this program is cannot be found out: {e}"))?;
        #[cfg(unix)]
        let exe = exe.canonicalize().map_err(|e| format!("{}: {e}", exe.display()))?;
        Ok(Self {
            program,
            current: env!("CARGO_PKG_VERSION").into(),
            release: option_env!("FRACTIQUE_RELEASE").is_some(),
            platform: platform(std::env::consts::OS, std::env::consts::ARCH),
            exe,
            appdir: std::env::var_os("APPDIR").map(PathBuf::from),
            pubkey: crate::PUBKEY.into(),
            releases: format!("https://github.com/{}/releases", crate::REPOSITORY),
            fetch: Arc::new(Https),
        })
    }

    fn downloads(&self, version: Option<&Version>) -> String {
        match version {
            Some(version) => format!("{}/tag/v{version}", self.releases),
            None => format!("{}/latest", self.releases),
        }
    }

    fn manifest(&self, wanted: Option<&Version>) -> Result<Manifest, String> {
        let address = manifest_address(&self.releases, wanted);
        match self.fetch.get(&address, MANIFEST_LIMIT) {
            Ok(bytes) => Manifest::parse(&bytes),
            Err(Failure::Other(reason)) => Err(reason),
            Err(Failure::Missing) => Err(match wanted {
                Some(wanted) => format!(
                    "release {wanted} holds no tools.json: there is no such release, or it was made before the \
                     programs could update themselves. Take its files by hand: {}",
                    self.releases
                ),
                None => format!(
                    "the newest release holds no tools.json: it was made before the programs could update \
                     themselves, or there is no release yet. Take its files by hand: {}",
                    self.downloads(None)
                ),
            }),
        }
    }
}

enum Stop {
    Refused(String),
    Failed(String),
}

pub fn run(setup: &Setup, options: &Options) -> Report {
    let (outcome, text) = match attempt(setup, options) {
        Ok(done) => done,
        Err(Stop::Refused(text)) => (Outcome::Refused, text),
        Err(Stop::Failed(text)) => (Outcome::Failed, text),
    };
    Report { outcome, text }
}

fn attempt(setup: &Setup, options: &Options) -> Result<(Outcome, String), Stop> {
    let Program { key, binary } = setup.program;
    let current = version(&setup.current).map_err(Stop::Failed)?;
    let wanted = options
        .version
        .as_deref()
        .map(version)
        .transpose()
        .map_err(Stop::Failed)?;
    let place = standing(&setup.exe, setup.release, setup.appdir.as_deref());
    let elsewhere = |version: Option<&Version>| match &place {
        Ok(()) => None,
        Err(refusal) => Some(format!(
            "{}\nDownloads: {}",
            refusal.says(binary, &setup.exe),
            setup.downloads(version)
        )),
    };
    if !options.check
        && let Some(text) = elsewhere(wanted.as_ref())
    {
        return Err(Stop::Refused(text));
    }

    let manifest = setup.manifest(wanted.as_ref()).map_err(Stop::Failed)?;
    let offered = version(&manifest.version).map_err(Stop::Failed)?;
    match &wanted {
        Some(wanted) if *wanted != offered => {
            return Err(Stop::Failed(format!(
                "the tools.json of release {wanted} is of version {offered}"
            )));
        }
        Some(_) if offered == current => {
            return Ok((Outcome::Current, format!("{binary} {current} is the version you have.")));
        }
        None if offered == current => {
            return Ok((Outcome::Current, format!("{binary} {current} is the newest version.")));
        }
        None if offered < current => {
            let text = format!("{binary} {current} is newer than the newest release, {offered}.");
            return Ok((Outcome::Current, text));
        }
        _ => {}
    }
    if options.check {
        let advice = elsewhere(Some(&offered))
            .map(|text| format!("But {text}"))
            .unwrap_or_else(|| format!("Install it with: {binary} update"));
        let text = format!("{binary} {offered} is out; this is {current}.\n{advice}");
        return Ok((Outcome::Newer, text));
    }

    let missing = |what: &str| {
        Stop::Refused(format!(
            "release {offered} holds no {key} for {what}.\nDownloads: {}",
            setup.downloads(Some(&offered))
        ))
    };
    let platform = setup.platform.as_deref().ok_or_else(|| missing("this system"))?;
    let file = manifest.file(key, platform).ok_or_else(|| missing(platform))?;
    let name = file.url.rsplit('/').next().unwrap_or_default();
    if !name.starts_with(&format!("{key}_{offered}_")) {
        return Err(Stop::Failed(format!(
            "tools.json names {name} as {key} {offered} for {platform}: nothing was replaced"
        )));
    }
    let untouched = |reason: String| Stop::Failed(format!("{reason}: nothing was replaced"));
    let bytes = setup.fetch.get(&file.url, DOWNLOAD_LIMIT).map_err(|failure| {
        untouched(match failure {
            Failure::Missing => format!("{} is not there", file.url),
            Failure::Other(reason) => reason,
        })
    })?;
    verify(&bytes, &file.signature, &setup.pubkey, name).map_err(untouched)?;
    let program = unpack(bytes, name, binary).map_err(untouched)?;
    replace(&setup.exe, &program).map_err(|e| untouched(format!("{}: {e}", setup.exe.display())))?;
    let text = format!(
        "{binary} {current} is now {offered}: {}\nA {binary} that is still running, a server above all, \
         stays {current} until it is restarted.",
        setup.exe.display()
    );
    Ok((Outcome::Installed, text))
}

pub fn hint(setup: &Setup, terminal: bool, off: bool, limit: Duration) -> Option<String> {
    if !terminal || off || !setup.release {
        return None;
    }
    let current = version(&setup.current).ok()?;
    let address = manifest_address(&setup.releases, None);
    let fetch = Arc::clone(&setup.fetch);
    let (sender, answer) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(fetch.get(&address, limit));
    });
    let bytes = answer.recv_timeout(limit).ok()?.ok()?;
    let offered = version(&Manifest::parse(&bytes).ok()?.version).ok()?;
    (offered > current).then(|| format!("{offered} is out: {} update", setup.program.binary))
}

pub fn start(program: Program) -> Option<ExitCode> {
    tidy();
    let mut arguments = std::env::args_os().skip(1);
    let (Some(only), None) = (arguments.next(), arguments.next()) else {
        return None;
    };
    if only != "--version" && only != "-V" {
        return None;
    }
    println!("{} {}", program.binary, env!("CARGO_PKG_VERSION"));
    let off = std::env::var_os("FRACTIQUE_NO_UPDATE_CHECK").is_some_and(|value| !value.is_empty() && value != "0");
    if std::io::stdout().is_terminal()
        && !off
        && let Ok(setup) = Setup::here(program)
        && let Some(line) = hint(&setup, true, off, HINT_LIMIT)
    {
        println!("{line}");
    }
    Some(ExitCode::SUCCESS)
}

pub fn update(program: Program, options: &Options) -> ExitCode {
    let report = match Setup::here(program) {
        Ok(setup) => run(&setup, options),
        Err(text) => Report {
            outcome: Outcome::Failed,
            text,
        },
    };
    match report.outcome {
        Outcome::Refused | Outcome::Failed => eprintln!("{}: {}", program.binary, report.text),
        _ => println!("{}", report.text),
    }
    ExitCode::from(report.outcome.code())
}
