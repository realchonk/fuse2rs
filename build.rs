use std::path::PathBuf;
use bindgen::{CargoCallbacks, Builder};


fn main() {
	println!("cargo:rustc-link-lib=fuse");

	let bindings = Builder::default()
		.header("src/wrapper.h")
		.parse_callbacks(Box::new(CargoCallbacks::new()))
		.generate()
		.unwrap();

	let path = PathBuf::from(std::env::var("OUT_DIR").unwrap())
		.join("bindings.rs");
	bindings.write_to_file(path).unwrap();
}
