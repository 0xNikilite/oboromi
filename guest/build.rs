fn main() {
    println!("cargo:rerun-if-changed=link.ld");
    println!("cargo:rustc-link-arg-bins=-T{}/link.ld", env!("CARGO_MANIFEST_DIR"));
}
