use std::collections::BTreeMap;
use std::fs;

fn main() {
    let input = fs::read_to_string("core/src/gpu/sm86_opcodes.txt")
        .expect("Cannot read sm86_opcodes.txt");

    let mut entries: BTreeMap<u32, String> = BTreeMap::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            eprintln!("Skipping malformed line: {}", line);
            continue;
        }
        let opcode = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("Bad hex: {}", parts[0]));
        let handler = parts[1].to_string();

        entries.entry(opcode).or_insert(handler);
    }

    let mut out = String::new();
    out.push_str("// Auto-generated from sm86_opcodes.txt — do not edit\n");
    out.push_str(&format!("// {} unique opcodes\n\n", entries.len()));
    out.push_str("impl<'a> Decoder<'a> {\n");
    out.push_str("    pub fn translate(&mut self, inst: u128) {\n");
    out.push_str("        let opcode = ((((inst >> 91) & 0x1) << 12) | (inst & 0xfff)) as u32;\n");
    out.push_str("        match opcode {\n");

    for (opcode, handler) in &entries {
        let handler_fn = match handler.as_str() {
            "break" | "match" | "yield" => format!("{}_", handler),
            _ => handler.clone(),
        };
        out.push_str(&format!(
            "            {:#06x} => self.{}(inst),\n",
            opcode, handler_fn
        ));
    }

    out.push_str("            _ => panic!(\"Unknown SM86 opcode: {:#06x}\", opcode),\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    fs::write("core/src/gpu/sm86_decoder_generated.rs", &out)
        .expect("Cannot write output");

    println!("Generated {} opcode entries", entries.len());
}
