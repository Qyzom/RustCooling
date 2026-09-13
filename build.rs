fn main() {
    slint_build::compile("ui/app.slint").unwrap();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/logo.ico");
        res.set("ProductName", "RustCooling");
        res.set("FileDescription", "RustCooling - ID-COOLING FX Series LCD Display Controller");
        res.set("LegalCopyright", "Copyright (c) 2025-2026 Qyzom");
        res.compile().unwrap();
    }
}
