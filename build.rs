use std::env;
use std::path::PathBuf;
use cmake::Config;

fn main() {
    println!("cargo:rerun-if-changed=third_party/dynarmic");
    println!("cargo:rerun-if-changed=third_party/dynarmic_interface.cpp");
    
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let third_party_dir = manifest_dir.join("third_party");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    let dynarmic_build = Config::new(third_party_dir.join("dynarmic"))
        .define("DYNARMIC_TESTS", "OFF")
        .define("DYNARMIC_EXAMPLES", "OFF")
        .define("DYNARMIC_FRONTENDS", "OFF")
        .define("BOOST_ROOT", third_party_dir.join("ext-boost").to_str().unwrap())
        .build_target("dynarmic")
        .build();
    
    cc::Build::new()
        .cpp(true)
        .include(third_party_dir.join("dynarmic/include"))
        .include(dynarmic_build.join("include"))
        .file(third_party_dir.join("dynarmic_interface.cpp"))
        .compile("dynarmic_interface");
    
    println!("cargo:rustc-link-search={}/build", dynarmic_build.display());
    println!("cargo:rustc-link-lib=static=dynarmic");
    println!("cargo:rustc-link-lib=static=boost_system");
    
    if env::var("TARGET").unwrap().contains("windows") {
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=shell32");
    }
}
