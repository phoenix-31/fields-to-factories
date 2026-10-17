//! The goods catalog, loaded from `goods.ron`: what each good provides and how it is made.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

use crate::people::Spread;

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
    pub improves: f32,
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
    pub lifetime: Option<Spread>,
}

impl Good {
    pub fn is_tool(&self) -> bool {
        self.lifetime.is_some()
    }
}

#[derive(Resource, Clone)]
pub struct Catalog {
    pub goods: Vec<Good>,
    pub characteristics: Vec<String>,
    pub resources: Vec<String>,
    pub build_order: Vec<GoodId>,
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
    lifetime: Option<Spread>,
}

#[derive(Deserialize)]
struct RecipeDef {
    rate: f32,
    #[serde(default)]
    uses: Option<String>,
    #[serde(default)]
    improves: f32,
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
            if def.recipe.improves < 0.0 {
                return Err(format!("\"{}\" can't improve by a negative amount", def.name));
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
                    if defs[good].lifetime.is_none() {
                        return Err(format!("tool \"{}\" must have a lifetime", t.good));
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
                lifetime: def.lifetime,
            });
        }

        if let Some(def) = defs.iter().find(|d| d.lifetime.is_some_and(|l| l.min() < 1.0)) {
            return Err(format!("\"{}\" needs a lifetime of at least 1 year", def.name));
        }

        let mut catalog = Self { goods, characteristics, resources, build_order: Vec::new() };
        for id in 0..catalog.len() {
            catalog.check_acyclic(id, &mut Vec::new())?;
        }
        let mut order = Vec::new();
        for id in 0..catalog.len() {
            catalog.visit(id, &mut order);
        }
        catalog.build_order = order;
        Ok(catalog)
    }

    fn visit(&self, good: GoodId, order: &mut Vec<GoodId>) {
        if order.contains(&good) {
            return;
        }
        for &(input, _) in &self.goods[good].recipe.inputs {
            self.visit(input, order);
        }
        order.push(good);
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
}
