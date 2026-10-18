#![cfg(test)]

use crate::gpu::demo_particles::{COUNT, HEIGHT, PARTICLES, Particles, RELEASE, TARGET, WIDTH, update_uniforms};
use crate::tests::gpu::harness::shared_device;
use std::io::Write;
use std::process::{Command, Stdio};

fn particles(scene: &mut Particles) -> Vec<[f32; 12]> {
    let mut bytes = vec![0; COUNT as usize * 48];
    scene.gpu.read(PARTICLES, &mut bytes).unwrap();
    bytes.chunks(48).map(|p| std::array::from_fn(|i| f32::from_le_bytes(p[4 * i..][..4].try_into().unwrap()))).collect()
}

#[test]
fn update_moves_every_particle_one_integrator_step() {
    let Some(state) = shared_device() else { return };
    let mut scene = Particles::new(state).unwrap();
    let before = particles(&mut scene);
    scene.update(0.0).unwrap();
    let mut release = [0; 4];
    scene.gpu.read(RELEASE, &mut release).unwrap();
    assert_eq!(u32::from_le_bytes(release), 1);
    let [ax, ay, az, a, bx, by, bz, b, dt, gravity, damping, _] = update_uniforms(0.0);
    let mut respawned = 0;
    for (i, (p, q)) in before.iter().zip(particles(&mut scene)).enumerate() {
        let pull = |x: f32, y: f32, z: f32, strength: f32| {
            let d = [x - p[0], y - p[1], z - p[2]];
            d.map(|c| c * (strength / (d.iter().map(|c| c * c).sum::<f32>() + 0.05)))
        };
        let [pa, pb] = [pull(ax, ay, az, a), pull(bx, by, bz, b)];
        let velocity = [0, 1, 2].map(|c| (p[4 + c] + (pa[c] + pb[c] - if c == 1 { gravity } else { 0.0 }) * dt) * damping);
        let age = p[3] + p[7] * dt;
        if age >= 1.0 {
            let offset = [q[0] - ax, q[1] - ay, q[2] - az];
            assert!(q[3] == 0.0 && offset.iter().all(|c| c.abs() <= 0.1), "{i}: respawned at {q:?}");
            respawned += 1;
            continue;
        }
        let heat = (velocity.iter().map(|c| c * c).sum::<f32>().sqrt() * 0.25).clamp(0.0, 1.0);
        let [cold, hot] = [[0.2, 0.45, 1.0], [1.0, 0.55, 0.15]];
        let colour = [0, 1, 2].map(|c| cold[c] + (hot[c] - cold[c]) * heat);
        let position = [0, 1, 2].map(|c| p[c] + velocity[c] * dt);
        let want = [&position[..], &[age], &velocity, &[p[7]], &colour, &[1.0 - age]].concat();
        assert!(want.iter().zip(q).all(|(w, g)| (w - g).abs() <= 1e-5), "{i}: {q:?} != {want:?}");
    }
    assert!(respawned > 0);
}

#[test]
fn particles_frame_glows_where_the_particles_cluster() {
    let Some(state) = shared_device() else { return };
    let mut scene = Particles::new(state).unwrap();
    scene.update(0.0).unwrap();
    scene.draw(0.0).unwrap();
    let mut frame = vec![0; (WIDTH * HEIGHT * 4) as usize];
    scene.gpu.read(TARGET, &mut frame).unwrap();
    let pixel = |x: usize, y: usize| &frame[(y * WIDTH as usize + x) * 4..][..4];
    assert_eq!([pixel(0, 0), pixel(0, HEIGHT as usize - 1)], [[14, 16, 34, 255], [2, 2, 6, 255]]);
    assert!(frame.chunks_exact(4).any(|pixel| pixel[..3] == [255; 3]));
    // Devices round the blended glow differently by a step or two, so only lavapipe, which CI runs, has a fixed frame.
    if state.is_llvmpipe() {
        let mut sha256sum = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        sha256sum.stdin.take().unwrap().write_all(&frame).unwrap();
        let digest = String::from_utf8(sha256sum.wait_with_output().unwrap().stdout).unwrap();
        assert_eq!(&digest[..64], "dda92ecb9385ea140c5302181d7e7962a61b7cfa570608e1993f39cbc573ad81");
    }
}
