//! Build script for embedding Windows icon.

fn main() {
    // Only embed icon on Windows
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/logo/no-tilt.ico");
        res.set("ProductName", "Lune Custom Build");
        res.set("FileDescription", "A standalone Luau runtime");
        res.set("LegalCopyright", "MPL-2.0");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to embed icon: {}", e);
        }
    }
}
