// Embeds the RetinaCare icon into the Windows executable. Linux builds are unaffected.
fn main() {
    println!("cargo:rerun-if-changed=../icon/RetinaCare.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../icon/RetinaCare.ico");
        res.set("ProductName", "RetinaCare");
        res.set(
            "FileDescription",
            "RetinaCare AI Screening Suite (research prototype)",
        );
        res.compile()
            .expect("failed to embed the Windows icon resource");
    }
}
