fn main() -> Result<(), i8> {
    println!("cargo:rerun-if-changed=linker.ld");

    println!("cargo:rustc-link-arg-bin=kernel=-n");
    println!("cargo:rustc-link-arg-bin=kernel=-Tlinker.ld");

    Ok(())
}
