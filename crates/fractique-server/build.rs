fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icon/server.ico");
        resource.set("ProductName", "Fractique");
        resource.set("FileDescription", "Fractique HTTP server");
        resource.set("InternalName", "fractique-server");
        resource.set("OriginalFilename", "fractique-server.exe");
        // compile() would link the resource into the tests of the package too.
        resource.compile_for("fractique-server").expect("the icon is embedded");
    }
}
