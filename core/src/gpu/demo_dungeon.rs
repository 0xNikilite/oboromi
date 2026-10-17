//! A first-person dungeon with HDR PBR materials, a compute river and positional emulated audio.
use super::demo::{Matrix, multiply};
use super::{VkState, device};
use super::demo_dungeon_sound::Sound;
use std::f32::consts::{PI, TAU};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
#[path = "demo_dungeon_render.rs"]
pub mod render;
pub static VIEW: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static CAPTURE_SOLO: AtomicU8 = AtomicU8::new(0);

pub fn set_dungeon_audio_solo(name: &str) {
    CAPTURE_SOLO.store(match name { "torch" => 1, "river" => 2, "footsteps" => 3, "drips" => 4, _ => 0 }, Ordering::Relaxed);
}

pub type V3 = [f32; 3];
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn scale(a: V3, s: f32) -> V3 {
    a.map(|c| c * s)
}
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
pub fn unit(a: V3) -> V3 {
    scale(a, 1.0 / length(a).max(1e-9))
}
fn mix3(a: V3, b: V3, t: f32) -> V3 {
    add(a, scale(sub(b, a), t))
}
fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub struct Rng(pub u32);

impl Rng {
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / 16_777_216.0
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

fn hash(x: i32, y: i32, seed: i32) -> f32 {
    let mut h = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ seed.wrapping_mul(1_274_126_177)) as u32;
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) as f32 / 4_294_967_296.0
}

/// Value noise in 0..1 that repeats every `period` cells.
fn noise(x: f32, y: f32, period: i32, seed: i32) -> f32 {
    noise_xy(x, y, [period; 2], seed)
}

fn noise_xy(x: f32, y: f32, period: [i32; 2], seed: i32) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (tx, ty) = (x - fx, y - fy);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let corner = |dx: i32, dy: i32| hash((fx as i32 + dx).rem_euclid(period[0]), (fy as i32 + dy).rem_euclid(period[1]), seed);
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// Octaves of `noise` from `period` cells up, summed to 0..1.
fn fbm(x: f32, y: f32, period: i32, octaves: u32, seed: i32) -> f32 {
    let (mut sum, mut amplitude, mut total) = (0.0, 1.0, 0.0);
    for octave in 0..octaves {
        let f = (1 << octave) as f32;
        sum += amplitude * noise(x * f, y * f, period << octave, seed + octave as i32);
        total += amplitude;
        amplitude *= 0.5;
    }
    sum / total
}

#[derive(Clone, Copy, Debug)]
pub struct Box3 {
    pub lo: V3,
    pub hi: V3,
}

impl Box3 {
    pub fn contains(&self, p: V3) -> bool {
        (0..3).all(|i| p[i] >= self.lo[i] && p[i] <= self.hi[i])
    }
    fn centre(&self) -> V3 {
        scale(add(self.lo, self.hi), 0.5)
    }
    fn shrink(&self, by: f32) -> Box3 {
        Box3 { lo: [self.lo[0] + by, self.lo[1], self.lo[2] + by], hi: [self.hi[0] - by, self.hi[1], self.hi[2] - by] }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Room,
    Corridor,
    Cave,
}

pub struct Zone {
    pub space: Box3,
    pub kind: Kind,
}

/// A hole in a zone wall: the wall's normal axis (0 or 2) and side, the span along the other axis and the top.
struct Door {
    zone: usize,
    axis: usize,
    side: usize,
    span: [f32; 2],
    top: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    Torch,
    Fire,
    River,
    Wind,
    Heart,
    Creak,
    Bats,
}

pub struct Light {
    pub pos: V3,
    pub colour: V3,
    /// 0 for a steady light, else how much it flickers.
    pub flicker: f32,
    /// flame sprite height, 0 for none.
    pub flame: f32,
}

pub enum Prop {
    Skull(V3, f32),
    Bone(V3, f32),
    Candle(V3, f32),
    Lantern(V3, f32),
    Rock(V3, f32),
    Log(V3, f32),
    Chain(V3, f32),
    Cage(V3),
    Web(V3, [f32; 2]),
    Stalactite(V3, f32, f32),
    Stalagmite(V3, f32, f32),
    Bracket(V3, V3),
}

pub struct World {
    pub zones: Vec<Zone>,
    doors: Vec<Door>,
    /// where the listener can stand.
    pub walk: Vec<Box3>,
    pub lights: Vec<Light>,
    pub sources: Vec<(Sfx, V3, f32)>,
    /// drip emitters: the tip and where the drop lands.
    pub drips: Vec<(V3, f32)>,
    pub path: Vec<V3>,
    pub props: Vec<Prop>,
    pub cave: usize,
    pub river: Box3,
    pub bridge: Box3,
    pub grate: Box3,
    /// the corridor the bats circle in, and the room the scream comes from.
    pub bats: usize,
    pub last: usize,
}

pub const WATER: f32 = -0.7;
const BED: f32 = -2.0;
const DOOR_TOP: f32 = 3.0;
pub const EYE: f32 = 1.65;

fn snap(v: f32) -> f32 {
    (v * 2.0).round() / 2.0
}

impl World {
    pub fn acoustic_rooms(&self) -> Vec<crate::audio::spatial::Room> {
        use crate::audio::spatial::{Doorway, Material, Room};
        self.zones.iter().enumerate().map(|(zone, room)| {
            let stone = if room.kind == Kind::Cave { Material::WET_STONE } else { Material::DRY_STONE };
            let mut walls = [stone; 6];
            if room.kind == Kind::Cave { walls[2] = Material::WATER; }
            let doorways = self.doors.iter().filter(|door| door.zone == zone).map(|door| {
                let (mut min, mut max) = (room.space.lo, room.space.hi);
                let plane = if door.side == 0 { room.space.lo[door.axis] } else { room.space.hi[door.axis] };
                min[door.axis] = plane - 0.05;
                max[door.axis] = plane + 0.05;
                min[2 - door.axis] = door.span[0];
                max[2 - door.axis] = door.span[1];
                min[1] = room.space.lo[1];
                max[1] = door.top;
                Doorway { min, max, material: Material::OPEN }
            }).collect();
            Room { min: room.space.lo, max: room.space.hi, walls, doorways }
        }).collect()
    }

    /// The zones containing `p`.
    pub fn zones_at(&self, p: V3) -> impl Iterator<Item = usize> + '_ {
        (0..self.zones.len()).filter(move |&z| self.zones[z].space.contains(p))
    }

    /// How many stretches of rock the segment from `a` to `b` passes through.
    pub fn walls_between(&self, a: V3, b: V3) -> u32 {
        let direction = sub(b, a);
        let mut spans = Vec::new();
        for zone in &self.zones {
            let (mut enter, mut leave) = (0.0f32, 1.0f32);
            for axis in 0..3 {
                if direction[axis].abs() < 1e-7 {
                    if a[axis] < zone.space.lo[axis] || a[axis] > zone.space.hi[axis] { leave = -1.0; }
                } else {
                    let p = (zone.space.lo[axis] - a[axis]) / direction[axis];
                    let q = (zone.space.hi[axis] - a[axis]) / direction[axis];
                    enter = enter.max(p.min(q));
                    leave = leave.min(p.max(q));
                }
            }
            if enter <= leave { spans.push((enter, leave)); }
        }
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mut end, mut walls) = (0.0f32, 0);
        for (enter, leave) in spans {
            walls += u32::from(enter > end + 1e-5);
            end = end.max(leave);
        }
        walls + u32::from(end < 1.0 - 1e-5)
    }

    /// The cave's inward wall and ceiling displacement at `p`, zero at doors so the corridors meet it.
    pub fn rock(&self, p: V3) -> f32 {
        let door = self.doors.iter().filter(|d| d.zone == self.cave).map(|d| {
            let along = p[2 - d.axis];
            let gap = (d.span[0] - along).max(along - d.span[1]).max(p[1] - d.top).max(0.0);
            let plane = self.zones[self.cave].space.hi[d.axis] * d.side as f32 + self.zones[self.cave].space.lo[d.axis] * (1 - d.side) as f32;
            gap.max((p[d.axis] - plane).abs() - 0.5)
        });
        let near_door = smooth(0.2, 2.0, door.fold(f32::MAX, f32::min));
        let n = fbm(p[0] * 0.35 + p[1] * 0.2, p[2] * 0.35 - p[1] * 0.27, 64, 4, 9);
        near_door * (0.15 + 1.1 * n * n)
    }

    /// The cave ceiling's height over (`x`, `z`).
    pub fn ceiling(&self, x: f32, z: f32) -> f32 {
        let top = self.zones[self.cave].space.hi[1];
        top - 2.2 * self.rock([x, top, z])
    }

    pub fn new(seed: u32) -> World {
        let mut r = Rng(seed | 1);
        // direction from the previous chamber, width, depth, height, kind.
        let plan = [([0, 0], 10.0, 9.0, 4.5, Kind::Room), ([1, 0], 11.0, 10.0, 5.5, Kind::Room), ([1, 0], 16.0, 14.0, 8.0, Kind::Cave), ([0, 1], 10.0, 10.0, 5.0, Kind::Room), ([-1, 0], 9.0, 9.0, 4.5, Kind::Room)];
        let mut zones: Vec<Zone> = Vec::new();
        let mut doors = Vec::new();
        let mut corridors = Vec::new();
        let mut chambers = Vec::new();
        for &(dir, w, d, h, kind) in &plan {
            let [w, d] = [w, d].map(|s: f32| snap(s + r.range(-1.0, 1.0)));
            let size = [w, h, d];
            let low = if kind == Kind::Cave { BED } else { 0.0 };
            let Some(&prev) = chambers.last() else {
                chambers.push(zones.len());
                zones.push(Zone { space: Box3 { lo: [-w / 2.0, low, -d / 2.0], hi: [w / 2.0, h, d / 2.0] }, kind });
                continue;
            };
            let p: Box3 = zones[prev].space;
            let axis = if dir[0] != 0 { 0 } else { 2 };
            let across = 2 - axis;
            let gap = snap(r.range(4.5, 6.5));
            let sign = (dir[0] + dir[1]) as f32;
            let mut centre = p.centre();
            centre[axis] += sign * (p.hi[axis] - p.lo[axis] + size[axis]) / 2.0 + sign * gap;
            centre[across] = snap(centre[across] + r.range(-1.5, 1.5));
            let space = Box3 { lo: [centre[0] - w / 2.0, low, centre[2] - d / 2.0], hi: [centre[0] + w / 2.0, h, centre[2] + d / 2.0] };
            let (lo, hi) = (p.lo[across].max(space.lo[across]) + 1.5, p.hi[across].min(space.hi[across]) - 1.5);
            let mut c = snap(r.range(lo, hi));
            if kind == Kind::Cave {
                c = c.min(snap(centre[2]) - 1.5);
            }
            let [near, far] = if sign > 0.0 { [p.hi[axis], space.lo[axis]] } else { [space.hi[axis], p.lo[axis]] };
            let mut corridor = Box3 { lo: [0.0; 3], hi: [0.0, DOOR_TOP, 0.0] };
            [corridor.lo[axis], corridor.hi[axis], corridor.lo[across], corridor.hi[across]] = [near, far, c - 1.0, c + 1.0];
            let next = zones.len();
            for (zone, side) in [(prev, usize::from(sign > 0.0)), (next, usize::from(sign < 0.0))] {
                doors.push(Door { zone, axis, side, span: [c - 1.0, c + 1.0], top: DOOR_TOP });
            }
            chambers.push(next);
            zones.push(Zone { space, kind });
            corridors.push((corridor, axis, c, prev, next));
        }
        for &(space, ..) in &corridors {
            zones.push(Zone { space, kind: Kind::Corridor });
        }
        let cave = chambers[2];
        let cs = zones[cave].space;
        let cz = snap(cs.centre()[2]);
        let river = Box3 { lo: [cs.lo[0], BED, cz + 0.5], hi: [cs.hi[0], WATER, cz + 4.0] };
        let south = corridors[2].2;
        let bridge = Box3 { lo: [south - 1.2, 0.0, river.lo[2] - 0.6], hi: [south + 1.2, 0.0, river.hi[2] + 0.6] };

        let mut world = World {
            zones,
            doors,
            walk: Vec::new(),
            lights: Vec::new(),
            sources: Vec::new(),
            drips: Vec::new(),
            path: Vec::new(),
            props: Vec::new(),
            cave,
            river,
            bridge,
            grate: Box3 { lo: [0.0; 3], hi: [0.0; 3] },
            bats: chambers.len() + 1,
            last: chambers[4],
        };
        for (i, &(c, axis, at, ..)) in corridors.iter().enumerate() {
            let shrunk = if axis == 0 { Box3 { lo: [c.lo[0] - 1.6, 0.0, at - 0.6], hi: [c.hi[0] + 1.6, 0.0, at + 0.6] } } else { Box3 { lo: [at - 0.6, 0.0, c.lo[2] - 1.6], hi: [at + 0.6, 0.0, c.hi[2] + 1.6] } };
            world.walk.push(shrunk);
            if i == 3 {
                world.sources.push((Sfx::Wind, add(c.centre(), [0.0, 1.0, 0.0]), 0.9));
            }
        }
        for &z in &chambers {
            let s = world.zones[z].space;
            match world.zones[z].kind {
                Kind::Cave => {
                    world.walk.push(Box3 { lo: [s.lo[0] + 1.6, 0.0, s.lo[2] + 1.6], hi: [s.hi[0] - 1.6, 0.0, river.lo[2] - 0.5] });
                    world.walk.push(Box3 { lo: [s.lo[0] + 1.6, 0.0, river.hi[2] + 0.5], hi: [s.hi[0] - 1.6, 0.0, s.hi[2] - 1.6] });
                    world.walk.push(Box3 { lo: [bridge.lo[0] + 0.5, 0.0, bridge.lo[2]], hi: [bridge.hi[0] - 0.5, 0.0, bridge.hi[2]] });
                }
                _ => world.walk.push(s.shrink(0.45)),
            }
        }
        world.furnish(&chambers, &corridors.iter().map(|&(c, axis, at, ..)| (c, axis, at)).collect::<Vec<_>>(), &mut r);
        world
    }

    fn furnish(&mut self, chambers: &[usize], corridors: &[(Box3, usize, f32)], r: &mut Rng) {
        let torch = [1.0, 0.42, 0.13];
        let wall_point = |s: &Box3, wall: usize, t: f32, inset: f32, y: f32| -> (V3, V3) {
            let (axis, side) = (if wall < 2 { 0 } else { 2 }, wall % 2);
            let mut p = [0.0, y, 0.0];
            let mut n = [0.0; 3];
            p[axis] = if side == 1 { s.hi[axis] - inset } else { s.lo[axis] + inset };
            n[axis] = if side == 1 { -1.0 } else { 1.0 };
            p[2 - axis] = s.lo[2 - axis] + (s.hi[2 - axis] - s.lo[2 - axis]) * t;
            (p, n)
        };
        let door_clear = |world: &World, z: usize, wall: usize, t: f32, margin: f32| {
            let s = world.zones[z].space;
            let (axis, side) = (if wall < 2 { 0 } else { 2 }, wall % 2);
            let along = s.lo[2 - axis] + (s.hi[2 - axis] - s.lo[2 - axis]) * t;
            world.doors.iter().filter(|d| d.zone == z && d.axis == axis && d.side == side).all(|d| along < d.span[0] - margin || along > d.span[1] + margin)
        };
        for (n, &z) in chambers.iter().enumerate() {
            let s = self.zones[z].space;
            let cave = z == self.cave;
            // Torches on the walls, away from the doors.
            let mut placed = 0;
            for wall in [2, 1, 3, 0].into_iter().filter(|_| !cave) {
                for t in [0.5, 0.3, 0.7] {
                    if placed < 3 && door_clear(self, z, wall, t, 1.6) {
                        let (p, normal) = wall_point(&s, wall, t, 0.28, 2.3);
                        self.lights.push(Light { pos: p, colour: scale(torch, 3.2), flicker: 1.0, flame: 0.42 });
                        self.props.push(Prop::Bracket(add(p, [0.0, -0.05, 0.0]), normal));
                        self.sources.push((Sfx::Torch, p, 0.55));
                        placed += 1;
                        break;
                    }
                }
            }
            if cave {
                for x in [self.bridge.lo[0] - 0.1, self.bridge.hi[0] + 0.1] {
                    let p = [x, 1.5, self.bridge.lo[2] + 0.2];
                    self.lights.push(Light { pos: p, colour: scale(torch, 2.4), flicker: 1.0, flame: 0.32 });
                    self.sources.push((Sfx::Torch, p, 0.45));
                }
                self.lights.push(Light { pos: [s.centre()[0], WATER + 0.4, self.river.centre()[2]], colour: [0.05, 0.13, 0.2], flicker: 0.0, flame: 0.0 });
                for i in 0..4 {
                    let x = s.lo[0] + (s.hi[0] - s.lo[0]) * (0.12 + 0.25 * i as f32);
                    self.sources.push((Sfx::River, [x, WATER, self.river.centre()[2]], 1.3));
                }
                for _ in 0..60 {
                    let p = [r.range(s.lo[0] + 1.0, s.hi[0] - 1.0), 0.0, r.range(s.lo[2] + 1.0, s.hi[2] - 1.0)];
                    let on_walk = self.walk[self.walk.len() - 3..].iter().any(|w| w.shrink(-0.6).contains([p[0], 0.0, p[2]]));
                    let near_path = (p[0] - self.bridge.centre()[0]).abs() < 2.2;
                    let top = self.ceiling(p[0], p[2]) + 0.4;
                    let length = r.range(0.4, 2.4);
                    self.props.push(Prop::Stalactite([p[0], top, p[2]], length, r.range(0.12, 0.42)));
                    let over_water = self.river.contains([p[0], WATER - 0.1, p[2]]);
                    let land = if over_water { WATER } else { 0.0 };
                    if !over_water && !on_walk && !near_path && r.next() < 0.6 {
                        let h = r.range(0.4, 1.8).min(top - length - 1.0);
                        self.props.push(Prop::Stalagmite([p[0], 0.0, p[2]], h, r.range(0.18, 0.4)));
                        self.drips.push(([p[0]+(p[0]*3.).sin()*0.17, top - length, p[2]+(p[2]*2.).cos()*0.17], h));
                    } else if r.next() < 0.5 {
                        self.drips.push(([p[0]+(p[0]*3.).sin()*0.17, top - length, p[2]+(p[2]*2.).cos()*0.17], land));
                    }
                }
                continue;
            }
            // Webs in the upper corners, skulls and bones along the walls.
            for corner in 0..4 {
                let (x, zz) = (if corner & 1 == 0 { s.lo[0] } else { s.hi[0] }, if corner & 2 == 0 { s.lo[2] } else { s.hi[2] });
                self.props.push(Prop::Web([x, s.hi[1], zz], [if corner & 1 == 0 { 1.0 } else { -1.0 }, if corner & 2 == 0 { 1.0 } else { -1.0 }]));
            }
            for _ in 0..6 {
                let wall = (r.next() * 4.0) as usize;
                let t = r.range(0.1, 0.9);
                if door_clear(self, z, wall, t, 0.8) {
                    let (p, _) = wall_point(&s, wall, t, r.range(0.35, 1.0), 0.0);
                    self.props.push(if r.next() < 0.45 { Prop::Skull(p, r.range(0.0, TAU)) } else { Prop::Bone(p, r.range(0.0, TAU)) });
                    for _ in 0..2 {
                        self.props.push(Prop::Bone(add(p, [r.range(-0.4, 0.4), 0.0, r.range(-0.4, 0.4)]), r.range(0.0, TAU)));
                    }
                }
            }
            // Candles in a free corner.
            let corner = [s.lo[0] + 0.7, 0.0, s.hi[2] - 0.7];
            if n != 1 {
                for i in 0..7 {
                    let a = i as f32 * 2.4;
                    let p = add(corner, [0.35 * a.cos() * (i as f32 / 7.0), 0.0, 0.35 * a.sin() * (i as f32 / 7.0)]);
                    let h = r.range(0.08, 0.32);
                    self.props.push(Prop::Candle(p, h));
                    self.lights.push(Light { pos: add(p, [0.0, h + 0.05, 0.0]), colour: scale([1.0, 0.6, 0.25], 0.18), flicker: 0.6, flame: 0.07 });
                }
            }
            let centre = s.centre();
            match n {
                0 | 4 => {
                    for k in 0..2 {
                        let p = [if k == 0 { s.hi[0] - 1.1 } else { s.lo[0] + 1.3 }, 0.0, if k == 0 { s.hi[2] - 1.2 } else { s.lo[2] + 1.2 }];
                        let face = (centre[0] - p[0]).atan2(centre[2] - p[2]);
                        self.props.push(Prop::Lantern(p, face));
                        self.lights.push(Light { pos: add(p, [0.0, 0.25, 0.0]), colour: [1.0, 0.45, 0.08], flicker: 0.8, flame: 0.0 });
                    }
                    if n == 4 {
                        self.sources.push((Sfx::Heart, add(centre, [0.0, 1.0, 0.0]), 1.4));
                    }
                }
                1 => {
                    let doors_z: f32 = self.doors.iter().filter(|d| d.zone == z).map(|d| d.span[0] + 1.0).sum::<f32>() / 2.0;
                    let pit = [centre[0], 0.0, centre[2] + if doors_z < centre[2] { 2.4 } else { -2.4 }];
                    for i in 0..11 {
                        let a = i as f32 / 11.0 * TAU;
                        self.props.push(Prop::Rock(add(pit, [0.75 * a.cos(), 0.0, 0.75 * a.sin()]), r.range(0.16, 0.24)));
                    }
                    for i in 0..4 {
                        self.props.push(Prop::Log(add(pit, [0.0, 0.1 + 0.05 * i as f32, 0.0]), i as f32 * 0.8 + 0.3));
                    }
                    self.lights.push(Light { pos: add(pit, [0.0, 0.8, 0.0]), colour: [7.5, 3.1, 0.9], flicker: 1.0, flame: 1.4 });
                    self.sources.push((Sfx::Fire, add(pit, [0.0, 0.6, 0.0]), 2.2));
                }
                _ => {
                    let g = [snap(centre[0] - 1.5), s.hi[1], snap(centre[2] + 1.0)];
                    self.grate = Box3 { lo: [g[0] - 0.8, g[1] - 0.1, g[2] - 0.8], hi: [g[0] + 0.8, g[1], g[2] + 0.8] };
                    let cage = [centre[0] + 2.2, s.hi[1], centre[2] - 1.8];
                    self.props.push(Prop::Cage(cage));
                    self.sources.push((Sfx::Creak, add(cage, [0.0, -1.6, 0.0]), 0.9));
                    for i in 0..5 {
                        let p = [centre[0] + r.range(-3.5, 3.5), s.hi[1], s.lo[2] + 1.0 + i as f32 * 1.7];
                        self.props.push(Prop::Chain(p, r.range(1.0, 2.6)));
                    }
                    for i in 0..3 {
                        self.drips.push(([centre[0] - 3.0 + i as f32 * 2.7, s.hi[1], s.lo[2] + 1.5 + i as f32], 0.0));
                    }
                }
            }
        }
        let bats = corridors[1].0.centre();
        self.sources.push((Sfx::Bats, bats, 0.7));
        self.path_through(chambers, corridors);
    }

    /// The autopilot tour: from the first room through every corridor to the last room.
    fn path_through(&mut self, chambers: &[usize], corridors: &[(Box3, usize, f32)]) {
        let a = self.zones[chambers[0]].space;
        let mut path = vec![[a.lo[0] + 2.5, 0.0, a.centre()[2] + 1.0]];
        for (i, &(c, axis, at)) in corridors.iter().enumerate() {
            let mut ends = [[0.0; 3]; 2];
            for (k, end) in ends.iter_mut().enumerate() {
                end[axis] = if k == 0 { c.lo[axis] - 1.4 } else { c.hi[axis] + 1.4 };
                end[2 - axis] = at;
            }
            let forward = (self.zones[chambers[i + 1]].space.centre()[axis] > c.centre()[axis]) as usize;
            path.push(ends[1 - forward]);
            path.push(ends[forward]);
            let next = self.zones[chambers[i + 1]].space;
            if chambers[i + 1] == self.cave {
                let x = self.bridge.centre()[0];
                path.push([x - 1.5, 0.0, self.bridge.lo[2] - 0.8]);
                path.push([x, 0.0, self.bridge.lo[2] + 0.2]);
                path.push([x, 0.0, self.bridge.hi[2] - 0.2]);
            } else if i + 2 < chambers.len() || i == corridors.len() - 1 {
                path.push(add(next.centre(), [0.0, -next.centre()[1], 0.0]));
            }
        }
        self.path = path;
    }

    /// `to` moved back inside the walkable area, sliding along the edge it left.
    pub fn clamp_walk(&self, from: V3, to: V3) -> V3 {
        let ok = |p: V3| self.walk.iter().any(|w| p[0] >= w.lo[0] && p[0] <= w.hi[0] && p[2] >= w.lo[2] && p[2] <= w.hi[2]);
        [to, [to[0], to[1], from[2]], [from[0], to[1], to[2]], from].into_iter().find(|&p| ok(p)).unwrap_or(from)
    }
}

#[derive(Clone, Copy, Default)]
struct Vertex {
    pos: V3,
    normal: V3,
    uv: [f32; 2],
    tint: V3,
    material: f32,
}

const UP: V3 = [0.0, 1.0, 0.0];
const STONE: V3 = [1.0, 1.0, 1.0];
const BONE: V3 = [1.25, 1.1, 0.85];
const IRON: V3 = [0.35, 0.33, 0.33];
const WOOD: V3 = [0.42, 0.27, 0.15];
const WAX: V3 = [1.5, 1.35, 1.0];
const PUMPKIN: V3 = [2.2, 0.9, 0.12];
const LIMESTONE: V3 = [1.25, 1.15, 0.95];

#[derive(Default)]
struct Builder {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
 }

impl Builder {
    fn grid(&mut self, f: &dyn Fn(f32, f32) -> V3, cells: [usize; 2], uv: &dyn Fn(f32, f32) -> [f32; 2], tint: V3, prop: bool) {
        let first = self.vertices.len() as u32;
        for j in 0..=cells[1] { for i in 0..=cells[0] {
            let [s,t]=[i as f32/cells[0] as f32,j as f32/cells[1] as f32];
            let (normal,_) = frame(f,s,t);
            let material=if prop { if tint==IRON {2.} else if tint==LIMESTONE || tint==STONE {1.} else {3.} } else if normal[1]>0.8 && f(s,t)[1]>=-0.1 {0.} else {1.};
            let pos=f(s,t);
            let tex=if prop { if normal[1].abs()>0.6 {[pos[0]*0.35,pos[2]*0.35]} else {[pos[0]*0.35+pos[2]*0.35,pos[1]*0.35]} } else {uv(s,t)};
            self.vertices.push(Vertex{pos,normal,uv:tex,tint,material});
        }}
        let row=cells[0] as u32+1;
        for j in 0..cells[1] as u32 { for i in 0..cells[0] as u32 {
            let a=first+j*row+i;self.indices.extend([a,a+1,a+row,a+1,a+row+1,a+row]);
        }}
    }
    fn surface(&mut self,f:&dyn Fn(f32,f32)->V3,_size:[f32;2],cells:[usize;2],uv:&dyn Fn(f32,f32)->[f32;2],tint:V3) {
        self.grid(f,cells,uv,tint,false);
    }
    fn prop(&mut self,f:&dyn Fn(f32,f32)->V3,cells:[usize;2],_centre:V3,_radius:f32,tint:V3) {
        self.grid(f,cells,&|s,t|[s,t],tint,true);
    }

    /// An ellipsoid of `radii` turned by `yaw`, its radius scaled by `shape`(longitude, latitude).
    fn blob(&mut self, centre: V3, radii: V3, yaw: f32, shape: &dyn Fn(f32, f32) -> f32, cells: [usize; 2], tint: V3) {
        let (sin, cos) = yaw.sin_cos();
        let f = |s: f32, t: f32| {
            let (a, b) = (s * TAU, (0.5 - t) * PI);
            let k = shape(a, b);
            let d = [b.cos() * a.cos() * radii[0] * k, b.sin() * radii[1] * k, b.cos() * a.sin() * radii[2] * k];
            add(centre, [d[0] * cos + d[2] * sin, d[1], d[2] * cos - d[0] * sin])
        };
        self.prop(&f, cells, centre, radii.into_iter().fold(0.0, f32::max), tint);
    }

    /// A tube from `a` to `b` whose radius follows `radius`(t).
    fn tube(&mut self, a: V3, b: V3, radius: &dyn Fn(f32) -> f32, segments: usize, tint: V3) {
        let axis = unit(sub(b, a));
        let side = unit(cross(axis, if axis[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { UP }));
        let other = cross(axis, side);
        // The first and last rings close the ends.
        let f = |s: f32, t: f32| {
            let (sin, cos) = (s * TAU).sin_cos();
            let along = ((t - 1.0 / 6.0) * 1.5).clamp(0.0, 1.0);
            let r = radius(along) * (t * 6.0).min((1.0 - t) * 6.0).min(1.0);
            add(mix3(a, b, along), add(scale(side, cos * r), scale(other, sin * r)))
        };
        self.prop(&f, [segments, 6], mix3(a, b, 0.5), (length(sub(b, a)) * 0.5).min(0.25), tint);
    }
}

/// The surface normal and tangent of `f` at (s, t) by central differences.
fn frame(f: &dyn Fn(f32, f32) -> V3, s: f32, t: f32) -> (V3, V3) {
    let e = 1e-3;
    let ds = sub(f((s + e).min(1.0), t), f((s - e).max(0.0), t));
    let dt = sub(f(s, (t + e).min(1.0)), f(s, (t - e).max(0.0)));
    let n = cross(ds, dt);
    if length(n) < 1e-12 {
        let centre = f(0.5, 0.5);
        return (unit(sub(f(s, t), centre)), unit(if length(ds) > 0.0 { ds } else { [1.0, 0.0, 0.0] }));
    }
    (unit(n), unit(if length(ds) > 1e-12 { ds } else { cross(dt, n) }))
}

impl World {
    /// Every wall, floor and ceiling, the river bed and the props.
    fn build(&self, b: &mut Builder) {
        for (z, zone) in self.zones.iter().enumerate() {
            let s = zone.space;
            let cave = zone.kind == Kind::Cave;
            let inward = |p: V3, n: V3| if cave { add(p, scale(n, self.rock(p))) } else { p };
            for wall in 0..4 {
                let (axis, side) = (if wall < 2 { 0 } else { 2 }, wall % 2);
                let k = 2 - axis;
                let mut n = [0.0; 3];
                n[axis] = if side == 1 { -1.0 } else { 1.0 };
                let e = cross(UP, n);
                let start = if e[k] > 0.0 { s.lo[k] } else { s.hi[k] };
                let length = s.hi[k] - s.lo[k];
                if zone.kind == Kind::Corridor && axis != 2 - self.corridor_axis(z) {
                    continue;
                }
                let mut holes: Vec<[f32; 3]> = self.doors.iter().filter(|d| d.zone == z && d.axis == axis && d.side == side)
                    .map(|d| { let [a, c] = d.span.map(|c| (c - start) * e[k]); [a.min(c), a.max(c), d.top] }).collect();
                holes.sort_by(|a, c| a[0].total_cmp(&c[0]));
                let mut pieces = Vec::new();
                let mut at = 0.0;
                for &[a, c, top] in &holes {
                    pieces.push([at, a, s.lo[1], s.hi[1]]);
                    pieces.push([a, c, top, s.hi[1]]);
                    at = c;
                }
                pieces.push([at, length, s.lo[1], s.hi[1]]);
                let mut plane = [0.0; 3];
                plane[axis] = if side == 1 { s.hi[axis] } else { s.lo[axis] };
                plane[k] = start;
                for [a, c, y0, y1] in pieces.into_iter().filter(|p| p[1] - p[0] > 1e-3 && p[3] - p[2] > 1e-3) {
                    let f = |u: f32, v: f32| inward(add(add(plane, scale(e, a + (c - a) * u)), [0.0, y0 + (y1 - y0) * v, 0.0]), n);
                    let uv = |u: f32, v: f32| [(a + (c - a) * u) / 4.0 + start * e[k] / 4.0, (y0 + (y1 - y0) * v) / 4.0];
                    let cells = if cave { [((c - a) * 2.0) as usize, ((y1 - y0) * 2.0) as usize].map(|c| c.max(1)) } else { [1, 1] };
                    b.surface(&f, [c - a, y1 - y0], cells, &uv, if cave { LIMESTONE } else { STONE });
                }
            }
            let [w, d] = [s.hi[0] - s.lo[0], s.hi[2] - s.lo[2]];
            let ceiling = |x: f32, zz: f32| if cave { self.ceiling(x, zz) } else { s.hi[1] };
            let cells = if cave { [(d * 2.0) as usize, (w * 2.0) as usize] } else { [1, 1] };
            let f = |u: f32, v: f32| [s.lo[0] + w * u, ceiling(s.lo[0] + w * u, s.lo[2] + d * v), s.lo[2] + d * v];
            b.surface(&f, [w, d], [cells[1], cells[0]], &|u, v| [(s.lo[0] + w * u) / 4.0, (s.lo[2] + d * v) / 4.0], if cave { LIMESTONE } else { STONE });
            let floors: Vec<[f32; 3]> = if cave {
                vec![[s.lo[2], self.river.lo[2], 0.0], [self.river.lo[2], self.river.hi[2], BED], [self.river.hi[2], s.hi[2], 0.0]]
            } else {
                vec![[s.lo[2], s.hi[2], s.lo[1]]]
            };
            for [z0, z1, y] in floors {
                let f = |u: f32, v: f32| [s.lo[0] + w * v, y, z0 + (z1 - z0) * u];
                b.surface(&f, [z1 - z0, w], [1, 1], &|u, v| [(z0 + (z1 - z0) * u) / 4.0 + z as f32 * 0.37, (s.lo[0] + w * v) / 4.0], if y < 0.0 { [0.5, 0.55, 0.55] } else if cave { LIMESTONE } else { STONE });
            }
            if cave {
                for (zz, sign) in [(self.river.lo[2], 1.0), (self.river.hi[2], -1.0)] {
                    let f = |u: f32, v: f32| { let x = if sign > 0.0 { s.lo[0] + w * u } else { s.hi[0] - w * u }; [x, BED - (BED * v), zz] };
                    b.surface(&f, [w, -BED], [1, 1], &|u, v| [u * w / 4.0, (BED - BED * v) / 4.0], LIMESTONE);
                }
            }
        }
        for door in &self.doors {
            let zone=&self.zones[door.zone];if zone.kind==Kind::Cave {continue;}
            let k=2-door.axis;let plane=if door.side==0{zone.space.lo[door.axis]}else{zone.space.hi[door.axis]};
            let centre=(door.span[0]+door.span[1])*0.5;let radius=(door.span[1]-door.span[0])*0.5+0.17;
            for i in 0..9 {
                let angle=(i as f32+0.5)/9.*PI;
                let mut p=[0.;3];p[door.axis]=plane;p[k]=centre+radius*angle.cos();p[1]=door.top-0.8+radius*angle.sin();
                let mut lo=sub(p,[0.22,0.19,0.22]);let mut hi=add(p,[0.22,0.19,0.22]);lo[door.axis]-=0.09;hi[door.axis]+=0.09;
                b.block(lo,hi,LIMESTONE);
            }
            for side in [-1.,1.] {for i in 0..7 {
                let mut p=[0.;3];p[door.axis]=plane;p[k]=centre+side*radius;p[1]=(i as f32+0.5)*0.32;
                b.block(sub(p,[0.23,0.15,0.23]),add(p,[0.23,0.15,0.23]),LIMESTONE);
            }}
        }
        self.bridge_and_props(b);
    }

    fn corridor_axis(&self, z: usize) -> usize {
        let s = self.zones[z].space;
        if s.hi[0] - s.lo[0] > s.hi[2] - s.lo[2] { 0 } else { 2 }
    }
}

fn turn(yaw: f32, v: V3) -> V3 {
    let (sin, cos) = yaw.sin_cos();
    [v[0] * cos + v[2] * sin, v[1], v[2] * cos - v[0] * sin]
}

impl Builder {
    /// The six faces of a stone block, each with its lightmap island.
    fn block(&mut self, lo: V3, hi: V3, tint: V3) {
        for i in 0..3 {
            for side in 0..2 {
                let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                let (u, v) = if side == 1 { (j, k) } else { (k, j) };
                let f = |a: f32, c: f32| {
                    let mut p = [0.0; 3];
                    p[i] = if side == 1 { hi[i] } else { lo[i] };
                    p[u] = lo[u] + (hi[u] - lo[u]) * a;
                    p[v] = lo[v] + (hi[v] - lo[v]) * c;
                    p
                };
                let size = [hi[u] - lo[u], hi[v] - lo[v]];
                self.surface(&f, size, [1, 1], &|a, c| [(lo[u] + size[0] * a) / 4.0, (lo[v] + size[1] * c) / 4.0], tint);
            }
        }
    }

    fn skull(&mut self, p: V3, yaw: f32) {
        let radii = [0.1, 0.095, 0.125];
        let centre = add(p, [0.0, 0.085, 0.0]);
        let socket = |a: f32, b: f32| (-((a.abs() - 0.5).powi(2) + (b + 0.05).powi(2)) / 0.03).exp();
        let shape = |a: f32, b: f32| {
            let front = a.cos().max(0.0);
            (1.0 - 0.22 * socket(if a > PI { a - TAU } else { a }, b) * front) * if b < -0.25 { 1.0 - 0.25 * front } else { 1.0 }
        };
        self.blob(centre, radii, yaw, &shape, [16, 12], BONE);
        for side in [-0.5f32, 0.5] {
            let eye = [side.cos() * radii[0] * 0.78, -0.01, side.sin() * radii[2] * 0.78];
            self.blob(add(centre, turn(yaw, eye)), [0.028; 3], 0.0, &|_, _| 1.0, [6, 4], [0.0; 3]);
        }
    }

    fn chain(&mut self, top: V3, length: f32) {
        for i in 0..(length / 0.085) as usize {
            let radii = if i % 2 == 0 { [0.012, 0.05, 0.032] } else { [0.032, 0.05, 0.012] };
            self.blob(add(top, [0.0, -0.085 * i as f32 - 0.04, 0.0]), radii, 0.0, &|_, _| 1.0, [6, 4], IRON);
        }
    }
}

impl World {
    fn bridge_and_props(&self, b: &mut Builder) {
        let g = self.bridge;
        let mid = g.centre();
        b.block([g.lo[0], -0.35, g.lo[2]], [g.hi[0], 0.0, g.hi[2]], LIMESTONE);
        b.block([mid[0] - 0.35, BED, self.river.centre()[2] - 0.35], [mid[0] + 0.35, -0.35, self.river.centre()[2] + 0.35], LIMESTONE);
        for x in [g.lo[0] + 0.08, g.hi[0] - 0.08] {
            let steps = ((g.hi[2] - g.lo[2]) / 1.0).round() as usize;
            for i in 0..=steps {
                let z = g.lo[2] + (g.hi[2] - g.lo[2]) * i as f32 / steps as f32;
                b.tube([x, 0.0, z], [x, 1.05, z], &|_| 0.03, 6, IRON);
            }
            for y in [0.5, 1.0] {
                b.tube([x, y, g.lo[2]], [x, y, g.hi[2]], &|_| 0.02, 6, IRON);
            }
        }
        for x in [g.lo[0] - 0.1, g.hi[0] + 0.1] {
            b.tube([x, 0.0, g.lo[2] + 0.2], [x, 1.45, g.lo[2] + 0.2], &|t| 0.06 - 0.02 * t, 8, WOOD);
        }
        if self.grate.hi[1] > 0.0 {
            let r = self.grate;
            for i in 0..7 {
                let t = i as f32 / 6.0;
                let [x, z] = [r.lo[0] + (r.hi[0] - r.lo[0]) * t, r.lo[2] + (r.hi[2] - r.lo[2]) * t];
                b.tube([x, r.lo[1], r.lo[2]], [x, r.lo[1], r.hi[2]], &|_| 0.03, 6, IRON);
                b.tube([r.lo[0], r.lo[1] - 0.03, z], [r.hi[0], r.lo[1] - 0.03, z], &|_| 0.03, 6, IRON);
            }
        }
        let mut r = Rng(77);
        for prop in &self.props {
            match *prop {
                Prop::Skull(p, yaw) => b.skull(p, yaw),
                Prop::Bone(p, yaw) => {
                    let half = turn(yaw, [0.17, 0.0, 0.0]);
                    let knob = |t: f32| 0.016 + 0.02 * ((-(t / 0.09).powi(2)).exp() + (-((1.0 - t) / 0.09).powi(2)).exp());
                    b.tube(add(sub(p, half), [0.0, 0.03, 0.0]), add(add(p, half), [0.0, 0.03, 0.0]), &knob, 7, BONE);
                }
                Prop::Candle(p, h) => b.tube(p, add(p, [0.0, h, 0.0]), &|t| 0.034 + 0.008 * (1.0 - t), 8, WAX),
                Prop::Lantern(p, yaw) => {
                    let ribs = |a: f32, _: f32| 1.0 - 0.07 * (4.0 * a).sin().abs();
                    b.blob(add(p, [0.0, 0.19, 0.0]), [0.24, 0.19, 0.24], yaw, &ribs, [24, 12], PUMPKIN);
                    b.tube(add(p, [0.0, 0.35, 0.0]), add(p, [0.03, 0.45, 0.0]), &|_| 0.025, 6, WOOD);
                }
                Prop::Rock(p, size) => {
                    let seed = r.next() * 50.0;
                    let lumpy = |a: f32, c: f32| 0.8 + 0.4 * noise(a * 1.3 + seed, c * 2.0 + seed, 64, 3);
                    b.blob(add(p, [0.0, size * 0.45, 0.0]), [size, size * 0.7, size * 0.9], seed, &lumpy, [24, 16], LIMESTONE);
                }
                Prop::Log(p, yaw) => {
                    let half = turn(yaw, [0.55, 0.0, 0.0]);
                    b.tube(sub(p, half), add(p, half), &|_| 0.075, 8, [0.22, 0.13, 0.07]);
                }
                Prop::Chain(top, length) => b.chain(top, length),
                Prop::Cage(top) => {
                    b.chain(top, 1.3);
                    let (bottom, height, radius) = (add(top, [0.0, -2.6, 0.0]), 1.2, 0.45);
                    for i in 0..12 {
                        let a = i as f32 / 12.0 * TAU;
                        let ring = |y: f32, a: f32| add(bottom, [radius * a.cos(), y, radius * a.sin()]);
                        b.tube(ring(0.0, a), add(ring(height, a), scale([a.cos(), 0.0, a.sin()], -0.15)), &|_| 0.014, 5, IRON);
                        for y in [0.0, height * 0.55] {
                            b.tube(ring(y, a), ring(y, a + TAU / 12.0), &|_| 0.016, 5, IRON);
                        }
                    }
                    b.blob(bottom, [radius, 0.02, radius], 0.0, &|_, _| 1.0, [12, 4], IRON);
                    b.skull(add(bottom, [0.05, 0.0, 0.1]), 2.0);
                }
                Prop::Stalactite(..) | Prop::Stalagmite(..) => {}
                Prop::Bracket(p, n) => {
                    b.tube(add(sub(p, scale(n, 0.27)), [0.0, -0.32, 0.0]), add(p, [0.0, -0.2, 0.0]), &|_| 0.018, 5, IRON);
                    b.tube(add(p, [0.0, -0.42, 0.0]), add(p, [0.0, -0.02, 0.0]), &|t| 0.03 + 0.012 * t, 7, WOOD);
                    b.blob(p, [0.06, 0.035, 0.06], 0.0, &|_, _| 1.0, [8, 4], IRON);
                }
                Prop::Web(..) => {}
            }
        }
    }
}

fn floats(values: &[f32]) -> Vec<u8> { values.iter().flat_map(|v| v.to_le_bytes()).collect() }

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

pub struct Dungeon {
    renderer: Option<render::Renderer>,
    world: World,
    sound: Sound,
    eye: V3,
    yaw: f32,
    pitch: f32,
    time: f32,
    idle: f32,
    waypoint: usize,
    foot_distance: f32,
    scream: bool,
    audio_time: f32,
    river: Option<River>,
    solo: u8,
}
impl Dungeon {
    pub fn new(state: &'static VkState) -> Result<Self, String> { Self::with_state(Some(state)) }
    fn with_state(state:Option<&'static VkState>)->Result<Self,String> {
        let world=World::new(8);
        let renderer=state.map(|state| {
            let mut mesh=Builder::default();world.build(&mut mesh);
            render::Renderer::new(state,&world,&mesh)
        }).transpose()?;
        let eye = add(world.path[0], [0., EYE, 0.]);
        let solo = if state.is_none() { CAPTURE_SOLO.load(Ordering::Relaxed) } else { 0 };
        let sources: Vec<_> = match solo {
            1 => vec![(Sfx::Torch, add(eye, [-1.0, 0.0, 0.0]), 1.0)],
            2 => world.sources.iter().copied().filter(|source| source.0 == Sfx::River).collect(),
            3 | 4 => Vec::new(),
            _ => world.sources.clone(),
        };
        let sound = Sound::new(&world, &sources);
        Ok(Self { renderer, world, sound, eye, yaw: if solo == 1 { 0. } else { PI/2. }, pitch: 0., time: 0., idle: if solo == 1 { f32::NEG_INFINITY } else { 3. }, waypoint: 1, foot_distance: 0., scream: false, audio_time: 0., river: state.map(River::new).transpose()?, solo })
    }

    pub fn frame(&mut self, seconds: f32) -> Result<Vec<i16>, String> {
        use super::demo_dungeon_sound::Sample;
        if let Some(renderer)=&mut self.renderer {
            let showcase=render::SHOWCASE.load(std::sync::atomic::Ordering::Relaxed);
            let state=device().map_err(|e|format!("{e:?}"))?;
            let rays=render::RAYS.load(std::sync::atomic::Ordering::Relaxed)&&state.acceleration_structure.is_some();
            if renderer.showcase!=showcase || renderer.rays!=rays {
                let mut mesh=Builder::default();self.world.build(&mut mesh);
                self.river=Some(River::new(state)?);
                *renderer=render::Renderer::new(state,&self.world,&mesh)?;
            }
            renderer.rays=rays;
        }
        let dt = (seconds - self.time).clamp(0.0, 0.1);
        let old_eye = self.eye;
        let input = crate::input::state().0;
        let moving = input.left.into_iter().chain(input.right).any(|v| v.abs() > 0.1);
        self.idle = if moving { 0.0 } else { self.idle + dt };
        if self.idle >= 3.0 {
            let to = sub(add(self.world.path[self.waypoint], [0.0, EYE, 0.0]), self.eye);
            let d = length(to);
            if d < 0.08 { self.waypoint = (self.waypoint + 1).min(self.world.path.len()-1); }
            let speed = self.world.path.windows(2).map(|p|length(sub(p[1],p[0]))).sum::<f32>()/18.5;
            self.eye = self.world.clamp_walk(self.eye, add(self.eye, scale(unit(to), (dt*speed).min(d))));
            let fire=self.world.sources.iter().find(|s|s.0==Sfx::Fire).unwrap().1;
            let interest=if length(sub(fire,self.eye))<6. && self.world.walls_between(self.eye,fire)==0 { Some(add(fire,[0.,0.2,0.])) }
                else if self.world.zones[self.world.cave].space.contains(self.eye) { Some([self.world.bridge.centre()[0]+1.7,WATER+0.2,self.world.river.centre()[2]]) }
                else if length(sub(self.world.grate.centre(),self.eye))<5.5 { Some(add(self.world.grate.centre(),[0.,-2.,0.])) }
                else {None};
            let look=interest.map_or(to,|p|sub(p,self.eye));
            let angle = look[0].atan2(-look[2]);
            let turn = (angle - self.yaw + PI).rem_euclid(TAU) - PI;
            self.yaw += turn * (dt * 4.0).min(1.0);
            let pitch=if interest.is_some(){look[1].atan2((look[0]*look[0]+look[2]*look[2]).sqrt()).clamp(-0.4,0.3)}else{0.};
            self.pitch += (pitch-self.pitch)*(dt*2.).min(1.);
        } else {
            self.yaw += input.right[0] * dt * 2.0;
            self.pitch = (self.pitch + input.right[1] * dt * 1.3).clamp(-0.8, 0.8);
            let v = [self.yaw.cos() * input.left[0] + self.yaw.sin() * input.left[1], 0.0, self.yaw.sin() * input.left[0] - self.yaw.cos() * input.left[1]];
            self.eye = self.world.clamp_walk(self.eye, add(self.eye, scale(v, dt * 2.6)));
            self.waypoint = self.world.path.iter().enumerate().min_by(|(_, a), (_, b)| length(sub(**a, self.eye)).total_cmp(&length(sub(**b, self.eye)))).unwrap().0;
        }
        match VIEW.load(std::sync::atomic::Ordering::Relaxed) {
            1 => { let p = self.world.sources.iter().find(|s| s.0 == Sfx::Fire).unwrap().1; self.eye=add(p,[-3.2,1.1,2.8]); self.yaw=0.85; self.pitch=-0.18; }
            2 => { self.eye=[self.world.bridge.centre()[0]-3.4,EYE,self.world.bridge.lo[2]-1.2]; self.yaw=2.35; self.pitch=-0.18; }
            3 => { let p=self.world.grate.centre();self.eye=add(p,[-2.8,EYE-p[1],-3.]);self.yaw=2.4;self.pitch=0.12; }
            _ => {}
        }
        let mut impacts = Vec::new();
        self.foot_distance += length(sub(self.eye, old_eye));
        let stride = 1.15 + 0.22 * (seconds * 2.3).sin() + 0.1 * (seconds * 5.1).sin();
        if matches!(self.solo, 0 | 3) && self.foot_distance > stride {
            self.foot_distance %= stride;
            let r = self.world.river;
            if self.eye[0] > r.lo[0] && self.eye[0] < r.hi[0] && self.eye[2] > r.lo[2] - 1.0 && self.eye[2] < r.hi[2] + 1.0 {
                impacts.push(([self.eye[0], WATER, self.eye[2].clamp(r.lo[2] + 0.2, r.hi[2] - 0.2)], 0.8));
            }
            let wet = noise(self.eye[0]*0.43, self.eye[2]*0.43,64,11) > 0.43;
            let gravel = self.world.zones_at(self.eye).any(|z| self.world.zones[z].kind == Kind::Corridor);
            self.sound.play(if wet { Sample::WetStep } else if gravel { Sample::GravelStep } else { Sample::Step }, self.eye, 0.04, 1.0);
        }
        if self.solo == 0 && !self.scream && seconds >= 14.0 {
            self.sound.play(Sample::Scream, self.world.zones[self.world.last].space.centre(), 1.4, 1.0);
            self.scream = true;
        }
        let right = [self.yaw.cos(), 0.0, self.yaw.sin()];
        let forward = [self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos()];
        let up = cross(right, forward);
        if self.solo == 0 {
            let bat_source=self.world.sources.iter().position(|s|s.0==Sfx::Bats).unwrap();
            let t=seconds*1.8;
            self.sound.move_source(bat_source,add(self.world.zones[self.world.bats].space.centre(),[0.7*t.cos(),0.5+0.2*t.sin(),0.7*(t*1.2).sin()]));
        }
        if matches!(self.solo, 0 | 4) { for (i, &(tip, floor)) in self.world.drips.iter().enumerate() {
            let period = 2.4 + (i % 5) as f32 * 0.3;
            let fall = ((tip[1] - floor) / 4.9).sqrt();
            let age = (seconds + i as f32 * 0.71).rem_euclid(period);
            let previous = (self.time + i as f32 * 0.71).rem_euclid(period);
            if previous < fall && age >= fall {
                if floor == WATER { impacts.push(([tip[0], floor, tip[2]], 0.55)); }
                self.sound.play(Sample::Drip, [tip[0], floor, tip[2]], 0.5, 0.8 + i as f32 * 0.03);
            }
        } }
        let back = scale(forward, -1.0);
        let axes = [right, up, back];
        let view: Matrix = std::array::from_fn(|column| std::array::from_fn(|row| if row == 3 { if column == 3 { 1.0 } else { 0.0 } } else if column == 3 { -dot(axes[row], self.eye) } else { axes[row][column] }));
        let focal = 1.0 / (PI / 6.0).tan();
        let projection = [[focal * HEIGHT as f32 / WIDTH as f32, 0.0, 0.0, 0.0], [0.0, -focal, 0.0, 0.0], [0.0, 0.0, -100.0 / 99.9, -1.0], [0.0, 0.0, -10.0 / 99.9, 0.0]];
        if let (Some(renderer),Some(river))=(&mut self.renderer,&mut self.river) {
            let waves=river.step(self.world.river,dt,&impacts,renderer.query_pool())?;
            renderer.report("simulation")?;
            renderer.frame(&self.world,self.eye,view,projection,seconds,right,up,&waves)?;
        }
        self.sound.listen(&self.world, self.eye, self.yaw, dt);
        self.audio_time += dt;
        let periods = (self.audio_time / 0.005).floor() as usize;
        self.audio_time -= periods as f32 * 0.005;
        self.time = seconds;
        Ok(self.sound.render(periods))
    }
}

pub fn render_dungeon_frame(seconds: f32) -> Result<(), String> {
    static DUNGEON: Mutex<Option<Dungeon>> = Mutex::new(None);
    let mut slot = DUNGEON.lock().unwrap();
    if slot.is_none() { *slot = Some(Dungeon::new(device().map_err(|e| format!("{e:?}"))?)?); }
    let dungeon = slot.as_mut().unwrap();
    let audio = dungeon.frame(seconds)?;
    for period in audio.chunks(480) { crate::audio::play(&[period]); }
    Ok(())
}

pub fn render_dungeon_audio_frame(seconds:f32)->Result<(),String> {
    static AUDIO:Mutex<Option<Dungeon>>=Mutex::new(None);
    let mut slot=AUDIO.lock().unwrap();
    if slot.is_none(){*slot=Some(Dungeon::with_state(None)?);}
    let audio=slot.as_mut().unwrap().frame(seconds)?;
    for period in audio.chunks(480){crate::audio::play(&[period]);}
    Ok(())
}

fn water_grid() -> [usize; 2] { if render::SHOWCASE.load(std::sync::atomic::Ordering::Relaxed) { [256,64] } else { [128,32] } }

struct River {
    state: &'static VkState,
    pipelines: [super::compute::ComputePipeline<'static>; 2],
    _heights: [super::compute::HostBuffer<'static>; 2],
    surface: super::compute::HostBuffer<'static>,
    uniforms: super::compute::HostBuffer<'static>,
    ping: usize,
    grid: [usize; 2],
    time: f32,
}

impl River {
    fn new(state: &'static VkState) -> Result<Self, String> {
        use super::compute::{ComputePipeline, HostBuffer};
        use ash::vk::DescriptorType as D;
        let named = |e| format!("{e:?}");
        let grid=water_grid();
        let n = (grid[0] * grid[1]) as u64;
        let mut heights = [HostBuffer::new(state, n * 8).map_err(named)?, HostBuffer::new(state, n * 8).map_err(named)?];
        for h in &mut heights { h.write(&vec![0; n as usize * 8]).map_err(named)?; }
        let surface = HostBuffer::new(state, n * 16).map_err(named)?;
        let uniforms = HostBuffer::new(state, 32 + 32 * 16).map_err(named)?;
        let code: Vec<u32> = include_bytes!("../../../demos/shaders/dungeon_water.spv").chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
        let bindings = [(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::STORAGE_BUFFER), (3, D::UNIFORM_BUFFER)];
        let mut pipelines = [ComputePipeline::new(state, &code, &bindings).map_err(named)?, ComputePipeline::new(state, &code, &bindings).map_err(named)?];
        for i in 0..2 { pipelines[i].descriptors.bind(&[(0, &heights[i]), (1, &heights[1 - i]), (2, &surface), (3, &uniforms)]); }
        Ok(Self { state, pipelines, _heights: heights, surface, uniforms, ping: 0, grid, time: 0. })
    }

    fn step(&mut self, river: Box3, dt: f32, impacts: &[(V3, f32)], timestamp:Option<ash::vk::QueryPool>) -> Result<Vec<[f32; 4]>, String> {
        let named = |e| format!("{e:?}");
        let grid=self.grid;self.time+=dt;
        let size = sub(river.hi, river.lo);
        let mut u = vec![grid[0] as f32, grid[1] as f32, dt.min(1.0 / 30.0), 2.0, size[0], size[2], impacts.len().min(32) as f32, self.time];
        for &(p, strength) in impacts.iter().take(32) {
            u.extend([(p[0] - river.lo[0]) / size[0] * grid[0] as f32, (p[2] - river.lo[2]) / size[2] * grid[1] as f32, strength, 2.0]);
        }
        self.uniforms.write(&floats(&u)).map_err(named)?;
        self.state.dispatch_profiled(&self.pipelines[self.ping], [grid[0] as u32 / 8, grid[1] as u32 / 8, 1], timestamp).map_err(named)?;
        self.ping ^= 1;
        let mut out = vec![0; grid[0] * grid[1] * 16];
        self.surface.read(&mut out).map_err(named)?;
        Ok(out.chunks_exact(16).map(|b| std::array::from_fn(|i| f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap()))).collect())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drips_do_not_blanket_the_river_with_foam() {
        let world = World::new(8);
        let mut river = River::new(device().unwrap()).unwrap();
        let mut foam = 0.;
        for frame in 0..1200 {
            let impacts = [(world.river.centre(), 0.55)];
            let waves = river.step(world.river, 1. / 60., if frame % 20 == 0 { &impacts } else { &[] }, None).unwrap();
            foam = waves.iter().filter(|p| p[3] > 0.5).count() as f32 / waves.len() as f32;
        }
        assert!(foam < 0.05, "{foam}");
    }
    #[test]
    fn autopilot_reaches_every_chamber_in_twenty_seconds() {
        let world=World::new(8);
        let mut at=add(world.path[0],[0.,EYE,0.]);let mut waypoint=1;
        let speed=world.path.windows(2).map(|p|length(sub(p[1],p[0]))).sum::<f32>()/18.5;
        let mut seen=vec![false;world.zones.len()];
        for _ in 0..1200 {
            for z in world.zones_at(at){seen[z]=true;}
            let to=sub(add(world.path[waypoint],[0.,EYE,0.]),at);let d=length(to);
            if d<0.08 {waypoint=(waypoint+1).min(world.path.len()-1);}
            at=world.clamp_walk(at,add(at,scale(unit(to),(speed/60.).min(d))));
        }
        for (z,zone) in world.zones.iter().enumerate() {if zone.kind!=Kind::Corridor {assert!(seen[z],"unvisited chamber {z}, waypoint {waypoint}, position {at:?}");}}
    }
}
