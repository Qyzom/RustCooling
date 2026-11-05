fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=build.rs");
    slint_build::compile("ui/app.slint").unwrap();

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/logo.ico");
        res.set("ProductName", "RustCooling");
        res.set("FileDescription", "RustCooling");
        res.set("LegalCopyright", "Copyright (c) 2025-2026 Qyzom");
        if let Err(e) = res.compile() {
            eprintln!("WINRES ERROR: {e:?}");
            panic!("winres failed: {e:?}");
        }

        // On GNU/MinGW targets, GNU ld ignores static archive members from libresource.a
        // because they don't export unresolved code symbols. Passing resource.o directly
        // forces GNU ld to include the Windows PE resource (.rsrc) section with icons!
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_o = std::path::Path::new(&out_dir).join("resource.o");
        if res_o.exists() {
            println!("cargo:rustc-link-arg={}", res_o.display());
        }
    }
}
