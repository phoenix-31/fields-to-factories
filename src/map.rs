//! The map, loaded from `map.ron`: smooth terrain that sets each spot's resources, and the market towns.

use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

use crate::goods::Catalog;

pub const COLS: usize = 25;
pub const ROWS: usize = 20;

#[derive(Clone, Copy, Deserialize, PartialEq)]
pub enum Terrain {
    Low,
    Middle,
    High,
    Anywhere,
}

#[derive(Clone, Deserialize)]
pub struct TownDef {
    pub name: String,
    pub at: (usize, usize),
}

#[derive(Deserialize)]
struct MapDef {
    seed: u64,
    feature_size: f32,
    terrain: Vec<(String, Terrain)>,
    towns: Vec<TownDef>,
    travel_cost: f32,
}

#[derive(Resource, Clone)]
pub struct WorldMap {
    pub towns: Vec<TownDef>,
    pub travel_cost: f32,
    pub elevation: Vec<f32>,
    pub fields: Vec<Vec<f32>>,
}

impl WorldMap {
    pub fn load(path: &Path, catalog: &Catalog) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        Self::parse(&text, catalog).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str, catalog: &Catalog) -> Result<Self, String> {
        let def: MapDef = ron::from_str(text).map_err(|e| e.to_string())?;
        if def.towns.is_empty() {
            return Err("the map needs at least one town".into());
        }
        for (i, town) in def.towns.iter().enumerate() {
            if town.at.0 >= COLS || town.at.1 >= ROWS {
                return Err(format!("town \"{}\" is off the {COLS} × {ROWS} map", town.name));
            }
            if def.towns[..i].iter().any(|t| t.name == town.name) {
                return Err(format!("town \"{}\" is defined twice", town.name));
            }
        }
        if let Some((name, _)) = def.terrain.iter().find(|(name, _)| !catalog.resources.contains(name)) {
            return Err(format!("terrain names \"{name}\", which no recipe uses"));
        }
        if def.feature_size < 1.0 {
            return Err("feature_size must be at least 1".into());
        }
        if def.travel_cost < 0.0 {
            return Err("travel_cost can't be negative".into());
        }

        let elevation = normalise(noise_field(def.seed, def.feature_size));
        let fields = catalog
            .resources
            .iter()
            .enumerate()
            .map(|(i, resource)| {
                let terrain = def
                    .terrain
                    .iter()
                    .find(|(name, _)| name == resource)
                    .map_or(Terrain::Anywhere, |(_, t)| *t);
                let own = noise_field(def.seed.wrapping_add(1 + i as u64), def.feature_size / 2.0);
                let shaped = elevation.iter().zip(&own).map(|(&e, &n)| {
                    let base = match terrain {
                        Terrain::Low => 1.0 - e,
                        Terrain::Middle => 1.0 - (2.0 * e - 1.0).abs(),
                        Terrain::High => e,
                        Terrain::Anywhere => n,
                    };
                    0.85 * base + 0.15 * n
                });
                normalise(shaped.collect())
            })
            .collect();
        Ok(Self { towns: def.towns, travel_cost: def.travel_cost, elevation, fields })
    }

    pub fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    pub fn town_at(&self, town: usize) -> (f32, f32) {
        let (x, y) = self.towns[town].at;
        (x as f32, y as f32)
    }

    pub fn render(&self, catalog: &Catalog) -> String {
        let mut out = String::from("Elevation (. low → ^ high) with towns as letters:\n");
        for row in 0..ROWS {
            for col in 0..COLS {
                let town = self.towns.iter().position(|t| t.at == (col, row));
                let e = self.elevation[row * COLS + col];
                out.push(match town {
                    Some(t) => (b'A' + t as u8) as char,
                    None => [b'.', b'-', b'~', b'^'][((e * 4.0) as usize).min(3)] as char,
                });
                out.push(' ');
            }
            out.push('\n');
        }
        for (i, town) in self.towns.iter().enumerate() {
            out.push_str(&format!("{} = {} at {:?}\n", (b'A' + i as u8) as char, town.name, town.at));
        }
        for (r, resource) in catalog.resources.iter().enumerate() {
            out.push_str(&format!("\n{resource} quality (0-9):\n"));
            for row in 0..ROWS {
                for col in 0..COLS {
                    let v = (self.fields[r][row * COLS + col] * 9.99) as u32;
                    out.push_str(&format!("{v} "));
                }
                out.push('\n');
            }
        }
        out
    }

    pub fn nearest_town(&self, at: (f32, f32)) -> (usize, f32) {
        (0..self.towns.len())
            .map(|t| (t, Self::distance(at, self.town_at(t))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("at least one town")
    }
}

fn hash(seed: u64, x: i64, y: i64) -> f32 {
    let mut z = seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (tx, ty) = (smooth(x - x0), smooth(y - y0));
    let (ix, iy) = (x0 as i64, y0 as i64);
    let top = hash(seed, ix, iy) * (1.0 - tx) + hash(seed, ix + 1, iy) * tx;
    let bottom = hash(seed, ix, iy + 1) * (1.0 - tx) + hash(seed, ix + 1, iy + 1) * tx;
    top * (1.0 - ty) + bottom * ty
}

fn noise_field(seed: u64, feature: f32) -> Vec<f32> {
    let mut field = Vec::with_capacity(COLS * ROWS);
    for row in 0..ROWS {
        for col in 0..COLS {
            let (x, y) = (col as f32 / feature, row as f32 / feature);
            field.push(value_noise(seed, x, y) + 0.5 * value_noise(seed ^ 0x5bd1_e995, 2.0 * x, 2.0 * y));
        }
    }
    field
}

fn normalise(field: Vec<f32>) -> Vec<f32> {
    let (lo, hi) = field.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let span = (hi - lo).max(1e-6);
    field.into_iter().map(|v| (v - lo) / span).collect()
}
