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
    pub fn blend(&self, other: &Taste) -> Taste {
        match (*self, *other) {
            (Taste::Log { weight: a }, Taste::Log { weight: b }) => Taste::Log { weight: (a + b) / 2.0 },
            (Taste::Saturating { weight: a, scale }, Taste::Saturating { weight: b, .. }) => {
                Taste::Saturating { weight: (a + b) / 2.0, scale }
            }
            (own, _) => own,
        }
    }

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

#[derive(Clone, Copy, Deserialize)]
pub struct Life {
    pub start_age: Spread,
    pub adult_at: u32,
    pub old_at: u32,
    pub old_work: f32,
    pub child_needs: f32,
    pub fertile: (u32, u32),
    pub birth_chance: f32,
    pub spare_half: f32,
    pub child_death: f32,
    pub adult_death: f32,
    pub ageing: f32,
    pub hunger_death: f32,
}

impl Default for Life {
    fn default() -> Self {
        Self {
            start_age: Spread::Range(15.0, 35.0),
            adult_at: 12,
            old_at: 55,
            old_work: 0.5,
            child_needs: 0.25,
            fertile: (16, 40),
            birth_chance: 0.3,
            spare_half: 2.0,
            child_death: 0.02,
            adult_death: 0.01,
            ageing: 0.07,
            hunger_death: 0.2,
        }
    }
}

impl Life {
    pub fn child_share(&self, age: u32) -> f32 {
        let grown = age as f32 / self.adult_at.max(1) as f32;
        self.child_needs + (1.0 - self.child_needs) * grown.min(1.0)
    }

    pub fn death_chance(&self, age: u32, needs_met: f32) -> f32 {
        let base = if age < self.adult_at {
            self.child_death
        } else {
            self.adult_death * (self.ageing * (age as f32 - 30.0)).exp()
        };
        (base + self.hunger_death * (1.0 - needs_met).max(0.0)).min(1.0)
    }

    pub fn birth_chance(&self, age: u32, needs_met: f32, spare_hours: f32) -> f32 {
        if age < self.fertile.0 || age > self.fertile.1 {
            return 0.0;
        }
        self.birth_chance * needs_met.min(1.0) * spare_hours / (spare_hours + self.spare_half.max(1e-6))
    }
}

#[derive(Resource, Clone)]
pub struct Population {
    pub count: Spread,
    pub migration: Migration,
    pub life: Life,
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
    #[serde(default)]
    life: Life,
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
    Spread::Fixed(300.0)
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
        let l = def.life;
        let chances = [l.birth_chance, l.child_death, l.adult_death, l.hunger_death, l.old_work];
        if chances.iter().any(|c| !(0.0..=1.0).contains(c)) || !(0.0..=1.0).contains(&l.child_needs) {
            return Err("life chances, old_work and child_needs must be between 0 and 1".into());
        }
        if l.adult_at >= l.old_at || l.fertile.0 > l.fertile.1 || l.start_age.min() < l.adult_at as f32 {
            return Err("life ages must run adult_at < old_at, with fertile (from, to) and start_age adults".into());
        }
        Ok(Self {
            count: def.count,
            migration: def.migration,
            life: def.life,
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

#[cfg(test)]
#[path = "tests/people.rs"]
mod tests;
