fn main() {
    if cfg!(target_os = "windows") {
        return;
    }

    bindgen::Builder::default()
        .header("/usr/include/fluidsynth.h")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("src/bindings_raw.rs")
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-lib=fluidsynth");
}
