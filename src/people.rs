//! The population, loaded from `people.ron`: each person's needs and tastes, drawn from ranges.

use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

use crate::goods::{Catalog, CharId};

#[derive(Clone, Copy, Deserialize)]
#[serde(untagged)]
pub enum Spread {
    Fixed(f32),
    Range(f32, f32),
}

impl Spread {
    pub fn min(&self) -> f32 {
        match *self {
            Spread::Fixed(v) => v,
            Spread::Range(lo, hi) => lo.min(hi),
        }
    }

    pub fn mean(&self) -> f32 {
        match *self {
            Spread::Fixed(v) => v,
            Spread::Range(lo, hi) => (lo + hi) / 2.0,
        }
    }

    pub fn draw(&self, unit: f32) -> f32 {
        match *self {
            Spread::Fixed(v) => v,
            Spread::Range(lo, hi) => lo + (hi - lo) * unit,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
pub enum Curve {
    Log { weight: Spread },
    Saturating { weight: Spread, scale: f32 },
}

#[derive(Clone, Copy)]
pub enum Taste {
    Log { weight: f32 },
    Saturating { weight: f32, scale: f32 },
}

impl Taste {
    pub fn value(&self, amount: f32) -> f32 {
        match *self {
            Taste::Log { weight } => weight * (1.0 + amount).ln(),
            Taste::Saturating { weight, scale } => weight * (1.0 - (-amount / scale).exp()),
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
pub struct Migration {
    pub chance: f32,
    pub look_at: usize,
    pub min_gain: f32,
}

impl Default for Migration {
    fn default() -> Self {
        Self { chance: 0.05, look_at: 6, min_gain: 0.1 }
    }
}

#[derive(Resource, Clone)]
pub struct Population {
    pub count: Spread,
    pub migration: Migration,
    pub endowments: Vec<Spread>,
    pub leisure: Spread,
    pub discount_rate: f32,
    pub needs: Vec<(CharId, Spread)>,
    pub preferences: Vec<(CharId, Curve)>,
}

#[derive(Deserialize)]
struct PopulationDef {
    #[serde(default = "default_count")]
    count: Spread,
    #[serde(default)]
    migration: Migration,
    endowments: Vec<(String, Spread)>,
    leisure: Spread,
    discount_rate: f32,
    needs: Vec<NeedDef>,
    preferences: Vec<PreferenceDef>,
}

#[derive(Deserialize)]
struct NeedDef {
    of: String,
    amount: Spread,
}

#[derive(Deserialize)]
struct PreferenceDef {
    of: String,
    curve: Curve,
}

fn default_count() -> Spread {
    Spread::Fixed(1500.0)
}

impl Population {
    pub fn load(path: &Path, catalog: &Catalog) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        Self::parse(&text, catalog).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str, catalog: &Catalog) -> Result<Self, String> {
        let def: PopulationDef = ron::from_str(text).map_err(|e| e.to_string())?;
        let resolve = |name: &str| {
            catalog
                .characteristic(name)
                .ok_or_else(|| format!("no good provides \"{name}\""))
        };
        let endowments = catalog
            .resources
            .iter()
            .map(|resource| {
                def.endowments
                    .iter()
                    .find(|(name, _)| name == resource)
                    .map(|(_, spread)| *spread)
                    .ok_or_else(|| format!("goods use \"{resource}\" but there is no endowment for it"))
            })
            .collect::<Result<Vec<Spread>, String>>()?;
        if let Some(i) = endowments.iter().position(|e| e.min() <= 0.0) {
            return Err(format!("endowment \"{}\" must be above zero", catalog.resources[i]));
        }
        if def.count.min() < 1.0 {
            return Err("count must be at least 1".into());
        }
        let m = def.migration;
        if !(0.0..=1.0).contains(&m.chance) || m.min_gain < 0.0 {
            return Err("migration chance must be between 0 and 1, and min_gain can't be negative".into());
        }
        Ok(Self {
            count: def.count,
            migration: def.migration,
            endowments,
            leisure: def.leisure,
            discount_rate: def.discount_rate,
            needs: def
                .needs
                .iter()
                .map(|n| Ok((resolve(&n.of)?, n.amount)))
                .collect::<Result<_, String>>()?,
            preferences: def
                .preferences
                .iter()
                .map(|p| Ok((resolve(&p.of)?, p.curve)))
                .collect::<Result<_, String>>()?,
        })
    }
}
