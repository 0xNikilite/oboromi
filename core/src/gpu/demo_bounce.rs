//! The bounce demo: a dozen balls under gravity in a box, stepped on the host once per audio period so every wall and
//! ball impact lands in the period it happens. Each impact restarts a free audren voice on a short knock, pitched by
//! the ball's size, as loud as the impact and panned by x. The NVIDIA driver's SM86 build of demos/shaders/particles.vert
//! and particles.frag draws the walls and the balls as one instanced draw of round quads.
use super::engine::Gpu;
use super::frame::cache_root;
use super::{VkState, device};
use crate::audio::demo_music::{Sound, pcm};
use crate::audio::play;
use crate::tests::gpu::engine::{
    BIND_GROUP_CONSTANT_BUFFER, CLEAR_SURFACE, DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, SET_COLOR_CLEAR_VALUE, SET_COLOR_TARGET_A,
    SET_CONSTANT_BUFFER_SELECTOR_A, SET_CT_WRITE, SET_OBJECT, SET_PIPELINE_SHADER, SET_REPORT_SEMAPHORE_A, SET_SCISSOR_ENABLE,
    SET_VERTEX_ATTRIBUTE_A, SET_VERTEX_STREAM_A_FORMAT, SET_VERTEX_STREAM_SIZE_A, SET_VIEWPORT_CLIP_HORIZONTAL, SET_VIEWPORT_SCALE_X,
    bytes, entry, inc, non_inc,
};
use crate::tests::gpu_test::inst::bra;
use std::f32::consts::TAU;
use std::sync::Mutex;

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
const BALLS: usize = 12;
/// The box's inner half extents and the screen's, in world units, y up.
const HALF: [f32; 2] = [1.6, 0.85];
const VIEW: [f32; 2] = [1.75, 0.984_375];
const GRAVITY: f32 = 3.0;
const STEP: f32 = 1.0 / 200.0;
const RESTITUTION: f32 = 0.8;
const SLOP: f32 = 0.001;
const CORRECTION: f32 = 0.8;
const SOLVER_ITERATIONS: usize = 16;
/// Impacts slower than this are a ball resting on another, not a knock.
const SOFT: f32 = 0.4;
const BEAD: f32 = 0.02;
/// The largest quad half size, which particles.vert reads from camera_right.w; smaller ones ride in position.w.
const LARGEST: f32 = 0.16;

const BASE: u64 = 0x1000_0000;
const PUSHBUFFER: u64 = BASE;
const DRAW_SEGMENT: u64 = BASE + 0x2000;
const VERTEX_PROGRAM: u64 = BASE + 0x1_0000;
const PIXEL_PROGRAM: u64 = BASE + 0x1_4000;
const VS_POINTERS: u64 = BASE + 0x2_0000;
const VS_UNIFORMS: u64 = BASE + 0x2_1000;
const INSTANCES: u64 = BASE + 0x10_0000;
pub const TARGET: u64 = BASE + 0x20_0000;
const RELEASE: u64 = BASE + 0xa0_0000;
const MAPPED: u64 = 0xa1_0000;
// particles.vert loads its uniform block handle with `ULDC.64 UR4, c[0x1d][0x0]`.
const POINTER_BANK: u32 = 0x1d;
const WAIT_FOR_IDLE: u32 = 0x0110;
const DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT: u32 = 0x1218;
const SET_BLEND_SEPARATE_FOR_ALPHA: u32 = 0x133c;
const SET_BLEND: u32 = 0x1360;
const SET_VERTEX_STREAM_INSTANCE_A: u32 = 0x1880;
const STRIP_OF_FOUR: u32 = 5 << 28 | 4 << 16;

fn named(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

struct Ball {
    at: [f32; 2],
    velocity: [f32; 2],
    radius: f32,
}

pub struct Bounce {
    pub sound: Sound,
    balls: Vec<Ball>,
}

impl Bounce {
    pub fn new() -> Self {
        let mut seed = 7u32;
        let mut random = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            seed as f32 / 4_294_967_296.0
        };
        let balls = (0..BALLS)
            .map(|i| Ball {
                at: [-1.35 + 0.245 * i as f32, 0.6 - 0.3 * (i % 3) as f32],
                velocity: [4.0 * random() - 2.0, 2.0 * random()],
                radius: 0.05 + 0.06 * random(),
            })
            .collect();
        let time = (0..6000).map(|n| n as f32 / 48_000.0);
        let knock = pcm(time.map(|t| 0.7 * (TAU * 520.0 * t).sin() * (-45.0 * t).exp() + 0.3 * (TAU * 1390.0 * t).sin() * (-70.0 * t).exp()));
        Self { sound: Sound::new(&[knock], 16), balls }
    }

    fn physics(&mut self) -> Vec<(usize, f32)> {
        let mut impacts = Vec::new();
        let fastest = self.balls.iter().map(|ball| ball.velocity[0].hypot(ball.velocity[1])).fold(0.0, f32::max);
        let smallest = self.balls.iter().map(|ball| ball.radius).fold(f32::INFINITY, f32::min);
        let substeps = (fastest * STEP / (0.5 * smallest)).ceil().max(1.0) as usize;
        let dt = STEP / substeps as f32;
        for _ in 0..substeps {
            for ball in &mut self.balls {
                ball.velocity[1] -= GRAVITY * dt;
                for axis in 0..2 {
                    ball.at[axis] += ball.velocity[axis] * dt;
                }
            }
            for _ in 0..SOLVER_ITERATIONS {
                for i in 0..BALLS {
                    for j in i + 1..BALLS {
                        let (a, b) = (&self.balls[i], &self.balls[j]);
                        let gap = [b.at[0] - a.at[0], b.at[1] - a.at[1]];
                        let distance = gap[0].hypot(gap[1]);
                        let overlap = a.radius + b.radius - distance;
                        if overlap <= 0.0 {
                            continue;
                        }
                        let normal = gap.map(|c| c / distance);
                        let closing = (b.velocity[0] - a.velocity[0]) * normal[0] + (b.velocity[1] - a.velocity[1]) * normal[1];
                        let [inv_a, inv_b] = [1.0 / (a.radius * a.radius), 1.0 / (b.radius * b.radius)];
                        let correction = CORRECTION * (overlap - SLOP).max(0.0) / (inv_a + inv_b);
                        for axis in 0..2 {
                            self.balls[i].at[axis] -= correction * inv_a * normal[axis];
                            self.balls[j].at[axis] += correction * inv_b * normal[axis];
                        }
                        if closing >= 0.0 {
                            continue;
                        }
                        let restitution = if -closing < SOFT { 0.0 } else { RESTITUTION };
                        let impulse = -(1.0 + restitution) * closing / (inv_a + inv_b);
                        for axis in 0..2 {
                            self.balls[i].velocity[axis] -= impulse * inv_a * normal[axis];
                            self.balls[j].velocity[axis] += impulse * inv_b * normal[axis];
                        }
                        impacts.push((i, -closing));
                    }
                }
                for (i, ball) in self.balls.iter_mut().enumerate() {
                    for axis in 0..2 {
                        let limit = HALF[axis] - ball.radius;
                        if ball.at[axis].abs() > limit {
                            ball.at[axis] = limit.copysign(ball.at[axis]);
                            if ball.at[axis] * ball.velocity[axis] > 0.0 {
                                let speed = ball.velocity[axis].abs();
                                impacts.push((i, speed));
                                ball.velocity[axis] = if speed < SOFT { 0.0 } else { -RESTITUTION * ball.velocity[axis] };
                            }
                        }
                    }
                }
            }
        }
        impacts
    }

    /// Advances the balls one period and knocks a voice for every impact in it.
    pub fn step(&mut self) {
        let impacts = self.physics();
        for (i, speed) in impacts.into_iter().filter(|&(_, speed)| speed > SOFT) {
            let ball = &self.balls[i];
            let voice = self.sound.free_voice();
            self.sound.trigger(voice, 0, 0.08 / ball.radius, (speed / 8.0).min(0.6), ball.at[0] / HALF[0]);
        }
    }

    /// particles.vert instances: position (x, y, 0, size as 2 - 2 size / LARGEST), unused, colour (rgb, brightness).
    fn instances(&self, walls: &[[f32; 2]]) -> Vec<f32> {
        let quad = |[x, y]: [f32; 2], size: f32, colour: [f32; 4]| [&[x, y, 0.0, 2.0 - 2.0 * size / LARGEST, 0.0, 0.0, 0.0, 0.0][..], &colour].concat();
        let wall = walls.iter().flat_map(|&at| quad(at, BEAD, [0.07, 0.08, 0.1, 10.0]));
        // A brightness of 10 saturates the disc out to 0.83 of the quad, so its centre clamps bright and its rim keeps the colour.
        let balls = self.balls.iter().enumerate().flat_map(|(i, ball)| {
            let hue = i as f32 * TAU / BALLS as f32;
            let colour = [0.0, 2.1, 4.2].map(|phase| 0.09 + 0.07 * (hue + phase).cos());
            let shine = [ball.at[0] - 0.3 * ball.radius, ball.at[1] + 0.3 * ball.radius];
            [quad(ball.at, ball.radius / 0.83, [colour[0], colour[1], colour[2], 10.0]), quad(shine, ball.radius * 0.5, [0.9, 0.9, 0.9, 0.8])].concat()
        });
        wall.chain(balls).collect()
    }
}

/// Bead centres along the four walls, just outside the box.
fn walls() -> Vec<[f32; 2]> {
    let [w, h] = HALF.map(|half| half + 0.75 * BEAD);
    let beads = |length: f32| (length / (0.35 * BEAD)) as usize;
    let (nx, ny) = (beads(2.0 * w), beads(2.0 * h));
    let along = |n: usize, k: usize, half: f32| -half + 2.0 * half * k as f32 / n as f32;
    let horizontal = (0..=nx).flat_map(|k| [[along(nx, k, w), -h], [along(nx, k, w), h]]);
    let vertical = (1..ny).flat_map(|k| [[-w, along(ny, k, h)], [w, along(ny, k, h)]]);
    horizontal.chain(vertical).collect()
}

struct Scene<'a> {
    gpu: Gpu<'a>,
    bounce: Bounce,
    walls: Vec<[f32; 2]>,
    frame: u32,
}

impl<'a> Scene<'a> {
    fn new(state: &'static VkState) -> Result<Self, String> {
        let mut gpu = Gpu::new(state, MAPPED as usize, Some((&cache_root(), "demos")), *super::SPEED_HACKS.lock().unwrap()).map_err(named)?;
        gpu.space.map(BASE, 0, MAPPED);
        let walls = walls();
        let count = (walls.len() + 2 * BALLS) as u32;
        const A8B8G8R8: u32 = 0xd5;
        const PITCH: u32 = 0x1000;
        const R32_G32_B32_A32_FLOAT: u32 = 0x01 << 21 | 7 << 27;
        const OGL_FUNC_ADD: u32 = 0x8006;
        const OGL_ONE: u32 = 0x4001;
        const OGL_ONE_MINUS_SRC_ALPHA: u32 = 0x4303;
        let (x, y) = (WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);
        let mut words = Vec::new();
        inc(SET_OBJECT, &[0xc797], &mut words);
        inc(SET_COLOR_TARGET_A, &[0, TARGET as u32, WIDTH * 4, HEIGHT, A8B8G8R8, PITCH], &mut words);
        inc(SET_VIEWPORT_SCALE_X, &[x, y, 1.0, x, y, 0.0].map(f32::to_bits), &mut words);
        inc(SET_VIEWPORT_CLIP_HORIZONTAL, &[WIDTH << 16, HEIGHT << 16, 0, 1.0f32.to_bits()], &mut words);
        inc(SET_SCISSOR_ENABLE, &[1, WIDTH << 16, HEIGHT << 16], &mut words);
        inc(SET_COLOR_CLEAR_VALUE, &[0.04f32, 0.05, 0.07, 1.0].map(f32::to_bits), &mut words);
        inc(SET_CT_WRITE, &[0x1111], &mut words);
        inc(SET_BLEND_SEPARATE_FOR_ALPHA, &[0, OGL_FUNC_ADD, OGL_ONE, OGL_ONE_MINUS_SRC_ALPHA], &mut words);
        inc(SET_BLEND, &[1], &mut words);
        inc(SET_VERTEX_STREAM_SIZE_A, &[0, count * 48], &mut words);
        inc(SET_VERTEX_STREAM_A_FORMAT, &[0x1000 | 48, 0, INSTANCES as u32, 1], &mut words);
        inc(SET_VERTEX_STREAM_INSTANCE_A, &[1], &mut words);
        inc(SET_VERTEX_ATTRIBUTE_A, &[R32_G32_B32_A32_FLOAT, 32 << 7 | R32_G32_B32_A32_FLOAT], &mut words);
        // Register counts are the highest register each SASS program names plus one: R9 and R11.
        for (pipeline, enable, registers, group, program) in [(1, 0x11, 10, 0, VERTEX_PROGRAM), (5, 0x51, 12, 4, PIXEL_PROGRAM)] {
            inc(SET_PIPELINE_SHADER + pipeline * 64, &[enable], &mut words);
            inc(SET_PIPELINE_SHADER + pipeline * 64 + 0xc, &[registers, group, 0, program as u32], &mut words);
        }
        inc(SET_CONSTANT_BUFFER_SELECTOR_A, &[0x100, 0, VS_POINTERS as u32], &mut words);
        inc(BIND_GROUP_CONSTANT_BUFFER, &[POINTER_BANK << 4 | 1], &mut words);
        let program = |code: &[u8]| [code, &bra(-16).to_le_bytes()].concat();
        // Column-major view_proj mapping the world, y up, onto the screen, then camera right (w = LARGEST) and up.
        let [sx, sy] = VIEW.map(|half| 1.0 / half);
        let uniforms = [sx, 0.0, 0.0, 0.0, 0.0, -sy, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 0.0, 0.0, LARGEST, 0.0, 1.0, 0.0, 0.0];
        // A bindless cbuf handle: the address >> 4 in bits 0..45 and the size >> 4 above (NVK nvk_bindless_cbuf).
        let handle = [(VS_UNIFORMS >> 4) as u32, (VS_UNIFORMS >> 36) as u32 | 0x100 >> 4 << 13];
        for (va, data) in [
            (VERTEX_PROGRAM, program(include_bytes!("../../tests/fixtures/driver-shaders/particles.vert.bin"))),
            (PIXEL_PROGRAM, program(include_bytes!("../../tests/fixtures/driver-shaders/particles.frag.bin"))),
            (VS_POINTERS, bytes(&handle)),
            (VS_UNIFORMS, floats(&uniforms)),
            (PUSHBUFFER, bytes(&words)),
        ] {
            gpu.write(va, &data).map_err(named)?;
        }
        gpu.submit(&[entry(PUSHBUFFER, words.len())])?;
        Ok(Self { gpu, bounce: Bounce::new(), walls, frame: 0 })
    }

    /// Clears the box, draws the walls and the balls where they are and waits for the frame's report.
    fn draw(&mut self) -> Result<(), String> {
        let instances = self.bounce.instances(&self.walls);
        self.frame += 1;
        let mut words = Vec::new();
        inc(WAIT_FOR_IDLE, &[0], &mut words);
        inc(CLEAR_SURFACE, &[0x3c], &mut words);
        inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_FIRST, &[STRIP_OF_FOUR], &mut words);
        non_inc(DRAW_VERTEX_ARRAY_BEGIN_END_INSTANCE_SUBSEQUENT, &vec![STRIP_OF_FOUR; instances.len() / 12 - 1], &mut words);
        inc(SET_REPORT_SEMAPHORE_A, &[0, RELEASE as u32, self.frame, 0x1000_f010], &mut words);
        self.gpu.write(INSTANCES, &floats(&instances)).map_err(named)?;
        self.gpu.write(DRAW_SEGMENT, &bytes(&words)).map_err(named)?;
        self.gpu.submit(&[entry(DRAW_SEGMENT, words.len())])?;
        let mut released = [0; 4];
        self.gpu.read(RELEASE, &mut released).map_err(named)?;
        match u32::from_le_bytes(released) == self.frame {
            true => Ok(()),
            false => Err(format!("frame {} did not release its report", self.frame)),
        }
    }
}

pub fn render_bounce_frame(seconds: f32) -> Result<(), String> {
    static SCENE: Mutex<Option<Scene<'static>>> = Mutex::new(None);
    let mut slot = SCENE.lock().unwrap();
    let scene = match &mut *slot {
        Some(scene) => scene,
        empty => empty.insert(Scene::new(device().map_err(named)?)?),
    };
    for _ in 0..scene.bounce.sound.due(seconds) {
        scene.bounce.step();
        play(&[scene.bounce.sound.period()]);
    }
    scene.draw()?;
    scene.gpu.present(TARGET, WIDTH, HEIGHT).map_err(named)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounce_does_not_gain_energy_or_sink_into_contacts() {
        let mut bounce = Bounce::new();
        let energy = |bounce: &Bounce| bounce.balls.iter().map(|ball| {
            ball.radius.powi(2) * (0.5 * ball.velocity.iter().map(|v| v * v).sum::<f32>() + GRAVITY * (ball.at[1] + HALF[1]))
        }).sum::<f32>();
        let mut previous = energy(&bounce);
        let mut periods = 0;
        let mut knocks = 0;
        let mut most_knocks = 0;
        for frame in 0..600 {
            let target = (frame + 1) * 200 / 60;
            while periods < target {
                let period_knocks = bounce.physics().into_iter().filter(|&(_, speed)| speed > SOFT).count();
                knocks += period_knocks;
                most_knocks = most_knocks.max(period_knocks);
                periods += 1;
            }
            let current = energy(&bounce);
            assert!(current <= previous + 1e-5, "frame {frame} gained {} energy", current - previous);
            previous = current;
            let floor_overlap = bounce.balls.iter().map(|ball| ball.radius - ball.at[1] - HALF[1]).fold(0.0, f32::max);
            let ball_overlap = (0..BALLS).flat_map(|i| (i + 1..BALLS).map(move |j| (i, j))).map(|(i, j)| {
                let [x, y] = [0, 1].map(|axis| bounce.balls[j].at[axis] - bounce.balls[i].at[axis]);
                bounce.balls[i].radius + bounce.balls[j].radius - x.hypot(y)
            }).fold(0.0, f32::max);
            assert!(floor_overlap.max(ball_overlap) <= SLOP + 1e-5, "frame {frame} overlap {}", floor_overlap.max(ball_overlap));
        }
        assert!(most_knocks <= 4, "{most_knocks} collision sounds in one period");
        eprintln!("collision sounds: {knocks}, at most {most_knocks} per period");
    }
}
