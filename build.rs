fn main() -> shadow_rs::SdResult<()> {
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"cargo-clippy\"))");
    shadow_rs::new()
}
