//! The goods catalog, loaded from `goods.ron`: what each good is wanted for and how it is made.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

pub type GoodId = usize;

#[derive(Clone, Copy, Deserialize)]
pub enum Want {
    Essential { per_appetite: f32, urgency: f32, luxury: f32 },
    Diminishing { weight: f32 },
    Tool,
}

#[derive(Clone, Copy)]
pub struct ToolUse {
    pub good: GoodId,
    pub multiplier: f32,
}

impl ToolUse {
    /// Output multiplier from holding `stock` of the tool. Every unit up to one saves
    /// the same number of hours, so a part-worn tool is worth a matching share of a new one.
    pub fn boost(&self, stock: f32) -> f32 {
        1.0 / (1.0 - (1.0 - 1.0 / self.multiplier) * stock.clamp(0.0, 1.0))
    }
}

#[derive(Clone)]
pub struct Recipe {
    pub rate: f32,
    pub uses_land: bool,
    pub improves: bool,
    pub inputs: Vec<(GoodId, f32)>,
    pub tool: Option<ToolUse>,
}

#[derive(Clone)]
pub struct Good {
    pub name: String,
    pub sector: String,
    pub color: [u8; 3],
    pub want: Want,
    pub recipe: Recipe,
    pub depreciation: Option<f32>,
}

#[derive(Resource, Clone)]
pub struct Catalog {
    pub goods: Vec<Good>,
}

#[derive(Deserialize)]
struct GoodDef {
    name: String,
    sector: String,
    color: [u8; 3],
    want: Want,
    recipe: RecipeDef,
    #[serde(default)]
    depreciation: Option<f32>,
}

#[derive(Deserialize)]
struct RecipeDef {
    rate: f32,
    #[serde(default)]
    uses_land: bool,
    #[serde(default)]
    improves: bool,
    #[serde(default)]
    inputs: Vec<(String, f32)>,
    #[serde(default)]
    tool: Option<ToolDef>,
}

#[derive(Deserialize)]
struct ToolDef {
    good: String,
    multiplier: f32,
}

impl Catalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let defs: Vec<GoodDef> = ron::from_str(text).map_err(|e| e.to_string())?;
        let mut ids = HashMap::new();
        for (id, def) in defs.iter().enumerate() {
            if ids.insert(def.name.clone(), id).is_some() {
                return Err(format!("\"{}\" is defined twice", def.name));
            }
        }
        let lookup = |name: &str, user: &str| {
            ids.get(name)
                .copied()
                .ok_or_else(|| format!("\"{user}\" refers to unknown good \"{name}\""))
        };

        let mut goods = Vec::with_capacity(defs.len());
        for def in &defs {
            if def.recipe.rate <= 0.0 {
                return Err(format!("\"{}\" needs a positive rate", def.name));
            }
            let inputs = def
                .recipe
                .inputs
                .iter()
                .map(|(name, qty)| Ok((lookup(name, &def.name)?, *qty)))
                .collect::<Result<_, String>>()?;
            let tool = match &def.recipe.tool {
                Some(t) => {
                    let good = lookup(&t.good, &def.name)?;
                    if defs[good].depreciation.is_none() {
                        return Err(format!("tool \"{}\" must have a depreciation rate", t.good));
                    }
                    Some(ToolUse { good, multiplier: t.multiplier })
                }
                None => None,
            };
            goods.push(Good {
                name: def.name.clone(),
                sector: def.sector.clone(),
                color: def.color,
                want: def.want,
                recipe: Recipe {
                    rate: def.recipe.rate,
                    uses_land: def.recipe.uses_land,
                    improves: def.recipe.improves,
                    inputs,
                    tool,
                },
                depreciation: def.depreciation,
            });
        }

        let catalog = Self { goods };
        for id in 0..catalog.len() {
            catalog.check_acyclic(id, &mut Vec::new())?;
        }
        Ok(catalog)
    }

    fn check_acyclic(&self, good: GoodId, path: &mut Vec<GoodId>) -> Result<(), String> {
        if path.contains(&good) {
            return Err(format!("\"{}\" is (indirectly) an input to itself", self.goods[good].name));
        }
        path.push(good);
        for &(input, _) in &self.goods[good].recipe.inputs {
            self.check_acyclic(input, path)?;
        }
        path.pop();
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.goods.len()
    }

    pub fn tool_multiplier(&self, good: GoodId, stock: &[f32]) -> f32 {
        match self.goods[good].recipe.tool {
            Some(tool) => tool.boost(stock[tool.good]),
            None => 1.0,
        }
    }

    pub fn direct_rate(&self, good: GoodId, land: f32, stock: &[f32]) -> f32 {
        let recipe = &self.goods[good].recipe;
        let land = if recipe.uses_land { land } else { 1.0 };
        recipe.rate * land * self.tool_multiplier(good, stock)
    }

    /// Hours of work in each sector per unit of `good`, including the hours behind its inputs.
    pub fn embodied_hours(&self, good: GoodId, land: f32, stock: &[f32]) -> Vec<f32> {
        let mut hours = vec![0.0; self.len()];
        hours[good] = 1.0 / self.direct_rate(good, land, stock);
        for &(input, qty) in &self.goods[good].recipe.inputs {
            for (h, sub) in hours.iter_mut().zip(self.embodied_hours(input, land, stock)) {
                *h += qty * sub;
            }
        }
        hours
    }
}
