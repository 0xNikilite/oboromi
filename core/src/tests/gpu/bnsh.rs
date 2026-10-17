#[cfg(test)]
mod tests {
    use crate::gpu::bnsh::{Program, programs, translate};
    use crate::gpu::pipeline_cache::Disk;
    use crate::gpu::sm86::{ResourceKind, Stage};
    use crate::tests::gpu_test::inst::{bra, exit, mov_imm, stg};
    use ash::vk;

    #[test]
    fn bnsh_programs_come_back_from_a_built_file() {
        let fixtures = [&include_bytes!("../../../tests/fixtures/driver-shaders/triangle.vert.bin")[..], include_bytes!("../../../tests/fixtures/driver-shaders/triangle.frag.bin")];
        let expected: Vec<(Vec<u8>, Vec<u8>)> = fixtures.iter().map(|f| (f[..0x80].to_vec(), [&f[0x80..], &bra(-16).to_le_bytes()].concat())).collect();
        let mut file = vec![0; 0x100];
        file[..4].copy_from_slice(b"BNSH");
        file[0x16] = 0x20;
        file[0x20..0x24].copy_from_slice(b"grsc");
        file[0x48] = 0x60;
        file[0x69] = 0x01;
        for (sph, code) in &expected {
            file.extend([0x78, 0x56, 0x34, 0x12]);
            file.extend([0; 0x2c]);
            file.extend([&sph[..], code].concat());
        }
        let pool_size = file.len() as u32 - 0x100;
        file[0x64..0x68].copy_from_slice(&pool_size.to_le_bytes());
        let found: Vec<_> = programs(&file).into_iter().map(|p| (p.sph.iter().flat_map(|w| w.to_le_bytes()).collect(), p.code)).collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn precompiled_program_metadata_survives_the_disk_cache() {
        let instructions = [mov_imm(4, 0), mov_imm(5, 1), stg(4, 0, 5), exit(), bra(-16)];
        let program = Program {
            stage: Stage::Fragment { flat: 0, screen_linear: 0, depth: false },
            sph: [0; 32],
            code: instructions.into_iter().flat_map(u128::to_le_bytes).collect(),
        };
        let (words, resources, stores_global) = translate(None, &program).unwrap();
        assert!(resources.contains(&ResourceKind::Global) && stores_global);

        let root = std::env::temp_dir().join(format!("oboromi-bnsh-cache-{}", std::process::id()));
        let disk = Disk::open(&root, "test", &vk::PhysicalDeviceProperties::default()).unwrap();
        disk.store_spirv(7, &words, &resources, stores_global).unwrap();
        assert_eq!(disk.spirv(7), Some((words, resources, true)));
        std::fs::remove_dir_all(root).unwrap();
    }
}
