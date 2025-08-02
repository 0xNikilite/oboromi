use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../third_party/dynarmic/src/dynarmic/dynarmic_interface.cpp");
    
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
    println!("cargo:warning=Our C++ interface file exists: {}", cpp_file.exists());
    
    // Ensure submodules are initialized
    let fmt_path = dynarmic_dir.join("externals/fmt");
    let zydis_path = dynarmic_dir.join("externals/zydis");
    let mcl_path = dynarmic_dir.join("externals/mcl");
    
    if !fmt_path.exists() || !zydis_path.exists() || !mcl_path.exists() {
        println!("cargo:warning=Initializing Dynarmic submodules...");
        let status = Command::new("git")
            .args(&["submodule", "update", "--init", "--recursive"])
            .current_dir(&dynarmic_dir)
            .status()
            .expect("Failed to run git submodule");
        
        if !status.success() {
            panic!("Git submodule update failed with exit code: {}", status);
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
    let dynarmic_lib_path = dynarmic_build_dir.join("src").join("dynarmic").join("Release").join("dynarmic.lib");
    
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
        
        // Run CMake configure
        println!("cargo:warning=Running CMake configure...");
        let cmake_status = Command::new("cmake")
            .args(&[
                "-S", &dynarmic_dir.to_string_lossy(),
                "-B", &dynarmic_build_dir.to_string_lossy(),
                "-DDYNARMIC_BUNDLES=ON",
                "-DDYNARMIC_TESTS=OFF",
                "-DDYNARMIC_ENABLE_ASM_SUPPORT=OFF",
                "-DDYNARMIC_EXAMPLES=OFF",
                "-DCMAKE_BUILD_TYPE=Release",
                "-DCMAKE_POLICY_VERSION_MINIMUM=3.5",
                "-DCMAKE_CXX_FLAGS=/wd4100 /wd4189",
            ])
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
        
        // Build Dynarmic
        println!("cargo:warning=Building Dynarmic...");
        let build_status = Command::new("cmake")
            .args(&[
                "--build", &dynarmic_build_dir.to_string_lossy(),
                "--config", "Release",
                "--target", "dynarmic",
                "--parallel", "4",
            ])
            .status();
        
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
        .warnings_into_errors(false)
        .debug(true)
        .flag("/std:c++20")
        .flag("/EHsc")
        .flag("/MD")
        .flag("/DWIN32_LEAN_AND_MEAN")
        .flag("/DNOMINMAX")
        .flag("/wd4100")
        .flag("/wd4189")
        .flag("/wd4458");
    
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
            
            // Try with C++14
            let mut build2 = cc::Build::new();
            build2
                .file(&cpp_file)
                .cpp(true)
                .warnings(false)
                .warnings_into_errors(false)
                .debug(true)
                .flag("/std:c++14")
                .flag("/EHsc")
                .flag("/MD")
                .flag("/DWIN32_LEAN_AND_MEAN")
                .flag("/DNOMINMAX")
                .flag("/wd4100")
                .flag("/wd4189")
                .flag("/wd4458");
            
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
    let lib_paths = vec![
        dynarmic_build_dir.join("src").join("dynarmic").join("Release"),
        dynarmic_build_dir.join("lib").join("Release"),
    ];
    
    // Dependency library paths
    let dep_paths = vec![
        dynarmic_build_dir.join("externals/fmt/Release"),
        dynarmic_build_dir.join("externals/zydis/Release"),
        dynarmic_build_dir.join("externals/mcl/src/Release"),
    ];
    
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
    
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=msvcrt");
        // Windows specific libraries
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=gdi32");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    
    println!("cargo:warning=== BUILD COMPLETED SUCCESSFULLY ===");
}