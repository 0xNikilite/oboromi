use std::{env, fs, path::{Path, PathBuf}, process::Command};

const REVISION: &str = "cad1e2c0921110e34661b9b0a4ed20d06290434b";

fn run(command: &mut Command) {
    let status = command.status().expect("run Dynarmic build command");
    assert!(status.success(), "Dynarmic build command failed: {command:?}");
}

fn build_dynarmic() {
    println!("cargo:rerun-if-changed=dynarmic/CMakeLists.txt");
    println!("cargo:rerun-if-changed=dynarmic/bridge.cpp");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source = out.join("dynarmic-src");
    if !source.join(".git").exists() {
        run(Command::new("git").args(["clone", "--recursive", "https://codeberg.org/yuzu-emu/dynarmic.git"]).arg(&source));
    }
    let head = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&source).output().expect("read Dynarmic revision");
    if String::from_utf8_lossy(&head.stdout).trim() != REVISION {
        run(Command::new("git").args(["fetch", "origin", REVISION]).current_dir(&source));
        run(Command::new("git").args(["checkout", "--detach", REVISION]).current_dir(&source));
        run(Command::new("git").args(["submodule", "update", "--init", "--recursive"]).current_dir(&source));
    }
    let build = out.join("dynarmic-build");
    let lib = out.join("dynarmic-lib");
    fs::create_dir_all(&lib).unwrap();
    run(Command::new("cmake")
        .args(["-S", "dynarmic", "-B"]).arg(&build)
        .arg(format!("-DDYNARMIC_SOURCE={}", source.display()))
        .arg(format!("-DCMAKE_ARCHIVE_OUTPUT_DIRECTORY={}", lib.display()))
        .args(["-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_POLICY_VERSION_MINIMUM=3.5", "-DDYNARMIC_FRONTENDS=A64", "-DDYNARMIC_TESTS=OFF", "-DDYNARMIC_USE_LLVM=OFF", "-DBUILD_TESTING=OFF"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR"))));
    run(Command::new("cmake").args(["--build"]).arg(&build).args(["--target", "oboromi_dynarmic", "--parallel"]));
    println!("cargo:rustc-link-search=native={}", lib.display());
    for name in ["oboromi_dynarmic", "dynarmic", "mcl", "fmt", "Zydis", "Zycore"] {
        println!("cargo:rustc-link-lib=static={name}");
    }
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

#[allow(dead_code)]
struct IpcMethod {
    pub index: u32,
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

impl IpcMethod {
    fn write_rs<W: std::fmt::Write>(&self, w: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = String::new();
        let mut cnt = 0;
        for e in self.outputs.iter() {
            if e.starts_with("buffer") {
                args += &format!("a{cnt}: OutBuffer<{}>, ", &e[7..e.len() - 1]);
            } else if e.starts_with("object") {
                args += &format!("a{cnt}: Out<{}>, ", &e[7..e.len() - 1]);
            } else {
                args += &format!("a{cnt}: Out<{}>, ", e);
            }
            cnt += 1;
        }
        for e in self.inputs.iter() {
            if e.starts_with("buffer") {
                args += &format!("a{cnt}: InBuffer<{}>, ", &e[7..e.len() - 1]);
            } else if e.starts_with("object") {
                args += &format!("a{cnt}: {}, ", &e[7..e.len() - 1]);
            } else {
                args += &format!("a{cnt}: {}, ", e);
            }
            cnt += 1;
        }
        writeln!(w, "    pub fn {}({args}) {{}}", self.name)?;
        Ok(())
    }
}

struct IpcInterface {
    pub methods: Vec<IpcMethod>,
    pub name: String,
}

impl IpcInterface {
    fn write_rs<W: std::fmt::Write>(&self, w: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(w, "impl {} {{", self.name)?;
        for m in self.methods.iter() {
            m.write_rs(w)?;
        }
        writeln!(w, "}}")?;
        Ok(())
    }
}

fn parse_param_list(tokens: &[&str], is_input: bool) -> (Vec<String>, Option<usize>) {
    let mut v = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if is_input && tokens[i] == "->" {
            return (v, Some(i + 1));
        }
        let arg = tokens[i].split(&[',', ')', ';', '(']).collect::<Vec<_>>();
        if let Some(&first) = arg.first() {
            if !first.is_empty() {
                if first.starts_with("buffer") {
                    if i + 2 < tokens.len() {
                        let joined = format!("{},{},{}", tokens[i], tokens[i + 1], tokens[i + 2]);
                        let cleaned: String = joined.split(&[',', ')', ';']).collect();
                        v.push(cleaned);
                        i += 2;
                    }
                } else {
                    v.push(first.to_string());
                }
            }
        }
        i += 1;
    }
    (v, None)
}

fn generate<W: std::fmt::Write>(w: &mut W, data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ifaces: Vec<IpcInterface> = Vec::new();

    for line in data.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() || tokens[0].starts_with('#') {
            continue;
        }

        if tokens[0] == "interface" {
            if tokens.len() < 2 {
                return Err(format!("interface missing name: `{line}`").into());
            }
            ifaces.push(IpcInterface {
                methods: Vec::new(),
                name: tokens[1].to_string(),
            });
        } else if tokens[0] == "type" {
        } else if tokens[0].starts_with('[') {
            let iface = ifaces
                .last_mut()
                .ok_or_else(|| format!("method outside interface: `{line}`"))?;

            if tokens.len() < 2 {
                return Err(format!("malformed method line: `{line}`").into());
            }

            let index_str = tokens[0]
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .ok_or_else(|| format!("malformed method index: `{}`", tokens[0]))?;
            let index = index_str
                .parse::<u32>()
                .map_err(|_| format!("non-numeric index: `{index_str}`"))?;

            let params: Vec<&str> = tokens[1].split(&['(', ',', ')']).collect();
            let method_name = params[0].to_string();

            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            if params.len() > 1 && !params[1].is_empty() {
                inputs.push(params[1].to_string());
            }

            let (p_inputs, fidx) = parse_param_list(&tokens[2..], true);
            inputs.extend(p_inputs);

            if let Some(fi) = fidx {
                (outputs, _) = parse_param_list(&tokens[2 + fi..], false);
            }

            iface.methods.push(IpcMethod {
                index,
                name: method_name,
                inputs,
                outputs,
            });
        }
    }

    for iface in ifaces {
        iface.write_rs(w)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    build_dynarmic();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/nn/auto.defs");
    let mut translator = vec![std::path::PathBuf::from("src/gpu/spirv.rs"), "src/gpu/sm86_decoder_generated.rs".into()];
    for entry in std::fs::read_dir("src/gpu/sm86")? {
        translator.push(entry?.path());
    }
    translator.sort();
    println!("cargo:rerun-if-changed=src/gpu/sm86");
    let mut hasher = std::hash::DefaultHasher::new();
    for path in &translator {
        println!("cargo:rerun-if-changed={}", path.display());
        std::hash::Hash::hash(&std::fs::read(path)?, &mut hasher);
    }
    println!("cargo:rustc-env=TRANSLATOR_HASH={:016x}", std::hash::Hasher::finish(&hasher));

    let data = std::fs::read_to_string("src/nn/auto.defs").unwrap_or_default();
    let mut w = String::new();
    generate(&mut w, &data)?;

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("ipc_generated.rs");
    std::fs::write(&dest_path, &w)?;

    Ok(())
}
