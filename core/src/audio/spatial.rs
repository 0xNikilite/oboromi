//! Block binaural rendering and shoebox-room acoustics for host-driven game audio.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const BLOCK: usize = 240;
const RATE: f32 = 48_000.0;
const SPEED: f32 = 343.0;
const HEAD_RADIUS: f32 = 0.0875;
const HISTORY: usize = 1 << 17;
const IMAGE_PATHS: usize = 3;
const GEOMETRY_BLOCKS: u64 = 10;

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
            self.at[i] += 1;
            if self.at[i] == self.lines[i].len() {
                self.at[i] = 0;
            }
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
    paths: [PathState; IMAGE_PATHS + 1],
    images: [Path; IMAGE_PATHS],
    image_count: usize,
    image_listener: V3,
    image_source: V3,
    geometry_block: u64,
}

impl SourceState {
    fn new() -> Self {
        Self {
            history: vec![0.0; HISTORY],
            written: 0,
            paths: [PathState::default(); IMAGE_PATHS + 1],
            images: [Path::default(); IMAGE_PATHS],
            image_count: 0,
            image_listener: [f32::INFINITY; 3],
            image_source: [f32::INFINITY; 3],
            geometry_block: 0,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Path {
    position: V3,
    gain: f32,
    absorption: f32,
}

#[derive(Clone, Copy, Default)]
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
    reveal_room: Option<usize>,
    mode: Mode,
    listener: V3,
    forward: V3,
    up: V3,
    states: HashMap<usize, SourceState>,
    reverbs: Vec<Fdn>,
    room_weights: Vec<f32>,
    room_data: Vec<(f32, f32)>,
    reverb_input: Vec<[f32; BLOCK]>,
    rooms_initialized: bool,
    block: u64,
}

impl Spatializer {
    pub fn new(rooms: &[Room], mode: Mode) -> Self {
        let reveal_room = rooms.iter().enumerate().max_by(|(_, a), (_, b)| room_volume(a).total_cmp(&room_volume(b))).map(|(i, _)| i);
        Self {
            rooms: rooms.to_vec(),
            reveal_room,
            mode,
            listener: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
            states: HashMap::new(),
            reverbs: (0..rooms.len()).map(|_| Fdn::new()).collect(),
            room_weights: vec![0.0; rooms.len()],
            room_data: rooms.iter().map(room_decay).collect(),
            reverb_input: vec![[0.0; BLOCK]; rooms.len()],
            rooms_initialized: false,
            block: 0,
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

    pub fn begin_block(&mut self) {
        let targets = self.room_weights(self.listener);
        if self.rooms_initialized {
            for (weight, target) in self.room_weights.iter_mut().zip(targets) {
                *weight += (target - *weight).clamp(-0.005, 0.005);
            }
        } else {
            self.room_weights = targets;
            self.rooms_initialized = true;
        }
        self.reverb_input.iter_mut().for_each(|input| input.fill(0.0));
    }

    pub fn finish_block(&mut self, out: &mut [[f32; 2]]) {
        for (room, ((reverb, input), &(decay, damping))) in self.reverbs.iter_mut().zip(&self.reverb_input).zip(&self.room_data).enumerate() {
            let weight = self.room_weights[room];
            for (frame, &dry) in out.iter_mut().zip(input) {
                let wet = reverb.process(dry, decay, damping);
                frame[0] += wet[0] * weight;
                frame[1] += wet[1] * weight;
            }
        }
        self.block += 1;
    }

    pub fn render(&mut self, source: &Source, mono: &[f32], out: &mut [[f32; 2]]) {
        assert_eq!(mono.len(), out.len());
        assert!(mono.len() <= BLOCK);
        out.fill([0.0; 2]);
        let refresh = self.states.get(&source.id).is_none_or(|state| {
            self.block - state.geometry_block >= GEOMETRY_BLOCKS
                && (distance_squared(self.listener, state.image_listener) > 0.04
                    || distance_squared(source.position, state.image_source) > 0.04)
        });
        let (images, image_count) = if refresh {
            self.image_paths(source.position)
        } else {
            let state = &self.states[&source.id];
            (state.images, state.image_count)
        };
        let direct = self.direct_path(source.position);
        let mut table = [Parameters::default(); IMAGE_PATHS + 1];
        table[0] = parameters(&direct, source.gain, self.listener, self.forward, self.up, self.mode);
        for (parameter, path) in table[1..].iter_mut().zip(&images[..image_count]) {
            *parameter = parameters(path, source.gain, self.listener, self.forward, self.up, self.mode);
        }
        let state = self
            .states
            .entry(source.id)
            .or_insert_with(SourceState::new);
        if refresh {
            state.images = images;
            state.image_count = image_count;
            state.image_listener = self.listener;
            state.image_source = source.position;
            state.geometry_block = self.block;
        }
        for &sample in mono {
            state.history[(state.written as usize) & (HISTORY - 1)] = sample;
            state.written += 1;
        }
        let block_start = state.written - mono.len() as u64;
        if table[0].gain.abs() >= 0.001 {
            for (index, parameter) in table[..=image_count].iter().enumerate() {
                binaural_block(parameter, &mut state.paths[index], &state.history, block_start, index == 0, out);
            }
            for (room, input) in self.reverb_input.iter_mut().enumerate() {
                let send = table[0].gain * 0.08 * self.room_weights[room];
                for (dry, &sample) in input.iter_mut().zip(mono) {
                    *dry += sample * send;
                }
            }
        }
        state.written = block_start + mono.len() as u64;
    }

    fn direct_path(&self, source: V3) -> Path {
        let listener_room = self.rooms.iter().position(|room| contains(room, self.listener));
        let source_room = self.rooms.iter().position(|room| contains(room, source));
        if self.rooms.is_empty() || listener_room == source_room && source_room.is_some() {
            return Path { position: source, gain: distance_gain(length(sub(source, self.listener))), absorption: 0.0 };
        }
        let Some(reveal) = self.reveal_room.filter(|&room| source_room == Some(room)) else { return Path::default() };
        let Some((door, position, distance)) = self.rooms[reveal].doorways.iter().map(|door| {
            let position = closest(door, self.listener);
            (door, position, length(sub(position, self.listener)))
        }).min_by(|a, b| a.2.total_cmp(&b.2)) else { return Path::default() };
        let route = length(sub(source, position)) + distance;
        let opening = 0.08 + 0.14 * (1.0 - distance / 10.0).clamp(0.0, 1.0).powi(2);
        Path { position, gain: distance_gain(route) * opening, absorption: 0.8 + door.material.scattering * 0.1 }
    }

    fn image_paths(&self, source: V3) -> ([Path; IMAGE_PATHS], usize) {
        let mut paths = Vec::with_capacity(42);
        let Some(room) = self
            .rooms
            .iter()
            .find(|room| contains(room, source) && contains(room, self.listener))
        else {
            return ([Path::default(); IMAGE_PATHS], 0);
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
                if second_wall == first_wall {
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
        for path in paths.iter_mut().take(6) {
            path.gain *= 0.14;
        }
        paths.sort_unstable_by(|a, b| length(sub(a.position, self.listener)).total_cmp(&length(sub(b.position, self.listener))));
        let count = paths.len().min(IMAGE_PATHS);
        let mut images = [Path::default(); IMAGE_PATHS];
        images[..count].copy_from_slice(&paths[..count]);
        (images, count)
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

fn binaural_block(parameters: &Parameters, previous: &mut PathState, history: &[f32], written: u64, direct: bool, out: &mut [[f32; 2]]) {
    if !previous.initialized {
        previous.gain = if direct { parameters.gain } else { 0.0 };
        previous.delay = parameters.delay;
        previous.initialized = true;
    }
    let from_gain = previous.gain;
    let from_delay = previous.delay;
    let step = parameters.gain.max(previous.gain) * 0.005;
    let target_gain = parameters.gain.clamp(previous.gain - step, previous.gain + step);
    for (n, frame) in out.iter_mut().enumerate() {
        let t = (n + 1) as f32 / BLOCK as f32;
        let gain = lerp(from_gain, target_gain, t);
        for ear in 0..2 {
            let ear_delay = lerp(from_delay[ear], parameters.delay[ear], t);
            let mut sample = delayed(history, written + n as u64, ear_delay);
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
    }
    previous.gain = target_gain;
    previous.delay = parameters.delay;
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

fn room_volume(room: &Room) -> f32 {
    (room.max[0] - room.min[0]) * (room.max[1] - room.min[1]) * (room.max[2] - room.min[2])
}

fn closest(door: &Doorway, point: V3) -> V3 {
    std::array::from_fn(|i| point[i].clamp(door.min[i], door.max[i]))
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
fn distance_squared(a: V3, b: V3) -> f32 {
    let d = sub(a, b);
    dot(d, d)
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
            spatial.begin_block();
            spatial.render(&source, &mono, &mut out);
            spatial.finish_block(&mut out);
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
        let mut reveal = room([0.2, -2.0, -3.0], [3.4, 2.0, 3.0]);
        reveal.doorways.push(Doorway { min: [0.15, -1.0, -0.8], max: [0.25, 1.5, 0.8], material: Material::OPEN });
        let rooms = [room([-3.0, -2.0, -3.0], [0.0, 2.0, 3.0]), reveal];
        let measure = |position| {
            let mut spatial = Spatializer::new(&rooms, Mode::Headphones);
            spatial.set_listener([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
            let mono: Vec<f32> = (0..BLOCK)
                .map(|n| (n * 7919 % 101) as f32 / 50.0 - 1.0)
                .collect();
            let mut out = [[0.0; 2]; BLOCK];
            for _ in 0..4 {
                spatial.begin_block();
                spatial.render(
                    &Source {
                        id: 0,
                        position,
                        gain: 1.0,
                    },
                    &mono,
                    &mut out,
                );
                spatial.finish_block(&mut out);
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
    fn only_the_reveal_carries_through_its_doorway() {
        let hall = room([-8.0, -2.0, -3.0], [0.0, 2.0, 3.0]);
        let mut reveal = room([0.0, -2.0, -4.0], [8.0, 3.0, 4.0]);
        reveal.doorways.push(Doorway { min: [-0.05, -1.0, -0.8], max: [0.05, 1.5, 0.8], material: Material::OPEN });
        let mut spatial = Spatializer::new(&[hall, reveal], Mode::Headphones);
        spatial.set_listener([-7.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let far = spatial.direct_path([4.0, 0.0, 0.0]).gain;
        spatial.set_listener([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let near = spatial.direct_path([4.0, 0.0, 0.0]).gain;
        spatial.set_listener([4.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!(far > 0.0 && far < near && near < spatial.direct_path([3.0, 0.0, 0.0]).gain);
        assert_eq!(spatial.direct_path([-4.0, 0.0, 0.0]).gain, 0.0);
    }

    #[test]
    fn doorway_crossfade_changes_reverb_energy_monotonically() {
        let mut reflective = room([-4.2, -2.0, -3.0], [0.0, 2.0, 3.0]);
        reflective.doorways.push(Doorway { min: [-0.05, -1.0, -0.8], max: [0.05, 1.5, 0.8], material: Material::OPEN });
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
                    spatial.begin_block();
                    spatial.render(&source, &mono, &mut out);
                    spatial.finish_block(&mut out);
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
            for block in 0..=(expected.ceil() as usize / BLOCK + 1) {
                let mut out = [[0.0; 2]; BLOCK];
                binaural_block(&parameters, &mut state, &history, (block * BLOCK) as u64, true, &mut out);
                if let Some(n) = out.iter().position(|frame| frame.iter().any(|sample| sample.abs() > 1e-7)) {
                    arrival = Some(block * BLOCK + n);
                    break;
                }
            }
            assert!(reflection_visible(image, listener, &room, wall));
            assert!((arrival.unwrap() as f32 - expected).abs() <= 1.0);
        }
    }
}
