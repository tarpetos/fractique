use std::io::ErrorKind;
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};

struct Net {
    proxy: TcpListener,
}

impl Net {
    fn watched() -> Self {
        let proxy = TcpListener::bind("127.0.0.1:0").expect("a port");
        proxy.set_nonblocking(true).expect("a listener that does not wait");
        Self { proxy }
    }

    // Piped, so stdout is not a terminal; every request would go to the proxy, which counts them.
    fn run(&self, program: &str, arguments: &[&str]) -> Output {
        let proxy = format!("http://{}", self.proxy.local_addr().expect("an address"));
        Command::new(program)
            .args(arguments)
            .env("HTTPS_PROXY", &proxy)
            .env("https_proxy", &proxy)
            .env("ALL_PROXY", &proxy)
            .env_remove("FRACTIQUE_NO_UPDATE_CHECK")
            .stdin(Stdio::null())
            .output()
            .expect("the program runs")
    }

    fn untouched(&self) -> bool {
        self.proxy.accept().is_err_and(|e| e.kind() == ErrorKind::WouldBlock)
    }
}

const PROGRAMS: [(&str, &str); 1] = [("fractique-server", env!("CARGO_BIN_EXE_fractique-server"))];

#[test]
fn version_prints_the_version_alone_and_asks_no_one() {
    let net = Net::watched();
    for (name, program) in PROGRAMS {
        for flag in ["--version", "-V"] {
            let output = net.run(program, &[flag]);
            assert!(output.status.success());
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{name} {}\n", env!("CARGO_PKG_VERSION"))
            );
            assert!(output.stderr.is_empty());
        }
    }
    assert!(net.untouched());
}

#[test]
fn update_leaves_a_build_from_the_source_alone() {
    let net = Net::watched();
    for (name, program) in PROGRAMS {
        for arguments in [&["update"][..], &["update", "--version", "0.1.0"]] {
            let output = net.run(program, arguments);
            assert_eq!(output.status.code(), Some(3));
            let said = String::from_utf8_lossy(&output.stderr);
            assert!(
                said.starts_with(&format!("{name}: this {name} was built from the source")),
                "{said}"
            );
            assert!(output.stdout.is_empty());
        }
    }
    assert!(net.untouched());
}

#[test]
fn update_refuses_what_it_does_not_know() {
    let net = Net::watched();
    for (_, program) in PROGRAMS {
        for arguments in [&["update", "--check", "--version", "0.1.0"][..], &["update", "--force"]] {
            assert_eq!(net.run(program, arguments).status.code(), Some(2));
        }
    }
    assert!(net.untouched());
}

#[test]
fn update_does_not_mind_the_settings_of_the_server() {
    let net = Net::watched();
    let output = Command::new(env!("CARGO_BIN_EXE_fractique-server"))
        .args(["update"])
        .env("FRACTIQUE_BIND", "0.0.0.0:8080")
        .env("FRACTIQUE_JOBS", "2")
        .env("FRACTIQUE_WEB", "/srv/fractique")
        .output()
        .expect("the program runs");
    assert_eq!(output.status.code(), Some(3));
    assert!(net.untouched());
}
