use std::io::Write;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum ParserState {
    None,
    Opcodes,
    Encoding,
    Class
}

#[derive(Default, Debug, Clone)]
struct ParserField {
    name: String,
    total_bits: usize,
    bits: Vec<(usize, usize)>,
    value: Option<usize>
}

#[derive(Default, Clone, Debug)]
struct ParserInst {
    name: String,
    op: usize,
    fixed_fields: Vec<ParserField>,
    fields: Vec<ParserField>
}

fn collect_sass_instructions(f: &str) -> Result<Vec<ParserInst>, Box<dyn std::error::Error>> {
    let mut instructions = Vec::new();
    let mut dfa_state = ParserState::None;
    for line in f.lines() {
        if line == "OPCODES" {
            dfa_state = ParserState::Opcodes;
        } else if line == "ENCODING" {
            dfa_state = ParserState::Encoding;
        } else if line == "CLASS" {
            dfa_state = ParserState::Class;
        } else if dfa_state == ParserState::Opcodes && line.starts_with("        ") {
            let mut iter = line.split("=").map(|x| x.trim());
            let name = iter.next().unwrap();
            if !name.ends_with("cbu_pipe") && !name.ends_with("mio_pipe") && !name.ends_with("int_pipe") 
            && !name.ends_with("lite_pipe") && !name.ends_with("fe_pipe") && !name.ends_with("malighter_pipe") 
            && !name.ends_with("fp16_pipe") && !name.ends_with("udp_pipe") && !name.ends_with("ttu_pipe") {
                let bits = iter.next().unwrap().trim_start_matches("0b").trim_end_matches(";");
                //writeln!(w, "{name}={bits}");
                instructions.push(ParserInst{
                    name: name.to_string(),
                    op: usize::from_str_radix(bits, 2)?,
                    fixed_fields: Vec::new(),
                    fields: Vec::new()
                });
            }
        } else if dfa_state == ParserState::Encoding && line.starts_with("BITS") {
            //writeln!(w, "{}", line);
            let tpt = line
                .split_once("=")
                .unwrap();
            let segments = tpt.0.split(",");
            for seg in segments {
                let mut pairs = seg
                    .trim_start_matches("BITS_").trim()
                    .split("_");
                let mut field: ParserField = ParserField::default();
                field.value = tpt.1.parse::<usize>().ok();
                field.total_bits = pairs.next().unwrap().parse::<usize>().unwrap();
                let mut rem_size = field.total_bits;
                while rem_size > 0 {
                    let (end, start) = (
                        pairs.next().unwrap().parse::<usize>().unwrap(),
                        pairs.next().unwrap().parse::<usize>().unwrap()
                    );
                    let bit_size = (end - start) + 1;
                    rem_size -= bit_size;
                    field.bits.push((start, end));
                }
                field.name = pairs.collect::<Vec<_>>().join("_").to_lowercase();
                if field.name == "opcode" { //horrible hack for opcodes
                    field.value = Some(instructions.last_mut().unwrap().op);
                }
                field.name = field.name.replace(".", "_");
                if field.value.is_some() {
                    instructions.last_mut().unwrap().fixed_fields.push(field);
                } else {
                    instructions.last_mut().unwrap().fields.push(field);
                }
            }
        }
    }

    /*for i in 0..instructions.len() {
        let mut count = 0;
        loop {
            let new_name = if count == 0 {
                format!("{}", instructions[i].name)
            } else {
                format!("{}_{count}", instructions[i].name)
            };
            let found = instructions.iter()
                .find(|&e| e.name == new_name).is_some();
            if !found {
                if count > 0 {
                    instructions[i].name = new_name;
                }
                break;
            }
            count += 1;
        }
    }*/
    Ok(instructions)
}

fn get_mask(start: usize, end: usize) -> usize {
    let count = (end - start) + 1;
    (1 << count) - 1
}

fn get_stmt_mask(name: &str, start: usize, end: usize) -> String {
    if start > 0 {
        format!("({name} >> {start}) & 0x{:x}", get_mask(start, end))
    } else {
        format!("{name} & 0x{:x}", get_mask(start, end))
    }
}

fn get_multi_merge_stmt(name: &str, total_bits: usize, bits: &Vec<(usize, usize)>) -> String {
    let mut s = String::new();
    let mut rem_bits = total_bits;
    for e in bits {
        rem_bits -= e.1 - e.0 + 1;
        s = format!("{s}{}(({}) << {rem_bits})",
            if s.is_empty() { "" } else { " | " },
            get_stmt_mask("inst", e.0, e.1),
        );
    }
    s
}

fn get_multi_cmp_stmt(total_bits: usize, bits: &Vec<(usize, usize)>, value: usize) -> String {
    let mut s = String::new();
    let mut rem_bits = total_bits;
    for e in bits {
        rem_bits -= e.1 - e.0 + 1;
        s = format!("{s}{}(({}) << {rem_bits})",
            if s.is_empty() { "" } else { " | " },
            get_stmt_mask("inst", e.0, e.1),
        );
    }
    format!("({s}) == 0x{value:0x}")
}

fn get_canon_inst_name(s: &str) -> String {
    let ns = s.replace(".", "_");
    if ns == "match" || ns == "break" || ns == "yield" {
        format!("{ns}_")
    } else {
        ns
    }
}

fn write_rust<W: std::fmt::Write>(w: &mut W, list: &Vec<ParserInst>) -> Result<(), Box<dyn std::error::Error>> {
    let mut instructions = list.clone();
    for iter in instructions.iter_mut() {
        let mut i = 0;
        while i < iter.fields.len() {
            if iter.fields[i].bits.len() == 0 {
                writeln!(w, "// removed empty field #{i} {} of {}", iter.fields[i].name, iter.name)?;
                iter.fields.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
    for e in instructions.iter_mut() {
        e.fields.sort_by(|a, b| a.bits[0].0.cmp(&b.bits[0].0));
    }
    instructions.sort_by(|a, b| a.name.cmp(&b.name));
    writeln!(w, "// this file is autogenered, do not modify")?;
/*    writeln!(w, r"pub struct Decoder {{
    /* symbolic only */
    ir: &mut spirv::Emitter
}}")?;*/
    writeln!(w, "impl<'a> Decoder<'a> {{")?;
    writeln!(w, "\t// {} instructions", instructions.len())?;
    writeln!(w, "\tpub fn translate(&self, inst: u128) {{")?;
    writeln!(w, "\t\tif false {{")?;
    writeln!(w, "\t\t\tunreachable!();")?;
    write!(w, "\t\t}} ")?;
    for inst in instructions.iter() {
        if let Some(opcode_field) = inst.fixed_fields.iter().find(|&e| e.name == "opcode") {
            let b_opcode = &opcode_field.bits;
            assert!(b_opcode.len() == 2);
            write!(w, "else if /*{}*/ ", inst.name)?;
            for ff in inst.fixed_fields.iter() {
                write!(w, "{}", get_multi_cmp_stmt(ff.total_bits, &ff.bits, ff.value.unwrap()))?;
            }
            writeln!(w, " {{")?;
            writeln!(w, "\t\t\tself.{}(inst);", get_canon_inst_name(&inst.name.to_lowercase()))?;
            write!(w, "\t\t}} ")?;
        }
    }
    writeln!(w, "")?;
    writeln!(w, "\t}}")?;
    writeln!(w, "}}")?;

    let mut seen_names = std::collections::BTreeSet::<String>::new();
    writeln!(w, "/*")?;
    writeln!(w, "impl Decoder {{")?;
    for inst in instructions.iter() {
        if let Some(opcode_field) = inst.fixed_fields.iter().find(|&e| e.name == "opcode") {
            if seen_names.contains(&inst.name) {
                //writeln!(w, "// dup {}", inst.name)?;
                continue;
            }
            seen_names.insert(inst.name.clone());
            
            let b_opcode = &opcode_field.bits;
            assert!(b_opcode.len() == 2);
            writeln!(w, "\tpub fn {}(&self, inst: u128) {{", get_canon_inst_name(&inst.name.to_lowercase()))?;
            for f in inst.fields.iter() {
                writeln!(w, "\t\tlet _{} = {};", f.name, get_multi_merge_stmt("inst", f.total_bits, &f.bits))?;
            }
            writeln!(w, "\t\ttodo!();")?;
            writeln!(w, "\t}}")?;
        }
    }
    writeln!(w, "}}")?;
    writeln!(w, "*/")?;
    Ok(())
}

fn get_html_field_color(s: &str) -> u32 {
    if s.starts_with("opcode") {
        0xffa0a0
    } else if s == "ra" {
        0xa0ffa0
    } else if s == "rb" {
        0xffa0ff
    } else if s == "rd" {
        0xa0ffff
    } else if s.starts_with("pg") {
        0xe0e0ff
    } else if s.is_empty() {
        0xe0e0e0
    } else {
        0xffffff
    }
}

fn get_html_field_name(_op: usize, f: &ParserField) -> String {
    f.name.clone()
    /*if f.name == "opcode" {
        format!("0x{:x}", op)
    } else {
        f.value.map(|e| format!("0x{e:x}"))
            .unwrap_or(f.name.clone())
    }*/
}

fn get_html_masked_value(ao: usize, f: &ParserField) -> usize {
    for i in 0..f.bits.len() {
        let count = f.bits[i].1 - f.bits[i].0 + 1;
        let offset = f.bits[i].0;
        if ao >= offset && ao <= offset + count {
            return get_mask(f.bits[i].0, f.bits[i].1);
        }
    }
    unreachable!()
}

fn write_html<W: std::fmt::Write>(w: &mut W, list: &Vec<ParserInst>) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(w, r#"<!DOCTYPE html>
<style>
table, th, td {{
  border: 1px solid black;
  border-collapse: collapse;
}}
</style>
<html>
    <body>
"#)?;
    writeln!(w, "<h1>SM86_0 (Ampere)</h1>")?;
    /*
    writeln!(w, "<a href='https://github.com/0xD0GF00D/DocumentSASS/blob/main/NOTES.md'>NOTES</a></br>")?;
    writeln!(w, "<a href='https://github.com/NoxNode/AmpItUp'>AmpItUp</a></br>")?;
    writeln!(w, "<a href='https://handmade.network/p/691/ampitup/blog/p/9034-iadd3_binary_encoding_breakdown'>IADD32</a></br>")?;
    */

    let mut instructions = list.clone();
    for iter in instructions.iter_mut() {
        let mut i = 0;
        while i < iter.fields.len() {
            if iter.fields[i].bits.len() == 0 {
                writeln!(w, "<b>removed empty field #{i} {} of {}</b>", iter.fields[i].name, iter.name)?;
                iter.fields.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
    for e in instructions.iter_mut() {
        e.fields.sort_by(|a, b| a.bits[0].0.cmp(&b.bits[0].0));
    }
    instructions.sort_by(|a, b| a.name.cmp(&b.name));
    let mut count = 0;

    let mut seen_insn = std::collections::BTreeSet::<String>::new();
    for inst in instructions {
        write!(w, "<div>")?;
        if seen_insn.iter().find(|&e| *e == inst.name).is_none() {
            write!(w, "<hr><h2>{}</h2>", inst.name)?;
        }
        seen_insn.insert(inst.name);

        write!(w, "<table style='width:100%;'>")?;
        let elems_per_col = 4;
        let elems_per_row = 128 / elems_per_col;
        for i in 0..elems_per_col {
            write!(w, "<tr>")?;
            for j in 0..elems_per_row {
                write!(w, "<th style='background-color:#d0d0d0;font-size:8px;'>{}</th>", i * elems_per_row + j)?;
            }
            write!(w, "</tr>")?;
            write!(w, "<tr>")?;
            let mut last_stride = 0;
            let mut last_name = String::new();
            let mut last_value = String::new();
            for j in 0..elems_per_row {
                let bitpos = i * elems_per_row + j;
                let color = get_html_field_color(&last_name);
                if let Some(f) = inst.fields.iter()
                    .find(|&e| e.bits.iter()
                        .find(|(start, end)| bitpos >= *start && bitpos <= *end).is_some()) {
                    if last_name != f.name && j != 0 {
                        write!(w, "<td style=\"background-color:#{color:x};\" colspan=\"{last_stride}\">{last_value}</td>")?;
                        last_stride = 0;
                    }
                    last_stride += 1;
                    last_name = f.name.clone();
                    last_value = if f.bits.len() > 1 {
                        let range = f.bits.iter().find(|(start, end)| bitpos >= *start && bitpos <= *end).unwrap();
                        format!("{} ({}-{})", f.name, range.0, range.1)
                    } else {
                        f.name.clone()
                    };
                } else if let Some(f) = inst.fixed_fields.iter()
                    .find(|&e| e.bits.iter()
                        .find(|(start, end)| bitpos >= *start && bitpos <= *end).is_some()) {
                    if last_name != f.name && j != 0 {
                        write!(w, "<td style=\"background-color:#{color:x};\" colspan=\"{last_stride}\">{last_value}</td>")?;
                        last_stride = 0;
                    }
                    last_stride += 1;
                    last_name = f.name.clone();
                    last_value = format!("{:b}", f.value.unwrap() & get_html_masked_value(bitpos, f));
                } else {
                    if last_name != "" && j != 0 {
                        write!(w, "<td style=\"background-color:#{color:x};\" colspan=\"{last_stride}\">{last_value}</td>")?;
                        last_stride = 0;
                    }
                    last_stride += 1;
                    last_name = "".to_string();
                    last_value = "".to_string();
                }
            }
            let color = get_html_field_color(&last_name);
            write!(w, "<td style=\"background-color:#{color:x};\" colspan=\"{last_stride}\">{last_value}</td>")?;
            write!(w, "</tr>")?;
        }
        write!(w, "</table>")?;
        write!(w, "</div>")?;
        count += 1;
        if count >= 16 {
            //break;
        }
    }
    writeln!(w, "</body></html>")?;
    Ok(())
}

pub fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let list = collect_sass_instructions(&std::fs::read_to_string("sm_86_instructions.txt")?)?;
    let mut w = String::new();
    write_rust(&mut w, &list)?;

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("sm86_decoder_generated.rs");
    std::fs::write(&dest_path, w).unwrap();
    
    //println!("{w}");
    Ok(())
}
