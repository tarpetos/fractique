fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let programs = [
            ("fractique", "../../assets/icon/cli.ico", "Fractique command line tool"),
            ("fractique-mcp", "../../assets/icon/mcp.ico", "Fractique MCP server"),
        ];
        for (program, icon, description) in programs {
            let mut resource = winresource::WindowsResource::new();
            resource.set_icon(icon);
            resource.set("ProductName", "Fractique");
            resource.set("FileDescription", description);
            resource.set("InternalName", program);
            resource.set("OriginalFilename", &format!("{program}.exe"));
            // compile() links a resource into every program and test of the package; each program gets its own.
            resource.compile_for(program).expect("the icon is embedded");
        }
    }
}
