fn main() -> Result<(), i8> {
    println!("cargo:rerun-if-changed=linker.ld");

    println!("cargo:rustc-link-arg-bin=kfs1=-n");
    println!("cargo:rustc-link-arg-bin=kfs1=-Tlinker.ld");

    Ok(())
}
