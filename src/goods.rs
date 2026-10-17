//! The goods catalog, loaded from `goods.ron`: what each good provides and how it is made.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

pub type GoodId = usize;
pub type CharId = usize;
pub type ResourceId = usize;

#[derive(Clone, Copy)]
pub struct ToolUse {
    pub good: GoodId,
    pub multiplier: f32,
}

#[derive(Clone)]
pub struct Recipe {
    pub rate: f32,
    pub uses: Option<ResourceId>,
    pub improves: bool,
    pub inputs: Vec<(GoodId, f32)>,
    pub tools: Vec<ToolUse>,
}

#[derive(Clone)]
pub struct Good {
    pub name: String,
    pub sector: String,
    pub color: [u8; 3],
    pub provides: Vec<(CharId, f32)>,
    pub recipe: Recipe,
    pub depreciation: Option<f32>,
}

#[derive(Resource, Clone)]
pub struct Catalog {
    pub goods: Vec<Good>,
    pub characteristics: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Deserialize)]
struct GoodDef {
    name: String,
    sector: String,
    color: [u8; 3],
    #[serde(default)]
    provides: Vec<(String, f32)>,
    recipe: RecipeDef,
    #[serde(default)]
    depreciation: Option<f32>,
}

#[derive(Deserialize)]
struct RecipeDef {
    rate: f32,
    #[serde(default)]
    uses: Option<String>,
    #[serde(default)]
    improves: bool,
    #[serde(default)]
    inputs: Vec<(String, f32)>,
    #[serde(default)]
    tools: Vec<ToolDef>,
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

        let mut characteristics: Vec<String> = Vec::new();
        let mut resources: Vec<String> = Vec::new();
        let mut goods = Vec::with_capacity(defs.len());
        for def in &defs {
            if def.recipe.rate <= 0.0 {
                return Err(format!("\"{}\" needs a positive rate", def.name));
            }
            let provides = def
                .provides
                .iter()
                .map(|(name, amount)| {
                    let id = characteristics.iter().position(|c| c == name).unwrap_or_else(|| {
                        characteristics.push(name.clone());
                        characteristics.len() - 1
                    });
                    (id, *amount)
                })
                .collect();
            let inputs = def
                .recipe
                .inputs
                .iter()
                .map(|(name, qty)| Ok((lookup(name, &def.name)?, *qty)))
                .collect::<Result<_, String>>()?;
            let tools = def
                .recipe
                .tools
                .iter()
                .map(|t| {
                    let good = lookup(&t.good, &def.name)?;
                    if defs[good].depreciation.is_none() {
                        return Err(format!("tool \"{}\" must have a depreciation rate", t.good));
                    }
                    if t.multiplier < 1.0 {
                        return Err(format!("tool \"{}\" needs a multiplier of at least 1", t.good));
                    }
                    Ok(ToolUse { good, multiplier: t.multiplier })
                })
                .collect::<Result<_, String>>()?;
            let uses = def.recipe.uses.as_ref().map(|name| {
                resources.iter().position(|r| r == name).unwrap_or_else(|| {
                    resources.push(name.clone());
                    resources.len() - 1
                })
            });
            goods.push(Good {
                name: def.name.clone(),
                sector: def.sector.clone(),
                color: def.color,
                provides,
                recipe: Recipe {
                    rate: def.recipe.rate,
                    uses,
                    improves: def.recipe.improves,
                    inputs,
                    tools,
                },
                depreciation: def.depreciation,
            });
        }

        let catalog = Self { goods, characteristics, resources };
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

    pub fn characteristic(&self, name: &str) -> Option<CharId> {
        self.characteristics.iter().position(|c| c == name)
    }

    pub fn provided(&self, good: GoodId, characteristic: CharId) -> f32 {
        self.goods[good]
            .provides
            .iter()
            .filter(|(c, _)| *c == characteristic)
            .map(|(_, amount)| amount)
            .sum()
    }

    pub fn hours_factor(&self, good: GoodId, stock: &[f32]) -> f32 {
        let tools = &self.goods[good].recipe.tools;
        let (mut covered, mut factor, mut used) = (0.0, 1.0, 0u64);
        while covered < 1.0 {
            let best = (0..tools.len())
                .filter(|&i| used & (1 << i) == 0)
                .max_by(|&a, &b| tools[a].multiplier.total_cmp(&tools[b].multiplier));
            let Some(i) = best else { break };
            used |= 1 << i;
            let share = stock[tools[i].good].clamp(0.0, 1.0 - covered);
            factor -= (1.0 - 1.0 / tools[i].multiplier) * share;
            covered += share;
        }
        factor
    }

    pub fn tool_multiplier(&self, good: GoodId, stock: &[f32]) -> f32 {
        1.0 / self.hours_factor(good, stock)
    }

    pub fn direct_rate(&self, good: GoodId, endowments: &[f32], stock: &[f32]) -> f32 {
        let recipe = &self.goods[good].recipe;
        let endowment = recipe.uses.map_or(1.0, |r| endowments[r]);
        recipe.rate * endowment * self.tool_multiplier(good, stock)
    }

    pub fn embodied_hours(&self, good: GoodId, endowments: &[f32], stock: &[f32]) -> Vec<f32> {
        let mut hours = vec![0.0; self.len()];
        hours[good] = 1.0 / self.direct_rate(good, endowments, stock);
        for &(input, qty) in &self.goods[good].recipe.inputs {
            for (h, sub) in hours.iter_mut().zip(self.embodied_hours(input, endowments, stock)) {
                *h += qty * sub;
            }
        }
        hours
    }
}
