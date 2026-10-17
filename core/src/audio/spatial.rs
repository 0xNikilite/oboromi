//! Block binaural rendering and shoebox-room acoustics for host-driven game audio.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const BLOCK: usize = 240;
const RATE: f32 = 48_000.0;
const SPEED: f32 = 343.0;
const HEAD_RADIUS: f32 = 0.0875;
const HISTORY: usize = 1 << 17;

pub type V3 = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Headphones,
    Speakers,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub absorption: f32,
    pub scattering: f32,
}

impl Material {
    pub const WET_STONE: Self = Self {
        absorption: 0.08,
        scattering: 0.18,
    };
    pub const DRY_STONE: Self = Self {
        absorption: 0.18,
        scattering: 0.32,
    };
    pub const WATER: Self = Self {
        absorption: 0.04,
        scattering: 0.08,
    };
    pub const OPEN: Self = Self {
        absorption: 0.95,
        scattering: 0.0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct Doorway {
    pub min: V3,
    pub max: V3,
    pub material: Material,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub min: V3,
    pub max: V3,
    /// Negative then positive X, Y and Z walls.
    pub walls: [Material; 6],
    pub doorways: Vec<Doorway>,
}

#[derive(Clone, Copy, Debug)]
pub struct Source {
    pub id: usize,
    pub position: V3,
    pub gain: f32,
}

pub struct Mix {
    block: [[f32; 2]; BLOCK],
}

impl Mix {
    pub fn new() -> Self {
        Self {
            block: [[0.0; 2]; BLOCK],
        }
    }

    pub fn add(&mut self, source: &[[f32; 2]]) {
        for (out, input) in self.block.iter_mut().zip(source) {
            out[0] += input[0];
            out[1] += input[1];
        }
    }

    pub fn finish(mut self) -> [[f32; 2]; BLOCK] {
        for frame in &mut self.block {
            for sample in frame {
                *sample *= 0.5 / (1.0 + sample.abs());
            }
        }
        self.block
    }
}

impl Default for Mix {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Default)]
struct Ear {
    shadow: f32,
    air: f32,
    material: f32,
    bass: f32,
    rear: f32,
    notch_lo: f32,
    notch_hi: f32,
}

#[derive(Clone, Copy, Default)]
struct PathState {
    ears: [Ear; 2],
    gain: f32,
    delay: [f32; 2],
    from_gain: f32,
    from_delay: [f32; 2],
    target_gain: f32,
    target_delay: [f32; 2],
    initialized: bool,
}

struct Fdn {
    lines: [Vec<f32>; 8],
    at: [usize; 8],
    damp: [f32; 8],
}

impl Fdn {
    fn new() -> Self {
        let lengths = [1423, 1559, 1699, 1871, 1999, 2131, 2281, 2417];
        Self {
            lines: lengths.map(|n| vec![0.0; n]),
            at: [0; 8],
            damp: [0.0; 8],
        }
    }

    fn process(&mut self, input: f32, decay: f32, damping: f32) -> [f32; 2] {
        let taps = std::array::from_fn::<_, 8, _>(|i| self.lines[i][self.at[i]]);
        let sum = taps.iter().sum::<f32>() * 0.25;
        for i in 0..8 {
            let mixed = sum - taps[i] + input * 0.22;
            self.damp[i] += (mixed - self.damp[i]) * damping;
            self.lines[i][self.at[i]] = self.damp[i] * decay;
            self.at[i] = (self.at[i] + 1) % self.lines[i].len();
        }
        [
            (taps[0] + taps[2] - taps[4] - taps[6]) * 0.18,
            (taps[1] + taps[3] - taps[5] - taps[7]) * 0.18,
        ]
    }
}

struct SourceState {
    history: Vec<f32>,
    written: u64,
    paths: [PathState; 43],
    reverbs: Vec<Fdn>,
    room_weights: Vec<f32>,
    rooms_initialized: bool,
}

impl SourceState {
    fn new(rooms: usize) -> Self {
        Self {
            history: vec![0.0; HISTORY],
            written: 0,
            paths: [PathState::default(); 43],
            reverbs: (0..rooms).map(|_| Fdn::new()).collect(),
            room_weights: vec![0.0; rooms],
            rooms_initialized: false,
        }
    }
}

#[derive(Clone, Copy)]
struct Path {
    position: V3,
    gain: f32,
    absorption: f32,
}

#[derive(Clone, Copy)]
struct Parameters {
    delay: [f32; 2],
    gain: f32,
    ear_gain: [f32; 2],
    shadow: [f32; 2],
    air: f32,
    material: f32,
    bass: f32,
    bass_filter: f32,
    rear: f32,
    rear_filter: f32,
    notch: [f32; 2],
}

pub struct Spatializer {
    rooms: Vec<Room>,
    mode: Mode,
    listener: V3,
    forward: V3,
    up: V3,
    states: HashMap<usize, SourceState>,
}

impl Spatializer {
    pub fn new(rooms: &[Room], mode: Mode) -> Self {
        Self {
            rooms: rooms.to_vec(),
            mode,
            listener: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            states: HashMap::new(),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    pub fn set_listener(&mut self, pos: V3, forward: V3, up: V3) {
        self.listener = pos;
        self.forward = unit(forward);
        self.up = unit(up);
    }

    pub fn render(&mut self, source: &Source, mono: &[f32], out: &mut [[f32; 2]]) {
        assert_eq!(mono.len(), out.len());
        assert!(mono.len() <= BLOCK);
        out.fill([0.0; 2]);
        let paths = self.paths(source.position);
        let parameters: Vec<_> = paths.iter().map(|path| parameters(path, source.gain, self.listener, self.forward, self.up, self.mode)).collect();
        let room_weights = self.room_weights(self.listener);
        let room_data: Vec<(f32, f32)> = self.rooms.iter().map(room_decay).collect();
        let rooms = self.rooms.len();
        let state = self
            .states
            .entry(source.id)
            .or_insert_with(|| SourceState::new(rooms));
        if !state.rooms_initialized {
            state.room_weights.copy_from_slice(&room_weights);
            state.rooms_initialized = true;
        } else {
            for (weight, &target) in state.room_weights.iter_mut().zip(&room_weights) {
                *weight += (target - *weight).clamp(-0.005, 0.005);
            }
        }
        for &sample in mono {
            state.history[(state.written as usize) & (HISTORY - 1)] = sample;
            state.written += 1;
        }
        let block_start = state.written - mono.len() as u64;
        for (n, frame) in out.iter_mut().enumerate() {
            state.written = block_start + n as u64;
            for (index, parameters) in parameters.iter().enumerate() {
                binaural(
                    parameters,
                    &mut state.paths[index],
                    &state.history,
                    state.written,
                    n,
                    index == 0,
                    frame,
                );
            }
            let dry = mono[n] * parameters[0].gain * 0.08;
            for (room, (&weight, &(decay, damping))) in
                state.room_weights.iter().zip(&room_data).enumerate()
            {
                let wet = state.reverbs[room].process(dry * weight, decay, damping);
                frame[0] += wet[0] * weight;
                frame[1] += wet[1] * weight;
            }
        }
        state.written = block_start + mono.len() as u64;
    }

    fn paths(&self, source: V3) -> Vec<Path> {
        let direct_distance = length(sub(source, self.listener));
        let (occlusion, absorption) = self.occlusion(source);
        let mut paths = vec![Path {
            position: source,
            gain: distance_gain(direct_distance) * occlusion,
            absorption,
        }];
        let Some(room) = self
            .rooms
            .iter()
            .find(|room| contains(room, source) && contains(room, self.listener))
        else {
            return paths;
        };
        let first: Vec<(V3, usize)> = (0..6)
            .map(|wall| (mirror(source, room, wall), wall))
            .collect();
        for &(image, wall) in &first {
            if reflection_visible(image, self.listener, room, wall) {
                let material = room.walls[wall];
                paths.push(Path {
                    position: image,
                    gain: distance_gain(length(sub(image, self.listener)))
                        * (1.0 - material.absorption)
                        * (1.0 - 0.5 * material.scattering),
                    absorption: material.absorption,
                });
            }
        }
        for &(image, first_wall) in &first {
            for second_wall in 0..6 {
                if second_wall == first_wall || paths.len() == 43 {
                    continue;
                }
                let image = mirror(image, room, second_wall);
                if reflection_visible(image, self.listener, room, second_wall) {
                    let a = (room.walls[first_wall].absorption
                        + room.walls[second_wall].absorption)
                        * 0.5;
                    paths.push(Path {
                        position: image,
                        gain: distance_gain(length(sub(image, self.listener)))
                            * (1.0 - a).powi(2)
                            * 0.02,
                        absorption: a,
                    });
                }
            }
        }
        for path in paths.iter_mut().skip(1).take(6) {
            path.gain *= 0.14;
        }
        paths
    }

    fn occlusion(&self, source: V3) -> (f32, f32) {
        if self
            .rooms
            .iter()
            .any(|room| contains(room, source) && contains(room, self.listener))
        {
            return (1.0, 0.0);
        }
        let direct = length(sub(source, self.listener));
        let around = self
            .rooms
            .iter()
            .flat_map(|room| &room.doorways)
            .map(|door| {
                let edge = closest(door, midpoint(source, self.listener));
                (length(sub(source, edge)) + length(sub(edge, self.listener)), door.material.scattering)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((around, scattering)) = around {
            (0.14 * direct.max(0.2) / around.max(direct), 0.76 + scattering * 0.1)
        } else {
            (0.035, 0.92)
        }
    }

    fn room_weights(&self, listener: V3) -> Vec<f32> {
        let mut weights: Vec<f32> = self
            .rooms
            .iter()
            .map(|room| {
                let outside = (0..3)
                    .map(|i| {
                        (room.min[i] - listener[i])
                            .max(listener[i] - room.max[i])
                            .max(0.0)
                    })
                    .sum::<f32>();
                (1.0 - outside / 0.8).clamp(0.0, 1.0)
            })
            .collect();
        let sum = weights.iter().sum::<f32>();
        if sum > 0.0 {
            weights.iter_mut().for_each(|weight| *weight /= sum);
        }
        weights
    }
}

fn parameters(path: &Path, source_gain: f32, listener: V3, forward: V3, up: V3, mode: Mode) -> Parameters {
    let offset = sub(path.position, listener);
    let distance = length(offset).max(0.05);
    let direction = scale(offset, 1.0 / distance);
    let right = unit(cross(forward, up));
    let azimuth = dot(direction, right).atan2(dot(direction, forward));
    let elevation = dot(direction, up).clamp(-1.0, 1.0).asin();
    let side = azimuth.sin();
    let theta = azimuth.abs();
    // Woodworth and Schlosberg's rigid-sphere interaural delay.
    let itd = HEAD_RADIUS / SPEED
        * if theta <= FRAC_PI_2 {
            theta + theta.sin()
        } else {
            PI - theta + theta.sin()
        };
    let delay = distance / SPEED * RATE;
    let near = (1.0 / distance.max(0.35)).min(1.8);
    let rear = ((-dot(direction, forward)).max(0.0) * 0.18).clamp(0.0, 0.18);
    let pan = ((side + 1.0) * PI * 0.25).clamp(0.0, FRAC_PI_2);
    let far = [side.max(0.0), (-side).max(0.0)];
    let notch = 8_000.0 + elevation / FRAC_PI_2 * 2_000.0;
    Parameters {
        delay: std::array::from_fn(|ear| delay + if mode == Mode::Headphones { itd * RATE * far[ear] } else { 0.0 }),
        gain: path.gain * source_gain * near,
        ear_gain: std::array::from_fn(|ear| if mode == Mode::Speakers { [pan.cos(), pan.sin()][ear] } else { 1.0 - 0.5 * far[ear] }),
        shadow: std::array::from_fn(|ear| coefficient(14_000.0 - 11_000.0 * far[ear])),
        air: coefficient(18_000.0 / (1.0 + 0.045 * distance * distance)),
        material: coefficient(18_000.0 * (1.0 - 0.82 * path.absorption)),
        bass: (1.0 - distance).max(0.0) * 0.45,
        bass_filter: coefficient(220.0),
        rear,
        rear_filter: coefficient(3_500.0),
        notch: [coefficient(notch - 700.0), coefficient(notch + 700.0)],
    }
}

fn binaural(parameters: &Parameters, previous: &mut PathState, history: &[f32], written: u64, n: usize, direct: bool, frame: &mut [f32; 2]) {
    if n == 0 {
        if !previous.initialized {
            previous.gain = if direct { parameters.gain } else { 0.0 };
            previous.delay = parameters.delay;
            previous.initialized = true;
        }
        previous.from_gain = previous.gain;
        previous.from_delay = previous.delay;
        let step = parameters.gain.max(previous.gain) * 0.005;
        previous.target_gain = parameters.gain.clamp(previous.gain - step, previous.gain + step);
        previous.target_delay = parameters.delay;
    }
    let t = (n + 1) as f32 / BLOCK as f32;
    let gain = lerp(previous.from_gain, previous.target_gain, t);
    for ear in 0..2 {
        let ear_delay = lerp(previous.from_delay[ear], previous.target_delay[ear], t);
        let mut sample = delayed(history, written, ear_delay);
        sample = lowpass(sample, parameters.shadow[ear], &mut previous.ears[ear].shadow);
        sample = lowpass(sample, parameters.air, &mut previous.ears[ear].air);
        sample = lowpass(sample, parameters.material, &mut previous.ears[ear].material);
        let bass = lowpass(sample, parameters.bass_filter, &mut previous.ears[ear].bass);
        sample += bass * parameters.bass;
        let rear_low = lowpass(sample, parameters.rear_filter, &mut previous.ears[ear].rear);
        sample -= (sample - rear_low) * parameters.rear;
        let lo = lowpass(sample, parameters.notch[0], &mut previous.ears[ear].notch_lo);
        let hi = lowpass(sample, parameters.notch[1], &mut previous.ears[ear].notch_hi);
        sample -= (hi - lo) * 0.24;
        frame[ear] += sample * gain * parameters.ear_gain[ear] * (1.0 - parameters.rear);
    }
    if n + 1 == BLOCK {
        previous.gain = previous.target_gain;
        previous.delay = previous.target_delay;
    }
}

fn delayed(history: &[f32], written: u64, age: f32) -> f32 {
    let position = written as f32 - age;
    if position < 0.0 || age >= HISTORY as f32 - 2.0 {
        return 0.0;
    }
    let base = position.floor() as u64;
    let fraction = position - base as f32;
    let a = history[(base as usize) & (HISTORY - 1)];
    let b = history[(base.wrapping_add(1) as usize) & (HISTORY - 1)];
    a + (b - a) * fraction
}

fn coefficient(cutoff: f32) -> f32 {
    1.0 - (-TAU * cutoff.clamp(80.0, 20_000.0) / RATE).exp()
}

fn lowpass(input: f32, a: f32, state: &mut f32) -> f32 {
    *state += (input - *state) * a;
    *state
}

fn room_decay(room: &Room) -> (f32, f32) {
    let size = sub(room.max, room.min);
    let volume = size[0] * size[1] * size[2];
    let areas = [
        size[1] * size[2],
        size[1] * size[2],
        size[0] * size[2],
        size[0] * size[2],
        size[0] * size[1],
        size[0] * size[1],
    ];
    let absorbed = (0..6)
        .map(|i| areas[i] * room.walls[i].absorption.max(0.01))
        .sum::<f32>();
    let rt60 = (0.161 * volume / absorbed).clamp(0.15, 4.0);
    let mean = room.walls.iter().map(|wall| wall.absorption).sum::<f32>() / 6.0;
    (
        10.0f32.powf(-3.0 * 0.04 / rt60),
        (0.42 - mean * 0.25).clamp(0.12, 0.42),
    )
}

fn reflection_visible(image: V3, listener: V3, room: &Room, wall: usize) -> bool {
    let axis = wall / 2;
    let plane = if wall % 2 == 0 {
        room.min[axis]
    } else {
        room.max[axis]
    };
    let t = (plane - listener[axis]) / (image[axis] - listener[axis]);
    (0.0..=1.0).contains(&t)
        && (0..3).filter(|&i| i != axis).all(|i| {
            let p = listener[i] + (image[i] - listener[i]) * t;
            p >= room.min[i] && p <= room.max[i]
        })
}

fn mirror(point: V3, room: &Room, wall: usize) -> V3 {
    let mut image = point;
    let axis = wall / 2;
    let plane = if wall % 2 == 0 {
        room.min[axis]
    } else {
        room.max[axis]
    };
    image[axis] = 2.0 * plane - image[axis];
    image
}

fn distance_gain(distance: f32) -> f32 {
    1.0 / distance.max(0.75)
}

fn contains(room: &Room, point: V3) -> bool {
    (0..3).all(|i| point[i] >= room.min[i] && point[i] <= room.max[i])
}

fn closest(door: &Doorway, point: V3) -> V3 {
    std::array::from_fn(|i| point[i].clamp(door.min[i], door.max[i]))
}

fn midpoint(a: V3, b: V3) -> V3 {
    scale(add(a, b), 0.5)
}

fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V3, s: f32) -> V3 {
    a.map(|v| v * s)
}
fn dot(a: V3, b: V3) -> f32 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn unit(a: V3) -> V3 {
    scale(a, 1.0 / length(a).max(1e-9))
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(min: V3, max: V3) -> Room {
        Room {
            min,
            max,
            walls: [Material::DRY_STONE; 6],
            doorways: Vec::new(),
        }
    }

    fn impulse_blocks(spatial: &mut Spatializer, source: Source, blocks: usize) -> Vec<[f32; 2]> {
        let mut result = Vec::new();
        for block in 0..blocks {
            let mut mono = [0.0; BLOCK];
            if block == 0 {
                mono[0] = 1.0;
            }
            let mut out = [[0.0; 2]; BLOCK];
            spatial.render(&source, &mono, &mut out);
            result.extend(out);
        }
        result
    }

    #[test]
    fn right_source_has_woodworth_itd_and_quieter_far_ear() {
        let mut spatial = Spatializer::new(&[], Mode::Headphones);
        spatial.set_listener([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
        let output = impulse_blocks(
            &mut spatial,
            Source {
                id: 0,
                position: [1.0, 0.0, 0.0],
                gain: 1.0,
            },
            3,
        );
        let peak = |ear: usize| {
            output
                .iter()
                .map(|frame| frame[ear].abs())
                .fold(0.0, f32::max)
        };
        let arrival = |ear: usize| {
            output
                .iter()
                .position(|frame| frame[ear].abs() > peak(ear) * 0.02)
                .unwrap()
        };
        let measured = arrival(0).abs_diff(arrival(1)) as f32 / RATE;
        let expected = HEAD_RADIUS / SPEED * (FRAC_PI_2 + 1.0);
        assert!(
            (measured - expected).abs() <= 20e-6,
            "measured {measured}, expected {expected}"
        );
        assert!(peak(0) < peak(1));
    }

    #[test]
    fn wall_occlusion_lowers_rms_and_spectral_centroid() {
        let rooms = [
            room([-3.0, -2.0, -3.0], [0.0, 2.0, 3.0]),
            room([0.2, -2.0, -3.0], [3.2, 2.0, 3.0]),
        ];
        let measure = |position| {
            let mut spatial = Spatializer::new(&rooms, Mode::Headphones);
            spatial.set_listener([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let mono: Vec<f32> = (0..BLOCK)
                .map(|n| (n * 7919 % 101) as f32 / 50.0 - 1.0)
                .collect();
            let mut out = [[0.0; 2]; BLOCK];
            for _ in 0..4 {
                spatial.render(
                    &Source {
                        id: 0,
                        position,
                        gain: 1.0,
                    },
                    &mono,
                    &mut out,
                );
            }
            let signal: Vec<f32> = out.iter().map(|p| p[0] + p[1]).collect();
            let rms = (signal.iter().map(|v| v * v).sum::<f32>() / BLOCK as f32).sqrt();
            let centroid = signal.windows(2).map(|p| (p[1] - p[0]).abs()).sum::<f32>()
                / signal.iter().map(|v| v.abs()).sum::<f32>();
            (rms, centroid)
        };
        let visible = measure([-2.0, 0.0, 0.0]);
        let blocked = measure([1.2, 0.0, 0.0]);
        assert!(blocked.0 < visible.0 && blocked.1 < visible.1);
    }

    #[test]
    fn doorway_crossfade_changes_reverb_energy_monotonically() {
        let reflective = room([-4.0, -2.0, -3.0], [0.0, 2.0, 3.0]);
        let mut dry = room([0.0, -2.0, -3.0], [4.0, 2.0, 3.0]);
        dry.walls = [Material::OPEN; 6];
        let energies: Vec<f32> = [-0.6, -0.3, 0.0, 0.3, 0.6]
            .into_iter()
            .map(|x| {
                let mut spatial =
                    Spatializer::new(&[reflective.clone(), dry.clone()], Mode::Headphones);
                spatial.set_listener([x, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]);
                let source = Source {
                    id: 0,
                    position: [-1.0, 0.0, 0.0],
                    gain: 1.0,
                };
                let mut energy = 0.0;
                for block in 0..30 {
                    let mono = if block == 0 {
                        [1.0; BLOCK]
                    } else {
                        [0.0; BLOCK]
                    };
                    let mut out = [[0.0; 2]; BLOCK];
                    spatial.render(&source, &mono, &mut out);
                    if block > 8 {
                        energy += out.iter().map(|p| p[0] * p[0] + p[1] * p[1]).sum::<f32>();
                    }
                }
                energy
            })
            .collect();
        assert!(energies.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn first_order_images_arrive_at_geometric_delays() {
        let room = room([-5.0, -4.0, -6.0], [7.0, 5.0, 8.0]);
        let listener = [0.0, 0.0, 0.0];
        let source = [1.0, 0.5, -1.0];
        for wall in 0..6 {
            let image = mirror(source, &room, wall);
            let expected = length(sub(image, listener)) / SPEED * RATE;
            let mut history = vec![0.0; HISTORY];
            history[0] = 1.0;
            let mut state = PathState::default();
            let path = Path {
                position: image,
                gain: 1.0,
                absorption: 0.0,
            };
            let parameters = parameters(&path, 1.0, listener, [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], Mode::Speakers);
            let mut arrival = None;
            for n in 0..=(expected.ceil() as usize + 2) {
                let mut frame = [0.0; 2];
                binaural(
                    &parameters,
                    &mut state,
                    &history,
                    n as u64,
                    n % BLOCK,
                    true,
                    &mut frame,
                );
                if frame.iter().any(|sample| sample.abs() > 1e-7) {
                    arrival = Some(n);
                    break;
                }
            }
            assert!(reflection_visible(image, listener, &room, wall));
            assert!((arrival.unwrap() as f32 - expected).abs() <= 1.0);
        }
    }
}
