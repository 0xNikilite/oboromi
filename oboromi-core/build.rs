use std::env;
use std::path::Path;
use std::process::Command;
use std::fs;

fn patch_boost_for_macos() -> Result<(), Box<dyn std::error::Error>> {
    // This patch is specifically for macOS to fix the Boost hash issue
    if !cfg!(target_os = "macos") {
        return Ok(());
    }

    let file_path = "../third_party/ext-boost/boost/container_hash/hash.hpp";
    let path = Path::new(file_path);
    
    if !path.exists() {
        println!("cargo:warning=Boost hash.hpp not found at {}, skipping patch", file_path);
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    
    let target = "struct hash_base : std::unary_function<T, std::size_t> {};";
    
    if content.contains(target) {
        let patched_content = content.replace(
            target,
            "struct hash_base { typedef T argument_type; typedef std::size_t result_type; };"
        );
        fs::write(path, patched_content)?;
        println!("cargo:warning=Successfully patched Boost for macOS compatibility");
        println!("cargo:warning=Replaced unary_function inheritance with typedefs");
    } else {
        let previous_target = "struct hash_base : boost::unary_function<T, std::size_t> {};";
        if content.contains(previous_target) {
            let patched_content = content.replace(
                previous_target,
                "struct hash_base { typedef T argument_type; typedef std::size_t result_type; };"
            );
            fs::write(path, patched_content)?;
            println!("cargo:warning=Updated Boost patch for macOS compatibility");
            println!("cargo:warning=Replaced boost::unary_function with typedefs");
        } else if content.contains("typedef T argument_type;") {
            println!("cargo:warning=Boost already properly patched, skipping");
        } else {
            println!("cargo:warning=Could not find target string in Boost file, patch may not be needed");
        }
    }

    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../third_party/dynarmic/src/dynarmic/dynarmic_interface.cpp");
    
    // Get the target triple and host OS
    let target = env::var("TARGET").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let is_msvc = target.contains("msvc");
    let is_apple = target_os == "macos" || target_os == "ios";
    let is_windows = target_os == "windows";
    
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
    
    // Ensure submodules are initialized
    let fmt_path = dynarmic_dir.join("externals/fmt");
    let zydis_path = dynarmic_dir.join("externals/zydis");
    let mcl_path = dynarmic_dir.join("externals/mcl");
    
    if !fmt_path.exists() || !zydis_path.exists() || !mcl_path.exists() {
        println!("cargo:warning=Initializing Dynarmic submodules...");

        // Initialize main submodules
        let status = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .current_dir(project_root)
            .status()
            .expect("Failed to run git submodule");
            
        if !status.success() {
            panic!("Failed to initialize main submodules");
        }

        // Initialize zydis submodules
        let status = Command::new("git")
            .args(&["submodule", "update", "--init", "--recursive"])
            .current_dir(&zydis_path)
            .status()
            .expect("Failed to run git submodule for zydis");
        
        if !status.success() {
            panic!("Git submodule update failed with exit code: {}", status);
        }
        
        // Apply macOS patch after submodule initialization
        if is_apple {
            println!("cargo:warning=Applying macOS compatibility patches...");
            if let Err(e) = patch_boost_for_macos() {
                println!("cargo:warning=Failed to patch Boost: {}", e);
                panic!("Failed to patch Boost for macOS: {}", e);
            }
        }
    }
    
    // Check if we need to clean the build directory
    if dynarmic_build_dir.exists() {
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
        
        // Check if dynarmic source exists
        if !dynarmic_dir.exists() {
            panic!("Dynarmic source directory not found: {}", dynarmic_dir.display());
        }
        
        let cmake_lists = dynarmic_dir.join("CMakeLists.txt");
        if !cmake_lists.exists() {
            panic!("CMakeLists.txt not found in: {}", cmake_lists.display());
        }
        
        // Create build directory
        if let Err(e) = std::fs::create_dir_all(&dynarmic_build_dir) {
            panic!("Failed to create build directory: {}", e);
        }
        
        // Create owned strings for CMake arguments
        let dynarmic_dir_str = dynarmic_dir.to_string_lossy().into_owned();
        let dynarmic_build_dir_str = dynarmic_build_dir.to_string_lossy().into_owned();
        
        // Prepare CMake arguments
        let mut cmake_args = vec![
            "-S", &dynarmic_dir_str,
            "-B", &dynarmic_build_dir_str,
            "-DDYNARMIC_USE_BUNDLED_EXTERNALS=ON",
            "-DDYNARMIC_TESTS=OFF",
            "-DDYNARMIC_ENABLE_ASM_SUPPORT=OFF",
            "-DDYNARMIC_EXAMPLES=OFF",
            "-DCMAKE_BUILD_TYPE=Release",
            "-DDYNARMIC_WARNINGS_AS_ERRORS=OFF",
            "-DZYDIS_BUILD_SHARED_LIB=OFF",
            "-DZYDIS_BUILD_EXAMPLES=OFF",
            "-DZYDIS_BUILD_TOOLS=OFF",
            "-DZYAN_SYSTEM_ZYCORE=OFF",
            "-DZYDIS_STATIC_DEFINE=ON",
            "-DZYDIS_DEV_MODE=OFF",
        ];
        
        // Add platform-specific flags
        if is_msvc {
            cmake_args.push("-DCMAKE_CXX_FLAGS=/wd4100 /wd4189");
        } else if is_apple {
            // Use Xcode toolchain explicitly
            cmake_args.push("-DCMAKE_OSX_DEPLOYMENT_TARGET=11.0");
            cmake_args.push("-DCMAKE_OSX_ARCHITECTURES=arm64");
            cmake_args.push("-DCMAKE_C_COMPILER=/usr/bin/clang");
            cmake_args.push("-DCMAKE_CXX_COMPILER=/usr/bin/clang++");
            cmake_args.push("-DCMAKE_CXX_FLAGS=-Wno-unused-parameter -Wno-unused-variable");
        } else {
            cmake_args.push("-DCMAKE_CXX_FLAGS=-Wno-unused-parameter -Wno-unused-variable");
        }
        
        // Run CMake configure
        println!("cargo:warning=Running CMake configure...");
        let cmake_status = Command::new("cmake")
            .args(&cmake_args)
            .status();
        
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
            .arg("Release")
            .arg("--target")
            .arg("all")  // Build all targets, not just dynarmic
            .arg("--parallel")
            .arg("4");
        
        if is_apple {
            build_cmd.env("MACOSX_DEPLOYMENT_TARGET", "11.0");
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
            .flag("/wd4100")    // unreferenced formal parameter
            .flag("/wd4189")    // local variable initialized but not referenced
            .flag("/wd4458");   // declaration hides class member
    } else {
        build
            .flag("-std=c++20")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-variable")
            .flag("-fexceptions");
        
        if is_apple {
            build
                .flag("-arch")
                .flag("arm64")
                .define("__APPLE__", None);
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
            
            // Try with C++14 if C++20 fails
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
                    .flag("-Wno-unused-parameter")
                    .flag("-Wno-unused-variable")
                    .flag("-fexceptions");
                
                if is_apple {
                    build2
                        .flag("-arch")
                        .flag("arm64")
                        .define("__APPLE__", None);
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
                    panic!("Failed to compile C++ interface with both C++20 and C++14. Last error: {}", e2);
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
    println!("cargo:rustc-link-lib=static=zydis");
    println!("cargo:rustc-link-lib=static=zycore");
    
    if is_windows {
        println!("cargo:rustc-link-lib=dylib=msvcrt");
        // Windows specific libraries
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=gdi32");
    } else if is_apple {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    println!("cargo:warning=== BUILD COMPLETED SUCCESSFULLY ===");
}
