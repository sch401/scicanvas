fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../../installer/assets/product.ico");
    let commit = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "dev".into());
    println!("cargo:rustc-env=SCICANVAS_COMMIT={commit}");
    #[cfg(windows)]
    {
        let version = env!("CARGO_PKG_VERSION");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../installer/assets/product.ico"
        ));
        resource.set("ProductName", "SciCanvas");
        resource.set("FileDescription", "SciCanvas");
        resource.set("ProductVersion", version);
        resource.set("FileVersion", version);
        resource.compile().expect("Windows version resources");
    }
}
