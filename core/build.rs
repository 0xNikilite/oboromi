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
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/nn/auto.defs");

    let data = std::fs::read_to_string("src/nn/auto.defs").unwrap_or_default();
    let mut w = String::new();
    generate(&mut w, &data)?;

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("ipc_generated.rs");
    std::fs::write(&dest_path, &w)?;

    Ok(())
}
