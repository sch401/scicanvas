fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    let commit = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "dev".into());
    println!("cargo:rustc-env=SCICANVAS_COMMIT={commit}");
    #[cfg(windows)]
    {
        let version = env!("CARGO_PKG_VERSION");
        let mut resource = winresource::WindowsResource::new();
        resource.set("ProductName", "SciCanvas");
        resource.set("FileDescription", "SciCanvas");
        resource.set("ProductVersion", version);
        resource.set("FileVersion", version);
        resource.compile().expect("Windows version resources");
    }
}
