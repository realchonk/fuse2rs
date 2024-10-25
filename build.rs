use std::path::PathBuf;

use bindgen::Builder;

fn main() {
	#[cfg(target_os = "freebsd")]
	println!("cargo:rustc-link-search=/usr/local/lib");
	println!("cargo:rustc-link-lib=fuse");

	let bindings = Builder::default()
		.clang_arg("-I/usr/local/include")
		.header("src/wrapper.h")
		.parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
		.generate()
		.unwrap();

	let path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs");
	bindings.write_to_file(path).unwrap();
}
