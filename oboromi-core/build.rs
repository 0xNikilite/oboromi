use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;

fn patch_boost_for_macos() -> Result<(), Box<dyn std::error::Error>> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "macos" {
        return Ok(());
    }
    let file_path = "../third_party/ext-boost/boost/container_hash/hash.hpp";
    let path = Path::new(file_path);
    if !path.exists() {
        println!("cargo:warning=Boost hash.hpp not found at {}, skipping patch", file_path);
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let patterns = [
        "struct hash_base : std::unary_function<T, std::size_t> {};",
        "struct hash_base : boost::unary_function<T, std::size_t> {};",
        "struct hash_base : ::std::unary_function<T, std::size_t> {};",
    ];
    let replacement = "struct hash_base { typedef T argument_type; typedef std::size_t result_type; };";
    let mut patched = false;
    let mut patched_content = content.clone();
    for pattern in patterns {
        if patched_content.contains(pattern) {
            patched_content = patched_content.replace(pattern, replacement);
            patched = true;
        }
    }
    if patched {
        fs::write(path, patched_content)?;
        println!("cargo:warning=Successfully patched Boost hash.hpp for macOS compatibility");
    } else if content.contains("typedef T argument_type;") {
        println!("cargo:warning=Boost hash.hpp already properly patched, skipping");
    } else {
        println!("cargo:warning=Could not find pattern to patch in Boost hash.hpp");
    }
    Ok(())
}

fn create_terminal_wrapper() -> Result<(), Box<dyn std::error::Error>> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        return Ok(());
    }
    
    let terminal_h_path = "../third_party/dynarmic/src/dynarmic/ir/terminal.h";
    let wrapper_path = "../third_party/dynarmic/src/dynarmic/ir/terminal_wrapper.h";
    
    if Path::new(wrapper_path).exists() {
        println!("cargo:warning=Terminal wrapper already exists, skipping creation");
        return Ok(());
    }
    
    let _terminal_content = fs::read_to_string(terminal_h_path)?;
    
    let wrapper_content = r#"// Wrapper for terminal.h to fix circular dependency issues on Linux/Mac
#pragma once

// First define the forward declarations
namespace Dynarmic::IR::Term {
struct Invalid;
struct Interpret;
struct ReturnToDispatch;
struct LinkBlock;
struct LinkBlockFast;
struct PopRSBHint;
struct FastDispatchHint;
struct If;
struct CheckBit;
struct CheckHalt;
}

// Include boost headers
#include <boost/variant/variant.hpp>
#include <boost/recursive_wrapper.hpp>

// Now include the original header but with our fixes
#define BOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES
#include "dynarmic/ir/terminal.h"
#undef BOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES
"#;
    
    fs::write(wrapper_path, wrapper_content)?;
    println!("cargo:warning=Created terminal wrapper to fix circular dependency");
    
    Ok(())
}

fn patch_terminal_h() -> Result<(), Box<dyn std::error::Error>> {
    let terminal_h_path = "../third_party/dynarmic/src/dynarmic/ir/terminal.h";
    let path = Path::new(terminal_h_path);
    
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(path)?;
    
    if content.contains("BOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES") {
        return Ok(());
    }
    
    let mut lines: Vec<&str> = content.lines().collect();
    
    let mut insert_pos = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("#include <boost/variant/variant.hpp>") {
            insert_pos = i + 1;
            break;
        }
    }
    
    lines.insert(insert_pos, "#define BOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES");
    
    let patched_content = lines.join("\n");
    fs::write(path, patched_content)?;
    println!("cargo:warning=Successfully patched terminal.h");
    
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../third_party/dynarmic/src/dynarmic/dynarmic_interface.cpp");
    
    // Get target information
    let target = env::var("TARGET").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let is_msvc = target.contains("msvc");
    let is_apple = target_os == "macos" || target_os == "ios";
    let is_windows = target_os == "windows";
    
    // Print target information for debugging
    println!("cargo:warning=Building for target: {}", target);
    println!("cargo:warning=Target arch: {}", target_arch);
    
    // Configure environment for ARM64 on macOS
    if target_os == "macos" && target_arch == "aarch64" {
        println!("cargo:warning=Configuring build for ARM64 on macOS");
        unsafe {
            env::set_var("CMAKE_OSX_ARCHITECTURES", "arm64");
            env::set_var("MACOSX_DEPLOYMENT_TARGET", "13.0");
        }
    }
    
    if let Err(e) = patch_terminal_h() {
        println!("cargo:warning=Failed to patch terminal.h: {}", e);
    }
    
    // Get the correct paths
    let project_root = Path::new("..");
    let third_party_dir = project_root.join("third_party");
    let cpp_file = third_party_dir.join("dynarmic").join("src").join("dynarmic").join("dynarmic_interface.cpp");
    let dynarmic_dir = third_party_dir.join("dynarmic");
    let dynarmic_build_dir = dynarmic_dir.join("build");
    let dynarmic_src_dir = dynarmic_dir.join("src");
    
    println!("cargo:warning=== DYNARMIC BUILD ===");
    println!("cargo:warning=Current directory: {:?}", env::current_dir());
    println!("cargo:warning=Project root: {:?}", project_root);
    println!("cargo:warning=Target: {}", target);
    println!("cargo:warning=Host OS: {}", target_os);
    println!("cargo:warning=Our C++ interface file exists: {}", cpp_file.exists());
    
    // Create terminal wrapper for Linux/Mac
    if !is_windows {
        if let Err(e) = create_terminal_wrapper() {
            println!("cargo:warning=Failed to create terminal wrapper: {}", e);
        }
    }
    
    // Ensure submodules are initialized
    let fmt_path = dynarmic_dir.join("externals/fmt");
    let zydis_path = dynarmic_dir.join("externals/zydis");
    let mcl_path = dynarmic_dir.join("externals/mcl");
    
    if !fmt_path.exists() || !zydis_path.exists() || !mcl_path.exists() {
        println!("cargo:warning=Initializing Dynarmic submodules...");
        let status = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .current_dir(project_root)
            .status()
            .expect("Failed to run git submodule");
            
        if !status.success() {
            panic!("Failed to initialize main submodules");
        }
        
        let status = Command::new("git")
            .args(&["submodule", "update", "--init", "--recursive"])
            .current_dir(&zydis_path)
            .status()
            .expect("Failed to run git submodule for zydis");
        
        if !status.success() {
            panic!("Git submodule update failed with exit code: {}", status);
        }
        
        if is_apple {
            println!("cargo:warning=Applying macOS compatibility patches...");
            if let Err(e) = patch_boost_for_macos() {
                println!("cargo:warning=Failed to patch Boost: {}", e);
                panic!("Failed to patch Boost for macOS: {}", e);
            }
        }
    }
    
    // Clean build directory completely for ARM64 builds
    if target_os == "macos" && target_arch == "aarch64" {
        if dynarmic_build_dir.exists() {
            println!("cargo:warning=Removing entire build directory for clean ARM64 build...");
            if let Err(e) = std::fs::remove_dir_all(&dynarmic_build_dir) {
                println!("cargo:warning=Failed to remove build directory: {}", e);
            }
        }
    } else if dynarmic_build_dir.exists() {
        let cmake_cache = dynarmic_build_dir.join("CMakeCache.txt");
        if cmake_cache.exists() {
            println!("cargo:warning=Cleaning existing CMake cache to avoid conflicts...");
            if let Err(e) = std::fs::remove_file(&cmake_cache) {
                println!("cargo:warning=Failed to remove CMakeCache.txt: {}", e);
            }
        }
    }
    
    // Build Dynarmic if needed
    let dynarmic_lib_name = if is_msvc {
        "dynarmic.lib"
    } else if is_apple {
        "libdynarmic.a"
    } else {
        "libdynarmic.a"
    };
    
    let dynarmic_lib_path = if is_msvc {
        dynarmic_build_dir.join("src").join("dynarmic").join("Release").join(dynarmic_lib_name)
    } else {
        dynarmic_build_dir.join("src").join("dynarmic").join(dynarmic_lib_name)
    };
    
    if !dynarmic_lib_path.exists() {
        println!("cargo:warning=== BUILDING DYNARMIC ===");
        
        if !dynarmic_dir.exists() {
            panic!("Dynarmic source directory not found: {}", dynarmic_dir.display());
        }
        
        let cmake_lists = dynarmic_dir.join("CMakeLists.txt");
        if !cmake_lists.exists() {
            panic!("CMakeLists.txt not found in: {}", cmake_lists.display());
        }
        
        if let Err(e) = std::fs::create_dir_all(&dynarmic_build_dir) {
            panic!("Failed to create build directory: {}", e);
        }
        
        let dynarmic_dir_str = dynarmic_dir.to_string_lossy().into_owned();
        let dynarmic_build_dir_str = dynarmic_build_dir.to_string_lossy().into_owned();
        
        // Prepare base CMake arguments
        let mut cmake_args = vec![
            "-S", &dynarmic_dir_str,
            "-B", &dynarmic_build_dir_str,
            "-DDYNARMIC_USE_BUNDLED_EXTERNALS=ON",
            "-DDYNARMIC_TESTS=OFF",
            "-DDYNARMIC_ENABLE_ASM_SUPPORT=OFF",
            "-DDYNARMIC_EXAMPLES=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DDYNARMIC_WARNINGS_AS_ERRORS=OFF",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DZYDIS_BUILD_SHARED_LIB=OFF",
            "-DZYCORE_BUILD_SHARED_LIB=OFF",
            "-DZYDIS_BUILD_EXAMPLES=OFF",
            "-DZYDIS_BUILD_TOOLS=OFF",
            "-DZYAN_SYSTEM_ZYCORE=OFF",
            "-DZYDIS_STATIC_DEFINE=ON",
            "-DZYDIS_DEV_MODE=OFF",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            "-DBoost_NO_BOOST_CMAKE=ON",
            "-DBoost_NO_SYSTEM_PATHS=ON",
            "-DBOOST_ROOT=../third_party/ext-boost",
            "-DBoost_USE_STATIC_LIBS=ON",
        ];
        
        // Platform-specific CMake arguments
        if is_msvc {
            cmake_args.extend(&[
                "-DCMAKE_CXX_STANDARD=20",
                "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
                "-DCMAKE_CXX_FLAGS=/wd4100 /wd4189",
            ]);
        } else if is_apple {
            // Force ARM64 architecture for ARM64 targets
            cmake_args.extend(&[
                "-DCMAKE_CXX_STANDARD=17",
                "-DCMAKE_CXX_FLAGS=-fno-strict-aliasing -Wno-unused-parameter -Wno-incomplete-types",
                "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0",
                "-DCMAKE_C_COMPILER=/usr/bin/clang",
                "-DCMAKE_CXX_COMPILER=/usr/bin/clang++",
                "-DCMAKE_POLICY_VERSION_MINIMUM=3.10",
                "-DCMAKE_OSX_ARCHITECTURES=arm64", // Force ARM64 for aarch64 targets
            ]);
        } else {
            cmake_args.extend(&[
                "-DCMAKE_CXX_STANDARD=17",
                "-DCMAKE_CXX_FLAGS=-fno-strict-aliasing -Wno-unused-parameter -Wno-incomplete-types",
                "-DBOOST_DISABLE_THREADS=OFF",
                "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
                "-DDYNARMIC_ENABLE_PRECOMPILED_HEADERS=OFF",
            ]);
        }
        
        // Run CMake configure
        println!("cargo:warning=Running CMake configure...");
        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd.args(&cmake_args);
        
        // Set environment variables for ARM64 builds
        if target_os == "macos" && target_arch == "aarch64" {
            cmake_cmd.env("CMAKE_OSX_ARCHITECTURES", "arm64");
            cmake_cmd.env("MACOSX_DEPLOYMENT_TARGET", "13.0");
        }
        
        let cmake_status = cmake_cmd.status();
        
        match cmake_status {
            Ok(status) => {
                if status.success() {
                    println!("cargo:warning=✓ CMake configure successful");
                } else {
                    panic!("CMake configure failed with exit code: {}", status);
                }
            }
            Err(e) => {
                panic!("Failed to run CMake configure: {}", e);
            }
        }
        
        // Build Dynarmic and all its dependencies
        println!("cargo:warning=Building Dynarmic and dependencies...");
        let mut build_cmd = Command::new("cmake");
        build_cmd
            .arg("--build")
            .arg(&dynarmic_build_dir)
            .arg("--config")
            .arg("Release");
        
        // Only add Zycore and Zydis targets for x86_64
        if target.contains("x86_64") {
            build_cmd.arg("--target").arg("Zycore");
            build_cmd.arg("--target").arg("Zydis");
        }
        
        build_cmd
            .arg("--target").arg("dynarmic")
            .arg("--parallel")
            .arg(env::var("NUM_JOBS").unwrap_or_else(|_| "4".into()));
        
        if is_apple {
            build_cmd.env("MACOSX_DEPLOYMENT_TARGET", "13.0");
            if target_arch == "aarch64" {
                build_cmd.env("CMAKE_OSX_ARCHITECTURES", "arm64");
            }
        }
        
        let build_status = build_cmd.status();
        
        match build_status {
            Ok(status) => {
                if status.success() {
                    println!("cargo:warning=✓ Dynarmic built successfully");
                } else {
                    panic!("Dynarmic build failed with exit code: {}", status);
                }
            }
            Err(e) => {
                panic!("Failed to build Dynarmic: {}", e);
            }
        }
        
        if dynarmic_lib_path.exists() {
            println!("cargo:warning=✓ Dynarmic library found at: {}", dynarmic_lib_path.display());
        } else {
            panic!("Dynarmic library not found after build! Expected at: {}", dynarmic_lib_path.display());
        }
    } else {
        println!("cargo:warning=✓ Dynarmic already built");
    }
    
    // Compile our C++ interface
    println!("cargo:warning=== COMPILING OUR C++ INTERFACE ===");
    
    let include_paths = vec![
        dynarmic_src_dir.join("dynarmic"),
        dynarmic_dir.join("src"),
        dynarmic_dir.join("externals/fmt/include"),
        dynarmic_dir.join("externals/mcl/include"),
    ];
    
    let mut build = cc::Build::new();
    build
        .file(&cpp_file)
        .cpp(true)
        .warnings(false)
        .warnings_into_errors(false);
    
    // Platform-specific flags
    if is_msvc {
        build
            .flag("/std:c++20")
            .flag("/EHsc")
            .flag("/MD")
            .flag("/DWIN32_LEAN_AND_MEAN")
            .flag("/DNOMINMAX")
            .flag("/wd4100")
            .flag("/wd4189")
            .flag("/wd4458");
    } else {
        build
            .flag("-std=c++17")
            .flag("-fno-strict-aliasing")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-variable")
            .flag("-Wno-incomplete-types")
            .flag("-fexceptions")
            .flag("-DBOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES")
            .flag("-ftemplate-depth=1024");
        
        if target_os == "linux" || target_os == "macos" {
            // Additional flags for Unix-like systems
        }
        
        if is_apple {
            build
                .flag("-arch").flag(if target.starts_with("aarch64-") { "arm64" } else { "x86_64" })
                .define("__APPLE__", None);
                
            // Additional flags for ARM64 on macOS
            if target_arch == "aarch64" {
                build.flag("-mmacosx-version-min=13.0");
            }
        }
    }
    
    // Add include paths that exist
    let mut valid_paths = Vec::new();
    for path in &include_paths {
        if path.exists() {
            build.include(path);
            valid_paths.push(path);
            println!("cargo:warning=Added include path: {}", path.display());
        }
    }
    
    match build.try_compile("dynarmic_interface") {
        Ok(_) => {
            println!("cargo:warning=✓ Our C++ interface compiled successfully!");
        }
        Err(e) => {
            println!("cargo:warning=✗ C++ interface compilation failed: {}", e);
            
            // Try with C++14 if C++17 fails
            let mut build2 = cc::Build::new();
            build2
                .file(&cpp_file)
                .cpp(true)
                .warnings(false)
                .warnings_into_errors(false);
            
            if is_msvc {
                build2
                    .flag("/std:c++14")
                    .flag("/EHsc")
                    .flag("/MD")
                    .flag("/DWIN32_LEAN_AND_MEAN")
                    .flag("/DNOMINMAX")
                    .flag("/wd4100")
                    .flag("/wd4189")
                    .flag("/wd4458");
            } else {
                build2
                    .flag("-std=c++14")
                    .flag("-fno-strict-aliasing")
                    .flag("-Wno-unused-parameter")
                    .flag("-Wno-unused-variable")
                    .flag("-Wno-incomplete-types")
                    .flag("-fexceptions")
                    .flag("-DBOOST_VARIANT_DO_NOT_USE_VARIADIC_TEMPLATES")
                    .flag("-ftemplate-depth=1024");
                
                if target_os == "linux" || target_os == "macos" {
                    // Additional flags for Unix-like systems
                }
                
                if is_apple {
                    build2
                        .flag("-arch").flag(if target.starts_with("aarch64-") { "arm64" } else { "x86_64" })
                        .define("__APPLE__", None);
                        
                    // Additional flags for ARM64 on macOS
                    if target_arch == "aarch64" {
                        build2.flag("-mmacosx-version-min=13.0");
                    }
                }
            }
            
            for path in &valid_paths {
                build2.include(path);
            }
            
            match build2.try_compile("dynarmic_interface") {
                Ok(_) => {
                    println!("cargo:warning=✓ C++ interface compiled successfully with C++14!");
                }
                Err(e2) => {
                    panic!("Failed to compile C++ interface with both C++17 and C++14. Last error: {}", e2);
                }
            }
        }
    }
    
    // Link libraries
    println!("cargo:warning=== LINKING LIBRARIES ===");
    
    // Main Dynarmic library paths
    let lib_paths = if is_msvc {
        vec![
            dynarmic_build_dir.join("src").join("dynarmic").join("Release"),
            dynarmic_build_dir.join("lib").join("Release"),
        ]
    } else {
        vec![
            dynarmic_build_dir.join("src").join("dynarmic"),
            dynarmic_build_dir.join("lib"),
        ]
    };
    
    // Dependency library paths
    let dep_paths = if is_msvc {
        vec![
            dynarmic_build_dir.join("externals/fmt/Release"),
            dynarmic_build_dir.join("externals/zydis/Release"),
            dynarmic_build_dir.join("externals/mcl/src/Release"),
            dynarmic_build_dir.join("externals/zydis/zycore/Release"),
        ]
    } else {
        vec![
            dynarmic_build_dir.join("externals/fmt"),
            dynarmic_build_dir.join("externals/zydis"),
            dynarmic_build_dir.join("externals/mcl/src"),
            dynarmic_build_dir.join("externals/zydis/zycore"),
        ]
    };
    
    // Combine all paths
    let mut all_paths = lib_paths.clone();
    all_paths.extend(dep_paths);
    all_paths.extend(vec![
        dynarmic_build_dir.join("build"),
        dynarmic_build_dir.join("build").join("Release"),
        dynarmic_build_dir.join("build").join("RelWithDebInfo"),
        dynarmic_build_dir.join("externals").join("zydis").join("dependencies").join("zycore"),
    ]);
    
    // Add all library paths to linker search
    for lib_path in &all_paths {
        if lib_path.exists() {
            let absolute_path = std::fs::canonicalize(lib_path)
                .expect("Failed to get absolute path");
            println!("cargo:rustc-link-search=native={}", absolute_path.display());
            println!("cargo:warning=Added library path: {}", absolute_path.display());
        }
    }
    
    // Link required libraries
    println!("cargo:rustc-link-lib=static=mcl");
    println!("cargo:rustc-link-lib=static=dynarmic");
    println!("cargo:rustc-link-lib=static=fmt");
    
    if target.contains("x86_64") {
        println!("cargo:rustc-link-lib=static=Zydis");
        println!("cargo:rustc-link-lib=static=Zycore");
        println!("cargo:warning=Linking x86_64-specific libraries: Zydis, Zycore");
    } else {
        println!("cargo:warning=Skipping x86_64-specific libraries on ARM64");
    }
    
    if is_windows {
        if is_msvc {
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
        } else {
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=shell32");
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
        }
    } else if is_apple {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        
        // Additional linking for ARM64 on macOS
        if target_arch == "aarch64" {
            println!("cargo:rustc-link-arg=-mmacosx-version-min=13.0");
        }
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
    println!("cargo:warning=== BUILD COMPLETED SUCCESSFULLY ===");
}
