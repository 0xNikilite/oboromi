#[cfg(test)]
mod tests {
    use crate::gpu::engine::{Geometry, Gpu};
    use crate::gpu::memory::PAGE;
    use crate::tests::gpu::engine::{SET_OBJECT, bytes, entry, inc};
    use crate::tests::gpu::harness::vk_state;
    use crate::tests::gpu_test::inst::bra;

    const GUEST: u64 = 0x2_0000_0000;
    const QMD: u64 = GUEST + 0x100;
    const CBUF0: u64 = GUEST + 0x1000;
    const CBUF1: u64 = GUEST + 0x2000;
    const VERTICES: u64 = GUEST + 0x3000;
    const INDICES: u64 = GUEST + 0x4000;
    const TLAS_ONE: u64 = GUEST + 0x5000;
    const TLAS_THREE: u64 = GUEST + 0x6000;
    const PROGRAM: u64 = GUEST + 0x8000;
    const RECORDS: u64 = GUEST + PAGE;
    const MISS: [u32; 4] = [0xffff_0000, 0, u32::MAX, 0];

    fn triangles(first_z: f32) -> Vec<u8> {
        let corners = [-1.0, -1.0, 1.0, -1.0, 0.0, 1.0];
        let positions = [first_z, 2.0, 3.0].map(|z| corners.chunks(2).flat_map(|xy| [xy[0], xy[1], z]).collect::<Vec<f32>>());
        positions.concat().iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    // Each lane traces +z from x, y in [-2, 2] against triangles (-1, -1) (1, -1) (0, 1) stacked from z = first_z, as
    // the rig's query-candidates variants do: every candidate is confirmed when tris is 1, only primitive 1 otherwise.
    fn expected(x: u32, y: u32, tris: u32, first_z: f32) -> [u32; 4] {
        let [x, y] = [x, y].map(|c| (c as f32 + 0.5) / 16.0 - 2.0);
        if y <= -1.0 || y >= 1.0 - 2.0 * x.abs() {
            return MISS;
        }
        let v = (y + 1.0) / 2.0;
        let unorm = |c: f32| (c * 255.0).round() as u32;
        let colour = 0xff00_00ff | unorm((x + 1.0 - v) / 2.0) << 8 | unorm(v) << 16;
        match tris {
            1 => [colour, first_z.to_bits(), 0, 1],
            _ => [colour, 2.0f32.to_bits(), 1, 2 | 0x10 << 8],
        }
    }

    // The NVIDIA 580 compile of the rig's query-candidates.comp reads the records buffer from c[0x0][0x30], its size
    // from c[0x0][0x38], the TLAS from c[0x0][0x40], tris from the push constants at c[0x0][0x638] and a 0x20 from c[0x1][0x0].
    fn launch(gpu: &mut Gpu, tlas: u64, tris: u32) -> Vec<[u32; 4]> {
        const SEND_PCAS_A: u32 = 0x02b4;
        const SEND_SIGNALING_PCAS2_B: u32 = 0x02c0;
        let mut cbuf0 = vec![0; 0x640];
        for (offset, value) in [(0x30, RECORDS), (0x38, 0x10000), (0x40, tlas), (0x638, tris.into())] {
            cbuf0[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        let mut qmd = [0; 64];
        [qmd[12], qmd[13], qmd[14]] = [8, 8, 1];
        [qmd[18], qmd[19], qmd[20]] = [8 << 16 | 3 << 4, 1 << 16 | 8, 0b11];
        for (i, (address, size)) in [(CBUF0, 0x640), (CBUF1, 0x10)].into_iter().enumerate() {
            [qmd[32 + 2 * i], qmd[33 + 2 * i]] = [address as u32, (address >> 32) as u32 | size >> 4 << 19];
        }
        [qmd[48], qmd[49]] = [PROGRAM as u32, (PROGRAM >> 32) as u32];
        let mut words = Vec::new();
        inc(SET_OBJECT, &[0xc7c0], &mut words);
        inc(SEND_PCAS_A, &[(QMD >> 8) as u32], &mut words);
        inc(SEND_SIGNALING_PCAS2_B, &[3], &mut words);
        gpu.write(CBUF0, &cbuf0).unwrap();
        gpu.write(QMD, &bytes(&qmd)).unwrap();
        gpu.write(GUEST, &bytes(&words)).unwrap();
        gpu.submit(&[entry(GUEST, words.len())]).unwrap();
        let mut records = vec![0; 0x10000];
        gpu.read(RECORDS, &mut records).unwrap();
        records.chunks(16).map(|r| [0, 4, 8, 12].map(|at| u32::from_le_bytes(r[at..at + 4].try_into().unwrap()))).collect()
    }

    #[test]
    fn driver_ray_query_traces_the_registered_structure_and_follows_its_vertices() {
        let Some(state) = vk_state() else { return };
        let mut gpu = Gpu::new(&state, (2 * PAGE) as usize);
        gpu.space.map(GUEST, 0, 2 * PAGE);
        let code = include_bytes!("../../../tests/fixtures/driver-shaders/query-candidates.comp.bin");
        gpu.write(PROGRAM, &[&code[..], &bra(-16).to_le_bytes()].concat()).unwrap();
        gpu.write(CBUF1, &0x20u32.to_le_bytes()).unwrap();
        gpu.write(VERTICES, &triangles(1.0)).unwrap();
        gpu.write(INDICES, &bytes(&(0..9).collect::<Vec<_>>())).unwrap();
        let identity = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
        for (tlas, tris) in [(TLAS_ONE, 1), (TLAS_THREE, 3)] {
            let geometry = Geometry { vertices: VERTICES, stride: 12, vertex_count: 3 * tris, indices: INDICES, index_count: 3 * tris, transform: identity, opaque: false };
            gpu.register_acceleration_structure(tlas, vec![geometry]).unwrap();
        }
        for (tlas, tris, first_z) in [(TLAS_ONE, 1, 1.0), (TLAS_THREE, 3, 1.0), (TLAS_ONE, 1, 1.5)] {
            gpu.write(VERTICES, &triangles(first_z)).unwrap();
            let records = launch(&mut gpu, tlas, tris);
            for (i, record) in (0..).zip(records) {
                assert_eq!(record, expected(i % 64, i / 64, tris, first_z), "tris {tris}, first z {first_z}, ray ({}, {})", i % 64, i / 64);
            }
        }
    }
}
