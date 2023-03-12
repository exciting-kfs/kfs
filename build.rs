use glob::glob;
use nasm_rs;

const LINKER_SCRIPT: &str = "src/linker.ld";
static NASM_FLAGS: &[&str] = &["-felf"];
const LIB_NAME: &str = "init";

/* Cargo Book, Rustc Book,
 * [keyword]: meaning
 *
 * [cargo:rerun-if-changed]: dependency check
 * [cargo:rustc-link-arg-bin=BIN=FLAG]: '-C link-arg=FLAG' option to the rustc when only build BIN
 * [cargo:rustc-link-lib=static,dylib,framework]: link the given library using the rustc's -l flag
 *
 */

fn main() {
	println!("cargo:rerun-if-changed={}", LINKER_SCRIPT);

	let v = Vec::from_iter(
		glob("src/asm/**/*.[sS]")
			.expect("invalid glob")
			.map(|file| file.expect("file matched. but unreachable")),
	);

	for asm in &v[..] {
		println!("cargo:rerun-if-changed={}", asm.display());
	}

	nasm_rs::compile_library_args(LIB_NAME, &v[..], NASM_FLAGS).expect("failed to compile asm.");

	println!("cargo:rustc-link-lib=static={}", LIB_NAME);
	println!("cargo:rustc-link-arg-bin=kernel=-n");
	println!("cargo:rustc-link-arg-bin=kernel=-T{}", LINKER_SCRIPT);
}
