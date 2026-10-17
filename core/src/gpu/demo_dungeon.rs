//! The dungeon demo: a generated layout of stone rooms, corridors and a river cave, walked through in first person and
//! drawn through the pbr scene path in four draws: the stone and props with their textures, a lightmap pass that
//! multiplies in GPU-composed torchlight, flicker and distance darkness, the river surface and
//! the additive flames, embers and glows. The sound runs through the emulated audio renderer (demo_dungeon_sound.rs).
use super::demo::{Matrix, Scene, multiply};
use super::texture::{level_gob_height_log2, pitch_to_block_linear};
use super::{VkState, device};
use super::demo_dungeon_sound::Sound;
use std::f32::consts::{PI, TAU};
use std::sync::Mutex;

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
                        self.drips.push(([p[0], top - length, p[2]], h));
                    } else if r.next() < 0.5 {
                        self.drips.push(([p[0], top - length, p[2]], land));
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

    /// The autopilot tour: from the first room through every corridor to the last room and back.
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
        let back: Vec<V3> = path[1..path.len() - 1].iter().rev().copied().collect();
        path.extend(back);
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
    tangent: V3,
    uv: [f32; 2],
    light: [f32; 2],
}

/// A lightmap texel to bake: where it sits, which way it faces and the colour its surface takes.
struct Texel {
    at: usize,
    pos: V3,
    normal: V3,
    tint: V3,
}

pub const LIGHTMAP: [usize; 2] = [1024, 512];
const DENSITY: f32 = 9.0;
const OCTA: usize = 8;
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
    texels: Vec<Texel>,
    /// shelf packer: x, y and height of the current row.
    shelf: [usize; 3],
}

impl Builder {
    fn island(&mut self, w: usize, h: usize) -> [usize; 2] {
        let [x, y, row] = &mut self.shelf;
        if *x + w > LIGHTMAP[0] {
            (*x, *y, *row) = (0, *y + *row, 0);
        }
        assert!(*y + h <= LIGHTMAP[1], "the dungeon's lightmap is full");
        let at = [*x, *y];
        *x += w;
        *row = (*row).max(h);
        at
    }

    /// A `cells` grid over `f`(s, t) for s, t in 0..1, its normals from the differences of `f`, with the stone uv
    /// `uv`(s, t) and the lightmap uv `light`(s, t, normal).
    fn grid(&mut self, f: &dyn Fn(f32, f32) -> V3, cells: [usize; 2], uv: &dyn Fn(f32, f32) -> [f32; 2], light: &mut dyn FnMut(f32, f32, V3) -> [f32; 2]) {
        let first = self.vertices.len() as u32;
        for j in 0..=cells[1] {
            for i in 0..=cells[0] {
                let [s, t] = [i as f32 / cells[0] as f32, j as f32 / cells[1] as f32];
                let (normal, tangent) = frame(f, s, t);
                self.vertices.push(Vertex { pos: f(s, t), normal, tangent, uv: uv(s, t), light: light(s, t, normal) });
            }
        }
        let row = cells[0] as u32 + 1;
        for j in 0..cells[1] as u32 {
            for i in 0..cells[0] as u32 {
                let a = first + j * row + i;
                self.indices.extend([a, a + 1, a + row, a + 1, a + row + 1, a + row]);
            }
        }
    }

    /// An architectural surface with its own lightmap island at the lightmap density.
    fn surface(&mut self, f: &dyn Fn(f32, f32) -> V3, size: [f32; 2], cells: [usize; 2], uv: &dyn Fn(f32, f32) -> [f32; 2], tint: V3) {
        let texels = size.map(|s| (s * DENSITY).ceil().max(1.0) as usize + 2);
        let at = self.island(texels[0], texels[1]);
        for b in 0..texels[1] {
            for a in 0..texels[0] {
                let [s, t] = [(a, 0), (b, 1)].map(|(k, i)| ((k as f32 - 0.5) / (texels[i] - 2) as f32).clamp(0.0, 1.0));
                let (normal, _) = frame(f, s, t);
                self.texels.push(Texel { at: (at[1] + b) * LIGHTMAP[0] + at[0] + a, pos: f(s, t), normal, tint });
            }
        }
        let corner = |k: usize, s: f32| (at[k] as f32 + 1.0 + s * (texels[k] - 2) as f32) / LIGHTMAP[k] as f32;
        self.grid(f, cells, uv, &mut |s, t, _| [corner(0, s), corner(1, t)]);
    }

    /// A prop lit by an octahedral island: every texel is the light at `centre` for one normal direction.
    fn prop(&mut self, f: &dyn Fn(f32, f32) -> V3, cells: [usize; 2], centre: V3, radius: f32, tint: V3) {
        let at = self.island(OCTA, OCTA);
        for b in 0..OCTA {
            for a in 0..OCTA {
                let [x, y] = [a, b].map(|k| ((k as f32 - 0.5) / (OCTA - 2) as f32).clamp(0.0, 1.0) * 2.0 - 1.0);
                let normal = octahedral(x, y);
                self.texels.push(Texel { at: (at[1] + b) * LIGHTMAP[0] + at[0] + a, pos: add(centre, scale(normal, radius)), normal, tint });
            }
        }
        let uv = |s: f32, t: f32| {
            let p = f(s, t);
            let _ = p;
            [0.15, 0.06]
        };
        self.grid(f, cells, &uv, &mut |_, _, n| {
            let n = scale(n, 1.0 / (n[0].abs() + n[1].abs() + n[2].abs()));
            let [x, y] = if n[1] >= 0.0 { [n[0], n[2]] } else { [(1.0 - n[2].abs()) * n[0].signum(), (1.0 - n[0].abs()) * n[2].signum()] };
            [0, 1].map(|k| (at[k] as f32 + 1.0 + ([x, y][k] * 0.5 + 0.5) * (OCTA - 2) as f32) / LIGHTMAP[k] as f32)
        });
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

fn octahedral(x: f32, y: f32) -> V3 {
    let up = 1.0 - x.abs() - y.abs();
    let [x, y] = if up >= 0.0 { [x, y] } else { [(1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum()] };
    unit([x, up, y])
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
                    b.blob(add(p, [0.0, size * 0.45, 0.0]), [size, size * 0.7, size * 0.9], seed, &lumpy, [10, 7], [0.8, 0.75, 0.7]);
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
                Prop::Stalactite(top, length, radius) => {
                    let wobble = r.next() * 9.0;
                    b.tube(top, add(top, [0.0, -length, 0.0]), &|t| radius * (1.0 - t).powf(1.6) * (0.9 + 0.2 * (t * 7.0 + wobble).sin()) + 0.006, 8, LIMESTONE);
                }
                Prop::Stalagmite(base, height, radius) => {
                    b.tube(add(base, [0.0, -0.1, 0.0]), add(base, [0.0, height, 0.0]), &|t| radius * (1.0 - t).powf(1.1) + 0.02, 9, LIMESTONE);
                }
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

pub const STONE_SIZE: usize = 2048;

fn srgb(c: f32) -> u8 {
    (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

/// Runs `f`(x, y) for every texel of a `width` x `height` image on all cores.
fn paint(width: usize, height: usize, f: &(dyn Fn(usize, usize) -> [u8; 4] + Sync)) -> Vec<[u8; 4]> {
    let mut out = vec![[0; 4]; width * height];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows = height.div_ceil(threads);
    std::thread::scope(|scope| {
        for (chunk, part) in out.chunks_mut(rows * width).enumerate() {
            scope.spawn(move || part.iter_mut().enumerate().for_each(|(i, texel)| *texel = f(i % width, chunk * rows + i / width)));
        }
    });
    out
}

/// The stone tile, four metres across: albedo, normal and roughness of irregular blocks with chipped bevels, cracks,
/// grit, moss and dirt in the joints and wet patches and streaks.
pub struct Stone {
    pub albedo: Vec<[u8; 4]>,
    pub normal: Vec<[u8; 4]>,
    pub rough: Vec<[u8; 4]>,
}

impl Stone {
    pub fn new() -> Stone {
        const N: usize = STONE_SIZE;
        const ROW: f32 = N as f32 / 8.0;
        let mut r = Rng(4242);
        let rows: Vec<(f32, Vec<f32>)> = (0..8)
            .map(|_| {
                let mut cuts = vec![0.0];
                while *cuts.last().unwrap() < N as f32 - 420.0 {
                    cuts.push(cuts.last().unwrap() + r.range(330.0, 640.0));
                }
                let fit = N as f32 / cuts.last().unwrap().max(1.0);
                cuts.iter_mut().for_each(|c| *c *= fit);
                cuts.push(N as f32);
                (r.range(0.0, N as f32), cuts)
            })
            .collect();
        // per texel: height, block id and joint distance.
        let field = |x: usize, y: usize| -> (f32, f32, f32) {
            let (fx, fy) = (x as f32, y as f32);
            let warp = [fbm(fx / 128.0, fy / 128.0, 16, 3, 1) - 0.5, fbm(fx / 128.0, fy / 128.0, 16, 3, 2) - 0.5];
            let (wx, wy) = (fx + warp[0] * 36.0, (fy + warp[1] * 30.0).rem_euclid(N as f32));
            let row = (wy / ROW) as usize % 8;
            let (shift, cuts) = &rows[row];
            let along = (wx + shift).rem_euclid(N as f32);
            let block = cuts.windows(2).position(|c| along < c[1]).unwrap_or(0);
            let edge = (along - cuts[block]).min(cuts[block + 1] - along).min(wy - row as f32 * ROW).min((row + 1) as f32 * ROW - wy);
            let id = hash(row as i32, block as i32, 7);
            let joint = 7.0 + 6.0 * noise(fx / 32.0, fy / 32.0, 64, 3);
            let detail = fbm(fx / 64.0, fy / 64.0, 32, 5, 4);
            let chips = smooth(0.62, 0.8, fbm(fx / 16.0, fy / 16.0, 128, 2, 5)) * smooth(joint + 40.0, joint, edge);
            let ridge = (fbm(fx / 128.0 + (id * 9.0).floor(), fy / 128.0, 16, 4, 6) - 0.5).abs();
            let crack = if id > 0.45 { smooth(0.014, 0.0, ridge) } else { 0.0 };
            let grit = noise(fx / 2.0, fy / 2.0, 1024, 8) - 0.5;
            let stone = 0.55 + 0.4 * smooth(joint, joint + 34.0, edge) + 0.22 * (detail - 0.5) - 0.3 * chips - 0.4 * crack + 0.05 * grit;
            let height = if edge < joint { 0.08 + 0.06 * grit + 0.05 * detail } else { stone };
            (height, id, edge - joint)
        };
        let heights: Vec<(f32, f32, f32)> = {
            let packed = paint(N, N, &|x, y| {
                let (h, id, j) = field(x, y);
                [(h.clamp(0.0, 1.0) * 255.0) as u8, (id * 255.0) as u8, (j.clamp(-20.0, 235.0) + 20.0) as u8, 0]
            });
            packed.iter().map(|t| (t[0] as f32 / 255.0, t[1] as f32 / 255.0, t[2] as f32 - 20.0)).collect()
        };
        let at = |x: isize, y: isize| heights[(y.rem_euclid(N as isize) as usize) * N + x.rem_euclid(N as isize) as usize].0;
        let wet = |x: usize, y: usize| {
            let (fx, fy) = (x as f32, y as f32);
            let puddle = smooth(0.58, 0.7, fbm(fx / 256.0, fy / 256.0, 8, 3, 11));
            let streak = smooth(0.62, 0.85, noise_xy(fx / 32.0, fy / 512.0, [64, 4], 12)) * smooth(0.3, 0.7, noise(fx / 256.0, fy / 256.0, 8, 13));
            puddle.max(streak)
        };
        let albedo = paint(N, N, &|x, y| {
            let (h, id, joint) = heights[y * N + x];
            let (fx, fy) = (x as f32, y as f32);
            let tone = mix3([0.22, 0.25, 0.26], [0.28, 0.26, 0.23], hash(id.to_bits() as i32, 0, 3));
            let mottle = 0.75 + 0.5 * fbm(fx / 128.0, fy / 128.0, 16, 4, 14);
            let speck = 0.55 + 0.9 * hash(x as i32, y as i32, 15);
            let mut c = scale(tone, mottle * speck * (0.85 + 0.3 * hash(id.to_bits() as i32, 1, 3)));
            let crevice = smooth(0.45, 0.1, h);
            let moss = smooth(0.45, 0.75, fbm(fx / 256.0, fy / 256.0, 8, 4, 16)) * crevice;
            c = mix3(c, [0.05, 0.045, 0.04], (crevice * 0.8).min(1.0) * if joint < 0.0 { 1.0 } else { 0.6 });
            c = mix3(c, [0.1, 0.16, 0.05], moss);
            c = scale(c, 1.0 - 0.45 * wet(x, y));
            [srgb(c[0]), srgb(c[1]), srgb(c[2]), 255]
        });
        let normal = paint(N, N, &|x, y| {
            let (x, y) = (x as isize, y as isize);
            let dx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
            let dy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
            let n = unit([-dx * 2.2, -dy * 2.2, 1.0]);
            [0, 1, 2, 3].map(|i| if i < 3 { ((n[i] * 0.5 + 0.5) * 255.0) as u8 } else { 255 })
        });
        let rough = paint(N, N, &|x, y| {
            let (h, ..) = heights[y * N + x];
            let w = wet(x, y).max(smooth(0.2, 0.05, h) * 0.5);
            [0, ((0.92 - 0.82 * w) * 255.0) as u8, 0, 255]
        });
        Stone { albedo, normal, rough }
    }

    /// Whether the stone at uv (`u`, `v`) is wet, for the footsteps.
    pub fn wet(&self, u: f32, v: f32) -> bool {
        let [x, y] = [u, v].map(|c| (c.rem_euclid(1.0) * STONE_SIZE as f32) as usize % STONE_SIZE);
        self.rough[y * STONE_SIZE + x][1] < 120
    }
}

/// The additive sprite atlas: flame frames, glows, embers, dust, drops, webs, the lantern face, the moon shaft,
/// reflections and mist, as rectangles in texels.
pub mod atlas {
    pub const SIZE: usize = 512;
    pub const FLAME: [usize; 4] = [0, 0, 64, 128];
    pub const WARM: [usize; 4] = [0, 128, 128, 128];
    pub const COLD: [usize; 4] = [128, 128, 128, 128];
    pub const WEB: [usize; 4] = [256, 128, 128, 128];
    pub const FACE: [usize; 4] = [384, 128, 128, 128];
    pub const BEAM: [usize; 4] = [0, 256, 64, 256];
    pub const STREAK: [usize; 4] = [64, 256, 64, 256];
    pub const MIST: [usize; 4] = [128, 256, 128, 128];
    pub const EMBER: [usize; 4] = [256, 256, 32, 32];
    pub const DUST: [usize; 4] = [288, 256, 32, 32];
    pub const DROP: [usize; 4] = [320, 256, 32, 32];
    pub const SPLASH: [usize; 4] = [352, 256, 32, 32];
}

fn sprites() -> Vec<[u8; 4]> {
    use atlas::*;
    let inside = |r: [usize; 4], x: usize, y: usize| (x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]).then(|| [(x - r[0]) as f32 / r[2] as f32, (y - r[1]) as f32 / r[3] as f32]);
    let glow = |u: f32, v: f32, k: f32| (-((u - 0.5).powi(2) + (v - 0.5).powi(2)) * k).exp();
    let paint_at = |x: usize, y: usize| -> V3 {
        if y < 128 {
            let frame = x / 64;
            let [u, v] = [(x % 64) as f32 / 64.0 * 2.0 - 1.0, 1.0 - y as f32 / 128.0];
            let t = frame as f32 * 0.9;
            let sway = (noise(v * 3.0 - t, frame as f32, 64, 21) - 0.5) * 0.7 * v;
            let width = 0.62 * (v.powf(0.45) * PI).sin().max(0.0) * (1.0 - v).powf(0.25) + 0.02;
            let lick = 0.75 + 0.5 * noise(u * 3.0 + 9.0, v * 5.0 - t * 2.0, 64, 22);
            let i = smooth(1.0, 0.25, (u - sway).abs() / width) * smooth(1.0, 0.55, v * lick);
            let heat = i * (1.0 - v * 0.6);
            return [heat.powf(0.7) * 1.0, heat.powf(1.3) * 0.62, heat.powf(3.0) * 0.32];
        }
        for (r, k, colour) in [(WARM, 18.0, [1.0, 0.5, 0.16]), (COLD, 14.0, [0.25, 0.45, 0.6]), (EMBER, 40.0, [1.0, 0.42, 0.08]), (DUST, 30.0, [0.5, 0.5, 0.48]), (DROP, 60.0, [0.6, 0.75, 0.85]), (MIST, 7.0, [0.12, 0.15, 0.17])] {
            if let Some([u, v]) = inside(r, x, y) {
                return scale(colour, glow(u, v, k));
            }
        }
        if let Some([u, v]) = inside(SPLASH, x, y) {
            let ring = ((u - 0.5).hypot(v - 0.5) - 0.32).abs();
            return scale([0.6, 0.7, 0.75], smooth(0.08, 0.0, ring));
        }
        if let Some([u, v]) = inside(WEB, x, y) {
            // threads from the corner (0, 0) and the sagging rings between them.
            let (radius, angle) = (u.hypot(v), v.atan2(u));
            let spoke = smooth(0.02, 0.0, ((angle / (PI / 2.0) * 9.0).fract() - 0.5).abs() * radius * 2.0 - 0.0);
            let ring = smooth(0.012, 0.0, ((radius * 11.0 + (angle * 9.0 * 2.0).sin().abs() * 0.3).fract() - 0.5).abs() / 11.0);
            let fade = smooth(1.0, 0.6, radius) * (0.6 + 0.4 * noise(u * 30.0, v * 30.0, 64, 23));
            return scale([0.012, 0.014, 0.018], (spoke.max(ring) * fade).min(1.0));
        }
        if let Some([u, v]) = inside(FACE, x, y) {
            let tri = |cx: f32, cy: f32, s: f32| (v > cy - s * 0.5 && v < cy + s * 0.5 && (u - cx).abs() < (v - (cy - s * 0.5)) * 0.6) as u8 as f32;
            let mouth = (v > 0.62 && v < 0.78 + 0.06 * ((u * 25.0).sin()) && (u - 0.5).abs() < 0.3 - (v - 0.7).abs()) as u8 as f32;
            let lit = tri(0.32, 0.38, 0.2).max(tri(0.68, 0.38, 0.2)).max(tri(0.5, 0.52, 0.1)).max(mouth);
            return scale([1.0, 0.6, 0.12], lit * (0.8 + 0.2 * noise(u * 9.0, v * 9.0, 64, 24)));
        }
        if let Some([u, v]) = inside(BEAM, x, y) {
            let across = smooth(0.5, 0.0, (u - 0.5).abs()) * (0.7 + 0.3 * noise(u * 8.0, v * 3.0, 64, 25));
            return scale([0.2, 0.3, 0.42], across * smooth(1.0, 0.1, v) * 0.5);
        }
        if let Some([u, v]) = inside(STREAK, x, y) {
            let across = smooth(0.5, 0.0, (u - 0.5).abs() * (1.2 + 0.8 * noise(v * 12.0, 1.0, 64, 26)));
            return scale([1.0, 0.5, 0.16], across * smooth(0.0, 0.2, v) * smooth(1.0, 0.4, v) * (0.5 + 0.5 * noise(u * 4.0, v * 20.0, 64, 27)));
        }
        [0.0; 3]
    };
    paint(SIZE, SIZE, &|x, y| {
        let c = paint_at(x, y);
        [srgb(c[0]), srgb(c[1]), srgb(c[2]), 255]
    })
}

/// `level0` and its box-filtered mip chain, block-linear with `gob_height_log2`, one level after another.
fn mips(level0: Vec<[u8; 4]>, size: [usize; 2], levels: usize, gob_height_log2: u32) -> Vec<u8> {
    let (mut image, [mut w, mut h]) = (level0, size);
    let mut out = Vec::new();
    for _ in 0..levels {
        let rows = h as u32;
        out.extend(pitch_to_block_linear(image.as_flattened(), w as u32, h as u32, 4, level_gob_height_log2(rows, gob_height_log2)));
        let [nw, nh] = [(w / 2).max(1), (h / 2).max(1)];
        image = (0..nw * nh)
            .map(|i| {
                let (x, y) = (i % nw * 2, i / nw * 2);
                let texels = [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)].map(|(a, b)| image[b.min(h - 1) * w + a.min(w - 1)]);
                std::array::from_fn(|c| (texels.iter().map(|t| t[c] as u32).sum::<u32>() / 4) as u8)
            })
            .collect();
        [w, h] = [nw, nh];
    }
    out
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn packed(vertices: &[Vertex], light: bool) -> Vec<u8> {
    vertices.iter().flat_map(|v| floats(&[&v.pos[..], &v.normal, &v.tangent, &[1.0], if light { &v.light } else { &v.uv }].concat())).collect()
}

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const SPRITES: usize = 512;
const VERTICES_VA: u64 = 0x1004_0000;

pub struct Dungeon {
    pub scene: Scene<'static>,
    world: World,
    stone: Stone,
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
    sprite_vertex: usize,
    water_vertex: usize,
    bat_vertex: usize,
    river: River,
    embers: Embers,
    fire: V3,
    light: Lightmap,
}

impl Dungeon {
    pub fn new(state: &'static VkState) -> Result<Self, String> {
        let world = World::new(8);
        let mut mesh = Builder::default();
        world.build(&mut mesh);
        let stone = Stone::new();
        let mut files = std::collections::HashMap::new();
        let mut textures = Vec::new();
        for (name, pixels, size, levels) in [
            ("stone", stone.albedo.clone(), [STONE_SIZE; 2], 12),
            ("normal", stone.normal.clone(), [STONE_SIZE; 2], 12),
            ("rough", stone.rough.clone(), [STONE_SIZE; 2], 12),
            ("light", vec![[0, 0, 0, 255]; LIGHTMAP[0] * LIGHTMAP[1]], LIGHTMAP, 1),
            ("sprites", sprites(), [atlas::SIZE; 2], 1),
            ("bat", paint(128, 128, &|x, y| {
                let (x, y) = ((x as f32 - 64.0) / 64.0, (y as f32 - 64.0) / 64.0);
                let wing = x.abs() < 0.94 && y > -0.35 + x.abs() * 0.4 && y < 0.32 - x.abs() * 0.25 + 0.12 * (x * 16.0).sin().abs();
                let body = x.abs() < 0.09 && y.abs() < 0.4;
                if wing || body { [0, 0, 0, 255] } else { [255; 4] }
            }), [128, 128], 1),
            ("water", vec![[10, 20, 24, 255]; WATER_GRID[0] * WATER_GRID[1]], WATER_GRID, 1),
            ("flat", vec![[128, 128, 255, 255]], [1, 1], 1),
        ] {
            let data = mips(pixels, size, levels, 1);
            textures.push(format!(r#"{{"file":"{name}","width":{},"height":{},"bytes":{},"mips":{levels},"gob_height_log2":1}}"#, size[0], size[1], data.len()));
            files.insert(name.to_string(), data);
        }
        let mut light = Vec::with_capacity(mesh.texels.len());
        for texel in &mesh.texels {
            let mut colour = [0.004; 3];
            for lamp in &world.lights {
                let ray = sub(lamp.pos, texel.pos);
                let d = length(ray);
                let n = dot(texel.normal, unit(ray)).max(0.0);
                if n > 0.0 && d < 15.0 && world.walls_between(add(texel.pos, scale(texel.normal, 0.03)), lamp.pos) == 0 {
                    colour = add(colour, scale(lamp.colour, n / (1.0 + 0.35 * d * d)));
                }
            }
            light.push((texel.at, texel.pos, std::array::from_fn(|i| colour[i] * texel.tint[i])));
        }
        let light = Lightmap::new(state, &light)?;
        let mut vertices = packed(&mesh.vertices, false);
        vertices.extend(packed(&mesh.vertices, true));
        let count = mesh.indices.len();
        let mut indices = mesh.indices.clone();
        indices.extend(mesh.indices.iter().map(|i| *i + mesh.vertices.len() as u32));
        let water_vertex = vertices.len() / 48;
        vertices.resize(vertices.len() + WATER_GRID[0] * WATER_GRID[1] * 48, 0);
        for y in 0..WATER_GRID[1] - 1 {
            for x in 0..WATER_GRID[0] - 1 {
                let a = (water_vertex + y * WATER_GRID[0] + x) as u32;
                let row = WATER_GRID[0] as u32;
                indices.extend([a, a + row, a + 1, a + 1, a + row, a + row + 1]);
            }
        }
        let water_indices = indices.len() - count * 2;
        let bat_vertex = vertices.len() / 48;
        vertices.resize(vertices.len() + 8 * 4 * 48, 0);
        for i in 0..8 {
            let v = (bat_vertex + i * 4) as u32;
            indices.extend([v, v + 1, v + 2, v + 2, v + 1, v + 3]);
        }
        let sprite_vertex = vertices.len() / 48;
        vertices.resize(vertices.len() + SPRITES * 4 * 48, 0);
        for i in 0..SPRITES {
            let v = (sprite_vertex + i * 4) as u32;
            indices.extend([v, v + 1, v + 2, v + 2, v + 1, v + 3]);
        }
        let material = |base, normal, rough, emissive, ambient, blend: &str| format!(r#"{{"textures":{{"baseColor":"{base}","normal":"{normal}","metallicRoughness":"{rough}","emissive":"{emissive}"}},"baseColorFactor":[1,1,1,1],"emissiveFactor":[{ambient},{ambient},{ambient}],"metallicFactor":0,"roughnessFactor":1,"normalScale":1,"occlusionStrength":0,"light":[0,1,0,0,0,0,0,0,{ambient},{ambient},{ambient},0]{blend}}}"#);
        let materials = [
            material("stone", "normal", "rough", "stone", "0.3", ""),
            material("light", "flat", "rough", "light", "1", ",\"blend\":[32774,17158,16384,0,0,0,0]"),
            material("water", "flat", "rough", "water", "0.5", ""),
            material("bat", "flat", "rough", "bat", "50", ",\"blend\":[32774,17158,16384,0,0,0,0]"),
            material("sprites", "flat", "rough", "sprites", "1", ",\"blend\":[32774,16385,16385,0,0,0,0]"),
        ];
        let manifest = format!(r#"{{"textures":[{}],"materials":[{}],"primitives":[{{"first_index":0,"index_count":{count},"material":0}},{{"first_index":{count},"index_count":{count},"material":1}},{{"first_index":{},"index_count":{water_indices},"material":2}},{{"first_index":{},"index_count":48,"material":3}},{{"first_index":{},"index_count":{},"material":4}}]}}"#, textures.join(","), materials.join(","), count * 2, count * 2 + water_indices, count * 2 + water_indices + 48, SPRITES * 6);
        files.insert("manifest.json".into(), manifest.into_bytes());
        files.insert("vertices.bin".into(), vertices);
        files.insert("indices.bin".into(), indices.iter().flat_map(|i| i.to_le_bytes()).collect());
        let scene = Scene::with_files(state, None, &|name| files.get(name).cloned().ok_or_else(|| format!("missing dungeon asset {name}")), WIDTH, HEIGHT, false)?;
        let eye = add(world.path[0], [0.0, EYE, 0.0]);
        let fire = world.sources.iter().find(|s| s.0 == Sfx::Fire).unwrap().1;
        let embers = Embers::new(state, fire)?;
        let sound = Sound::new(&world, &world.sources);
        Ok(Self { scene, world, stone, sound, eye, yaw: PI / 2.0, pitch: 0.0, time: 0.0, idle: 3.0, waypoint: 1, foot_distance: 0.0, scream: false, audio_time: 0.0, sprite_vertex, light, water_vertex, bat_vertex, river: River::new(state)?, embers, fire })
    }

    pub fn frame(&mut self, seconds: f32) -> Result<Vec<i16>, String> {
        use super::demo_dungeon_sound::Sample;
        let dt = (seconds - self.time).clamp(0.0, 0.1);
        let old_eye = self.eye;
        let input = crate::input::state().0;
        let moving = input.left.into_iter().chain(input.right).any(|v| v.abs() > 0.1);
        self.idle = if moving { 0.0 } else { self.idle + dt };
        if self.idle >= 3.0 {
            let to = sub(add(self.world.path[self.waypoint], [0.0, EYE, 0.0]), self.eye);
            let d = length(to);
            if d < 0.3 { self.waypoint = (self.waypoint + 1) % self.world.path.len(); }
            let next = add(self.eye, scale(unit(to), (dt * 4.8).min(d)));
            self.eye = self.world.clamp_walk(self.eye, next);
            let angle = to[0].atan2(-to[2]);
            let turn = (angle - self.yaw + PI).rem_euclid(TAU) - PI;
            self.yaw += turn * (dt * 4.0).min(1.0);
            self.pitch *= 1.0 - (dt * 2.0).min(1.0);
        } else {
            self.yaw += input.right[0] * dt * 2.0;
            self.pitch = (self.pitch + input.right[1] * dt * 1.3).clamp(-0.8, 0.8);
            let v = [self.yaw.cos() * input.left[0] + self.yaw.sin() * input.left[1], 0.0, self.yaw.sin() * input.left[0] - self.yaw.cos() * input.left[1]];
            self.eye = self.world.clamp_walk(self.eye, add(self.eye, scale(v, dt * 2.6)));
            self.waypoint = self.world.path.iter().enumerate().min_by(|(_, a), (_, b)| length(sub(**a, self.eye)).total_cmp(&length(sub(**b, self.eye)))).unwrap().0;
        }
        let mut impacts = Vec::new();
        self.foot_distance += length(sub(self.eye, old_eye));
        if self.foot_distance > 1.25 {
            self.foot_distance %= 1.25;
            let r = self.world.river;
            if self.eye[0] > r.lo[0] && self.eye[0] < r.hi[0] && self.eye[2] > r.lo[2] - 1.0 && self.eye[2] < r.hi[2] + 1.0 {
                impacts.push(([self.eye[0], WATER, self.eye[2].clamp(r.lo[2] + 0.2, r.hi[2] - 0.2)], 0.8));
            }
            let wet = self.stone.wet(self.eye[2] / 4.0, self.eye[0] / 4.0);
            self.sound.play(if wet { Sample::WetStep } else { Sample::Step }, self.eye, 1.0, 0.96 + 0.08 * (seconds * 7.0).sin());
        }
        if !self.scream && self.world.zones[self.world.last].space.contains(self.eye) {
            self.sound.play(Sample::Scream, self.world.zones[self.world.last].space.centre(), 1.4, 1.0);
            self.scream = true;
        }
        let right = [self.yaw.cos(), 0.0, self.yaw.sin()];
        let forward = [self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos()];
        let up = cross(right, forward);
        let mut bats = Vec::new();
        let chamber = self.world.zones[self.world.bats].space;
        let centre = chamber.centre();
        let bat_position = |i: usize| add(centre, [0.5 * (seconds * 1.8 + i as f32).cos(), 0.5 + 0.2 * (seconds * 2.0 + i as f32).sin(), 0.5 * (seconds * 2.2 + i as f32 * 1.8).sin()]);
        let bat_source = self.world.sources.iter().position(|s| s.0 == Sfx::Bats).unwrap();
        self.sound.move_source(bat_source, bat_position(0));
        for i in 0..8 {
            let p = bat_position(i);
            let flap = 0.025 + 0.12 * (seconds * TAU * 11.0 + i as f32).sin().abs();
            for [x, y] in [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]] {
                bats.push(Vertex { pos: add(p, add(scale(right, x * 0.22), scale(UP, y * flap))), normal: scale(forward, -1.0), tangent: right, uv: [(x + 1.0) * 0.5, (y + 1.0) * 0.5], light: [0.0; 2] });
            }
        }
        self.scene.gpu.write(VERTICES_VA + self.bat_vertex as u64 * 48, &packed(&bats, false)).map_err(|e| format!("{e:?}"))?;
        let mut billboards = Vec::new();
        let mut quad = |p: V3, axes: [V3; 2], size: [f32; 2], rect: [usize; 4]| {
            for [x, y] in [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]] {
                let uv = [(rect[0] as f32 + (x + 1.0) * 0.5 * rect[2] as f32) / atlas::SIZE as f32, (rect[1] as f32 + (1.0 - y) * 0.5 * rect[3] as f32) / atlas::SIZE as f32];
                billboards.push(Vertex { pos: add(p, add(scale(axes[0], x * size[0]), scale(axes[1], y * size[1]))), normal: scale(forward, -1.0), tangent: axes[0], uv, light: uv });
            }
        };
        for (i, p) in self.embers.step(self.fire, seconds, dt)?.into_iter().enumerate() {
            quad(p, [right, up], [if i % 3 == 0 { 0.025 } else { 0.012 }; 2], if i % 3 == 0 { atlas::DUST } else { atlas::EMBER });
        }
        for lamp in &self.world.lights {
            if lamp.flame == 0.0 { continue; }
            let frame = ((seconds * 12.0 + lamp.pos[0]) as usize % 8) * 64;
            quad(add(lamp.pos, [0.0, lamp.flame * 0.4, 0.0]), [right, UP], [lamp.flame * 0.3, lamp.flame * 0.55], [frame, 0, 64, 128]);
            quad(lamp.pos, [right, up], [lamp.flame * 0.5; 2], atlas::WARM);
        }
        for (i, &(tip, floor)) in self.world.drips.iter().enumerate() {
            let period = 2.4 + (i % 5) as f32 * 0.3;
            let fall = ((tip[1] - floor) / 4.9).sqrt();
            let age = (seconds + i as f32 * 0.71).rem_euclid(period);
            let previous = (self.time + i as f32 * 0.71).rem_euclid(period);
            if age < fall {
                quad([tip[0], tip[1] - 4.9 * age * age, tip[2]], [right, UP], [0.025, 0.065], atlas::DROP);
            }
            if previous < fall && age >= fall {
                if floor == WATER { impacts.push(([tip[0], floor, tip[2]], 0.55)); }
                self.sound.play(Sample::Drip, [tip[0], floor, tip[2]], 0.5, 0.8 + i as f32 * 0.03);
            }
        }
        for prop in &self.world.props {
            match *prop {
                Prop::Web(p, size) => quad(p, [right, UP], size, atlas::WEB),
                Prop::Lantern(p, yaw) => quad(add(p, turn(yaw, [0.23, 0.2, 0.0])), [turn(yaw, [0.0, 0.0, 1.0]), UP], [0.17, 0.13], atlas::FACE),
                _ => {}
            }
        }
        let moon = self.world.grate.centre();
        quad(add(moon, [0.0, -2.2, 0.0]), [right, UP], [0.9, 2.2], atlas::BEAM);
        billboards.resize(SPRITES * 4, Vertex::default());
        self.scene.gpu.write(VERTICES_VA + self.sprite_vertex as u64 * 48, &packed(&billboards, false)).map_err(|e| format!("{e:?}"))?;
        let flicker = 0.88 + 0.07 * (seconds * 11.7).sin() + 0.05 * (seconds * 23.1).sin();
        self.scene.gpu.write(self.scene.textures[3], self.light.step(self.eye, flicker)?).map_err(|e| format!("{e:?}"))?;
        let back = scale(forward, -1.0);
        let axes = [right, up, back];
        let view: Matrix = std::array::from_fn(|column| std::array::from_fn(|row| if row == 3 { if column == 3 { 1.0 } else { 0.0 } } else if column == 3 { -dot(axes[row], self.eye) } else { axes[row][column] }));
        let focal = 1.0 / (PI / 6.0).tan();
        let projection = [[focal * HEIGHT as f32 / WIDTH as f32, 0.0, 0.0, 0.0], [0.0, -focal, 0.0, 0.0], [0.0, 0.0, -100.0 / 99.9, -1.0], [0.0, 0.0, -10.0 / 99.9, 0.0]];
        let identity = std::array::from_fn(|c| std::array::from_fn(|r| if c == r { 1.0 } else { 0.0 }));
        let waves = self.river.step(self.world.river, dt, &impacts)?;
        let river = self.world.river;
        let mut water_vertices = Vec::with_capacity(waves.len());
        let mut water_pixels = Vec::with_capacity(waves.len());
        for (i, &[h, sx, sz, foam]) in waves.iter().enumerate() {
            let uv = [(i % WATER_GRID[0]) as f32 / (WATER_GRID[0] - 1) as f32, (i / WATER_GRID[0]) as f32 / (WATER_GRID[1] - 1) as f32];
            let pos = [river.lo[0] + uv[0] * (river.hi[0] - river.lo[0]), WATER + h, river.lo[2] + uv[1] * (river.hi[2] - river.lo[2])];
            let normal = unit([-sx, 1.0, -sz]);
            water_vertices.push(Vertex { pos, normal, tangent: unit([1.0, sx, 0.0]), uv, light: uv });
            let view = unit(sub(self.eye, pos));
            let mut colour = add([0.006, 0.018, 0.024], [foam * 0.3; 3]);
            for lamp in &self.world.lights {
                let ray = sub(lamp.pos, pos);
                if length(ray) > 12.0 { continue; }
                let half = unit(add(unit(ray), view));
                let specular = dot(normal, half).max(0.0).powf(150.0) * 3.0 / (1.0 + dot(ray, ray) * 0.1);
                colour = add(colour, scale(lamp.colour, specular * flicker));
            }
            water_pixels.push([srgb(colour[0]), srgb(colour[1]), srgb(colour[2]), 255]);
        }
        self.scene.gpu.write(VERTICES_VA + self.water_vertex as u64 * 48, &packed(&water_vertices, false)).map_err(|e| format!("{e:?}"))?;
        self.scene.gpu.write(self.scene.textures[6], &mips(water_pixels, WATER_GRID, 1, 1)).map_err(|e| format!("{e:?}"))?;
        let nearest = self.world.lights.iter().min_by(|a, b| length(sub(a.pos, self.eye)).total_cmp(&length(sub(b.pos, self.eye)))).unwrap();
        let direction = unit(sub(nearest.pos, self.eye));
        let lighting = [&direction[..], &[0.0], &scale(nearest.colour, 0.6), &[0.0, 0.08, 0.09, 0.12, 0.0]].concat();
        self.scene.gpu.write(Scene::uniforms(0), &floats(&lighting)).map_err(|e| format!("{e:?}"))?;
        self.scene.gpu.write(Scene::uniforms(0) + 80, &floats(&[0.02, 0.02, 0.02, 0.0])).map_err(|e| format!("{e:?}"))?;
        self.scene.draw(identity, multiply(projection, view), self.eye)?;
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
    dungeon.scene.gpu.present(dungeon.scene.target, WIDTH, HEIGHT).map_err(|e| format!("{e:?}"))
}

struct AudioCapture {
    world: World,
    sound: Sound,
    eye: V3,
    yaw: f32,
    time: f32,
    audio_time: f32,
    waypoint: usize,
    foot_distance: f32,
    scream: bool,
}

pub fn render_dungeon_audio_frame(seconds: f32) {
    static CAPTURE: Mutex<Option<AudioCapture>> = Mutex::new(None);
    let mut slot = CAPTURE.lock().unwrap();
    let capture = slot.get_or_insert_with(|| {
        let world = World::new(8);
        let eye = add(world.path[0], [0.0, EYE, 0.0]);
        let sound = Sound::new(&world, &world.sources);
        AudioCapture { world, sound, eye, yaw: PI / 2.0, time: 0.0, audio_time: 0.0, waypoint: 1, foot_distance: 0.0, scream: false }
    });
    let dt = (seconds - capture.time).clamp(0.0, 0.1);
    let old_eye = capture.eye;
    let to = sub(add(capture.world.path[capture.waypoint], [0.0, EYE, 0.0]), capture.eye);
    let distance = length(to);
    if distance < 0.3 { capture.waypoint = (capture.waypoint + 1) % capture.world.path.len(); }
    capture.eye = capture.world.clamp_walk(capture.eye, add(capture.eye, scale(unit(to), (dt * 4.8).min(distance))));
    let angle = to[0].atan2(-to[2]);
    capture.yaw += ((angle - capture.yaw + PI).rem_euclid(TAU) - PI) * (dt * 4.0).min(1.0);
    capture.foot_distance += length(sub(capture.eye, old_eye));
    if capture.foot_distance > 1.25 {
        capture.foot_distance %= 1.25;
        let wet = capture.world.river.contains(capture.eye) || capture.world.zones[capture.world.cave].space.contains(capture.eye);
        capture.sound.play(if wet { super::demo_dungeon_sound::Sample::WetStep } else { super::demo_dungeon_sound::Sample::Step }, capture.eye, 1.0, 0.96 + 0.08 * (seconds * 7.0).sin());
    }
    if !capture.scream && capture.world.zones[capture.world.last].space.contains(capture.eye) {
        capture.sound.play(super::demo_dungeon_sound::Sample::Scream, capture.world.zones[capture.world.last].space.centre(), 1.4, 1.0);
        capture.scream = true;
    }
    for (i, &(tip, floor)) in capture.world.drips.iter().enumerate() {
        let period = 2.4 + (i % 5) as f32 * 0.3;
        let fall = ((tip[1] - floor) / 4.9).sqrt();
        let age = (seconds + i as f32 * 0.71).rem_euclid(period);
        let previous = (capture.time + i as f32 * 0.71).rem_euclid(period);
        if previous < fall && age >= fall { capture.sound.play(super::demo_dungeon_sound::Sample::Drip, [tip[0], floor, tip[2]], 0.5, 0.8 + i as f32 * 0.03); }
    }
    let bats = capture.world.zones[capture.world.bats].space.centre();
    let bat_source = capture.world.sources.iter().position(|source| source.0 == Sfx::Bats).unwrap();
    capture.sound.move_source(bat_source, add(bats, [0.5 * (seconds * 1.8).cos(), 0.5 + 0.2 * (seconds * 2.0).sin(), 0.5 * (seconds * 2.2).sin()]));
    capture.sound.listen(&capture.world, capture.eye, capture.yaw, dt);
    capture.audio_time += dt;
    let periods = (capture.audio_time / 0.005).floor() as usize;
    capture.audio_time -= periods as f32 * 0.005;
    capture.time = seconds;
    let audio = capture.sound.render(periods);
    for period in audio.chunks(480) { crate::audio::play(&[period]); }
}

const WATER_GRID: [usize; 2] = [128, 32];

struct River {
    state: &'static VkState,
    pipelines: [super::compute::ComputePipeline<'static>; 2],
    _heights: [super::compute::HostBuffer<'static>; 2],
    surface: super::compute::HostBuffer<'static>,
    uniforms: super::compute::HostBuffer<'static>,
    ping: usize,
}

impl River {
    fn new(state: &'static VkState) -> Result<Self, String> {
        use super::compute::{ComputePipeline, HostBuffer};
        use ash::vk::DescriptorType as D;
        let named = |e| format!("{e:?}");
        let n = (WATER_GRID[0] * WATER_GRID[1]) as u64;
        let mut heights = [HostBuffer::new(state, n * 8).map_err(named)?, HostBuffer::new(state, n * 8).map_err(named)?];
        for h in &mut heights { h.write(&vec![0; n as usize * 8]).map_err(named)?; }
        let surface = HostBuffer::new(state, n * 16).map_err(named)?;
        let uniforms = HostBuffer::new(state, 32 + 32 * 16).map_err(named)?;
        let code: Vec<u32> = include_bytes!("../../../demos/shaders/dungeon_water.spv").chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
        let bindings = [(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::STORAGE_BUFFER), (3, D::UNIFORM_BUFFER)];
        let mut pipelines = [ComputePipeline::new(state, &code, &bindings).map_err(named)?, ComputePipeline::new(state, &code, &bindings).map_err(named)?];
        for i in 0..2 { pipelines[i].descriptors.bind(&[(0, &heights[i]), (1, &heights[1 - i]), (2, &surface), (3, &uniforms)]); }
        Ok(Self { state, pipelines, _heights: heights, surface, uniforms, ping: 0 })
    }

    fn step(&mut self, river: Box3, dt: f32, impacts: &[(V3, f32)]) -> Result<Vec<[f32; 4]>, String> {
        let named = |e| format!("{e:?}");
        let size = sub(river.hi, river.lo);
        let mut u = vec![WATER_GRID[0] as f32, WATER_GRID[1] as f32, dt.min(1.0 / 30.0), 2.0, size[0], size[2], impacts.len().min(32) as f32, 0.0];
        for &(p, strength) in impacts.iter().take(32) {
            u.extend([(p[0] - river.lo[0]) / size[0] * WATER_GRID[0] as f32, (p[2] - river.lo[2]) / size[2] * WATER_GRID[1] as f32, strength, 2.0]);
        }
        self.uniforms.write(&floats(&u)).map_err(named)?;
        self.state.dispatch(&self.pipelines[self.ping], [WATER_GRID[0] as u32 / 8, WATER_GRID[1] as u32 / 8, 1]).map_err(named)?;
        self.ping ^= 1;
        let mut out = vec![0; WATER_GRID[0] * WATER_GRID[1] * 16];
        self.surface.read(&mut out).map_err(named)?;
        Ok(out.chunks_exact(16).map(|b| std::array::from_fn(|i| f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap()))).collect())
    }
}

struct Embers {
    gpu: super::engine::Gpu<'static>,
}

impl Embers {
    fn new(state: &'static VkState, origin: V3) -> Result<Self, String> {
        use crate::tests::gpu::engine::bytes;
        let mut gpu = super::engine::Gpu::new(state, 0x20000, None, *super::SPEED_HACKS.lock().unwrap()).map_err(|e| format!("{e:?}"))?;
        gpu.space.map(0x1000_0000, 0, 0x20000);
        let code = [include_bytes!("../../tests/fixtures/driver-shaders/particles_update.comp.bin").as_slice(), &crate::tests::gpu_test::inst::bra(-16).to_le_bytes()].concat();
        gpu.write(0x1000_0000, &code).map_err(|e| format!("{e:?}"))?;
        let mut c0 = [0u32; 19];
        c0[12] = 0x1000_9000 >> 4;
        c0[13] = 0x100 >> 4 << 13;
        c0[16] = 0x1001_0000;
        c0[18] = 256 * 48;
        gpu.write(0x1000_8000, &bytes(&c0)).map_err(|e| format!("{e:?}"))?;
        let mut r = Rng(63);
        let initial: Vec<f32> = (0..256).flat_map(|_| [origin[0] + r.range(-0.3, 0.3), origin[1] + r.range(0.0, 2.0), origin[2] + r.range(-0.3, 0.3), r.next(), 0.0, 0.0, 0.0, r.range(0.3, 0.7), 0.0, 0.0, 0.0, 0.0]).collect();
        gpu.write(0x1001_0000, &floats(&initial)).map_err(|e| format!("{e:?}"))?;
        Ok(Self { gpu })
    }

    fn step(&mut self, origin: V3, seconds: f32, dt: f32) -> Result<Vec<V3>, String> {
        use crate::tests::gpu::engine::{bytes, entry, header};
        let named = |e| format!("{e:?}");
        let mut uniforms = floats(&[origin[0], origin[1], origin[2], -0.2, origin[0], origin[1] + 3.0, origin[2], 0.3, dt, -0.25, 0.99, seconds]);
        uniforms.extend(256u32.to_le_bytes());
        self.gpu.write(0x1000_9000, &uniforms).map_err(named)?;
        let mut qmd = [0u32; 64];
        qmd[4] = 0x40;
        qmd[5] = 0xfc00_0000;
        [qmd[12], qmd[13], qmd[14]] = [1, 1, 1];
        [qmd[18], qmd[19], qmd[20]] = [256 << 16 | 3 << 4, 1 << 16 | 1, 24 << 8 | 3];
        [qmd[32], qmd[33], qmd[34], qmd[35]] = [0x1000_8000, 0x1000 >> 4 << 19, 0x1000_9000, 0x100 >> 4 << 19];
        qmd[48] = 0x1000_0000;
        self.gpu.write(0x1000_a000, &bytes(&qmd)).map_err(named)?;
        let words = [header(1, 0, 1) | 1 << 13, 0xc7c0, header(1, 0x02b4, 1) | 1 << 13, 0x1000_a000 >> 8, header(1, 0x02c0, 1) | 1 << 13, 3];
        self.gpu.write(0x1000_b000, &bytes(&words)).map_err(named)?;
        self.gpu.submit(&[entry(0x1000_b000, words.len())])?;
        let mut particles = vec![0; 256 * 48];
        self.gpu.read(0x1001_0000, &mut particles).map_err(named)?;
        Ok(particles.chunks_exact(48).map(|p| std::array::from_fn(|i| f32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()))).collect())
    }
}

struct Lightmap {
    state: &'static VkState,
    pipeline: super::compute::ComputePipeline<'static>,
    texels: super::compute::HostBuffer<'static>,
    pixels: super::compute::HostBuffer<'static>,
    uniforms: super::compute::HostBuffer<'static>,
}

impl Lightmap {
    fn new(state: &'static VkState, light: &[(usize, V3, V3)]) -> Result<Self, String> {
        use super::compute::{ComputePipeline, HostBuffer};
        use ash::vk::DescriptorType as D;
        let named = |e| format!("{e:?}");
        let mut texels = HostBuffer::new(state, light.len() as u64 * 32).map_err(named)?;
        let data: Vec<u8> = light.iter().flat_map(|&(at, pos, colour)| floats(&[pos[0], pos[1], pos[2], f32::from_bits(at as u32), colour[0], colour[1], colour[2], 0.0])).collect();
        texels.write(&data).map_err(named)?;
        let mut pixels = HostBuffer::new(state, (LIGHTMAP[0] * LIGHTMAP[1] * 4) as u64).map_err(named)?;
        pixels.write(&[0, 0, 0, 255].repeat(LIGHTMAP[0] * LIGHTMAP[1])).map_err(named)?;
        let uniforms = HostBuffer::new(state, 16).map_err(named)?;
        let code: Vec<u32> = include_bytes!("../../../demos/shaders/dungeon_light.spv").chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
        let mut pipeline = ComputePipeline::new(state, &code, &[(0, D::STORAGE_BUFFER), (1, D::STORAGE_BUFFER), (2, D::UNIFORM_BUFFER)]).map_err(named)?;
        pipeline.descriptors.bind(&[(0, &texels), (1, &pixels), (2, &uniforms)]);
        Ok(Self { state, pipeline, texels, pixels, uniforms })
    }

    fn step(&mut self, eye: V3, flicker: f32) -> Result<&[u8], String> {
        let named = |e| format!("{e:?}");
        self.uniforms.write(&floats(&[eye[0], eye[1], eye[2], flicker])).map_err(named)?;
        self.state.dispatch(&self.pipeline, [(self.texels.size / 32).div_ceil(64) as u32, 1, 1]).map_err(named)?;
        self.pixels.view(0..self.pixels.size).map_err(named)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lightmap_compute_preserves_fog_flicker_and_block_linear_texels() {
        let Some(state) = crate::tests::gpu::harness::shared_device() else { return };
        let light: Vec<_> = [0, 3, 4, 7, 8, 15, 16, 1023, 1024, 8191, 8192, 16383, 16384, LIGHTMAP[0] * LIGHTMAP[1] - 1]
            .into_iter().enumerate().map(|(i, at)| (at, [i as f32, 2.0, -3.0], [0.004, i as f32 * 0.1, 20.0])).collect();
        let mut map = Lightmap::new(state, &light).unwrap();
        for (eye, flicker) in [([0.0; 3], 0.76), ([5.0, 1.0, -2.0], 1.0), ([100.0; 3], 0.9)] {
            let mut expected = vec![[0, 0, 0, 255]; LIGHTMAP[0] * LIGHTMAP[1]];
            for &(at, pos, colour) in &light {
                let fog = (-length(sub(eye, pos)) * 0.045).exp();
                let c = colour.map(|c| srgb(c * 0.12 * flicker * fog));
                expected[at] = [c[0], c[1], c[2], 255];
            }
            let expected = pitch_to_block_linear(expected.as_flattened(), LIGHTMAP[0] as u32, LIGHTMAP[1] as u32, 4, 1);
            assert!(map.step(eye, flicker).unwrap().iter().zip(expected).all(|(&actual, expected)| actual.abs_diff(expected) <= 1));
        }
    }
}
