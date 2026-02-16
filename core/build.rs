struct IpcMethod {
    pub index: u32,
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>
}
impl IpcMethod {
    pub fn write_rs<W: std::fmt::Write>(&self, w: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = String::new();
        let mut cnt = 0;
        for e in self.outputs.iter() {
            if e.starts_with("buffer") {
                args = format!("{args}a{cnt}: OutBuffer<{}>, ", e[7..e.len() - 1].to_string());
            } else if e.starts_with("object") {
                args = format!("{args}a{cnt}: Out<{}>, ", e[7..e.len() - 1].to_string());
            } else {
                args = format!("{args}a{cnt}: Out<{}>, ", e);
            }
            cnt += 1;
        }
        for e in self.inputs.iter() {
            if e.starts_with("buffer") {
                args = format!("{args}a{cnt}: InBuffer<{}>, ", e[7..e.len() - 1].to_string());
            } else if e.starts_with("object") {
                args = format!("{args}a{cnt}: {}, ", e[7..e.len() - 1].to_string());
            } else {
                args = format!("{args}a{cnt}: {}, ", e);
            }
            cnt += 1;
        }
        writeln!(w, "    pub fn {}({args}) {{}}", self.name)?;
        Ok(())
    }
}
struct IpcInterface {
    pub methods: Vec<IpcMethod>,
    pub name: String
}
impl IpcInterface {
    pub fn write_rs<W: std::fmt::Write>(&self, w: &mut W) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(w, "impl {} {{", self.name)?;
        for m in self.methods.iter() {
            m.write_rs(w)?;
        }
        writeln!(w, "}}")?;
        Ok(())
    }
}

pub fn parse_param_list(tokens: &[&str], is_input: bool) -> (Vec<String>, Option<usize>) {
    let mut v = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let arg = tokens[i].split(&[',', ')', ';', '(']).collect::<Vec<_>>();
        if is_input && tokens[i] == "->" {
            return (v, Some(i + 1));
        }
        if arg.len() > 0 && arg[0].len() > 0 {
            if arg[0].starts_with("buffer") {
                v.push(format!("{},{},{}", tokens[i], tokens[i + 1], tokens[i + 2]).split(&[',', ')', ';']).collect::<String>());
                i += 2;
            } else {
                v.push(arg[0].to_string());
            }
        }
        i += 1;
    }
    (v, None)
}

pub fn write_rs<W: std::fmt::Write>(w: &mut W, data: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut ifaces = Vec::<IpcInterface>::new();
    for line in data.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.len() == 0 || tokens[0].starts_with("#") {
            // comment ignore
            continue;
        } else if tokens[0] == "interface" {
            ifaces.push(IpcInterface{
                methods: Vec::default(),
                name: tokens[1].to_string()
            });
        } else if tokens[0] == "type" {
            
        } else if tokens[0].starts_with("[") {            
            let params = tokens[1].split(&['(', ',', ')']).collect::<Vec<_>>();
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();
            // Fn(OneArg)
            if params.len() > 1 && params[1].len() > 0 {
                inputs.push(params[1].to_string());
            }

            let (p_inputs, fidx) = parse_param_list(&tokens[2..], true);
            for e in p_inputs {
                inputs.push(e);
            }
            if let Some(fidx) = fidx {
                (outputs, _) = parse_param_list(&tokens[(2 + fidx)..], false);
            }

            ifaces.last_mut().unwrap().methods.push(IpcMethod{
                index: tokens[0][1..tokens[0].len() - 1].parse::<u32>()?,
                name: params[0].to_string(),
                inputs,
                outputs,
            });
        }
    }
    for e in ifaces {
        e.write_rs(w)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/nn/auto.defs");

    let mut w = String::new();
    write_rs(&mut w, std::fs::read_to_string("src/nn/auto.defs")?)?;
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("ipc_generated.rs");
    std::fs::write(&dest_path, w).unwrap();

    Ok(())
}
