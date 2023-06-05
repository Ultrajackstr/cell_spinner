#[cfg(windows)]
extern crate winresource;

#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("src/resources/icon.ico");
    res.compile().unwrap_or_default();
    // static_vcruntime::metabuild();
}

// Dummy main for non-windows platforms
#[cfg(not(windows))]
fn main() {}