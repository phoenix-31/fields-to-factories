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

#[derive(Resource, Clone)]
pub struct Population {
    pub land_quality: Spread,
    pub leisure: Spread,
    pub discount_rate: f32,
    pub needs: Vec<(CharId, Spread)>,
    pub preferences: Vec<(CharId, Curve)>,
}

#[derive(Deserialize)]
struct PopulationDef {
    land_quality: Spread,
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
        Ok(Self {
            land_quality: def.land_quality,
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
