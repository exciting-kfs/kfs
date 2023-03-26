use glob::glob;
use nasm_rs;

static NASM_FLAGS: &[&str] = &["-w+all", "-w+error", "-Isrc/asm/include"];
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
	let files: Vec<_> = glob("src/asm/**/*.[sS]")
		.expect("invalid glob")
		.map(|file| {
			let file = file.expect("Unreachable file detected.");
			println!("cargo:rerun-if-changed={}", file.display());
			file
		})
		.collect();

	nasm_rs::compile_library_args(LIB_NAME, &files[..], NASM_FLAGS)
		.expect("failed to compile asm.");

	println!("cargo:rustc-link-lib=static={LIB_NAME}",);
}
