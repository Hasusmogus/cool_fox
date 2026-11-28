fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Runtime consumers such as D3D12 read the EXE's export table directly.
        println!("cargo::rustc-link-arg-bin=victory=/NOIMPLIB");
        println!("cargo::rustc-link-arg-bin=victory=/NOEXP");
    }
}
