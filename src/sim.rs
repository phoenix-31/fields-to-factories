//! People splitting a 16-hour day across the catalog's goods and leisure. Needs are met first;
//! spare hours go wherever the person's own tastes say they add the most, including tools.

use std::path::PathBuf;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::goods::{Catalog, CharId, GoodId};
use crate::people::{Curve, Population, Taste};

pub const WAKING_HOURS: f32 = 16.0;
const SLICE: f32 = 0.25;
const SLICES: usize = (WAKING_HOURS / SLICE) as usize;

pub const POPULATION_COLS: usize = 25;
pub const POPULATION_ROWS: usize = 20;
const POPULATION: usize = POPULATION_COLS * POPULATION_ROWS;

const INITIAL_HOUR_VALUE: f32 = 0.3;
const SEED: u64 = 42;
pub const MAX_RATE: f32 = 20.0;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimTick;

#[derive(Component)]
pub struct Person {
    pub rank: usize,
}

#[derive(Component)]
pub struct Endowments(pub Vec<f32>);

#[derive(Component)]
pub struct Needs(pub Vec<(CharId, f32)>);

#[derive(Component)]
pub struct Tastes {
    pub preferences: Vec<(CharId, Taste)>,
    pub leisure: f32,
}

#[derive(Component, Default, Clone)]
pub struct TimeUse {
    pub hours: Vec<f32>,
    pub leisure: f32,
}

impl TimeUse {
    fn new(goods: usize) -> Self {
        Self { hours: vec![0.0; goods], leisure: 0.0 }
    }
}

#[derive(Component)]
pub struct Stock(pub Vec<f32>);

#[derive(Component, Default)]
pub struct NeedsMet(pub f32);

#[derive(Component)]
pub struct HourValue(pub f32);

#[derive(Resource)]
pub struct Economy {
    pub auto_growth: bool,
    pub growth_rate: f32,
}

impl Default for Economy {
    fn default() -> Self {
        Self { auto_growth: true, growth_rate: 0.015 }
    }
}

#[derive(Resource)]
pub struct Definitions {
    pub goods_path: PathBuf,
    pub people_path: PathBuf,
    pub version: u32,
    pub error: Option<String>,
}

#[derive(Resource, Default)]
pub struct Clock {
    pub year: u32,
}

#[derive(Clone)]
pub struct Sample {
    pub year: u32,
    pub rates: Vec<f32>,
    pub shares: Vec<f32>,
    pub leisure_share: f32,
    pub hungry_share: f32,
    pub coverage: Vec<f32>,
}

#[derive(Resource, Default)]
pub struct History(pub Vec<Sample>);

#[derive(Resource)]
pub struct SimRng(u64);

impl SimRng {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub struct SimPlugin {
    pub catalog: Catalog,
    pub population: Population,
    pub goods_path: PathBuf,
    pub people_path: PathBuf,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.catalog.clone())
            .insert_resource(self.population.clone())
            .insert_resource(Definitions {
                goods_path: self.goods_path.clone(),
                people_path: self.people_path.clone(),
                version: 0,
                error: None,
            })
            .init_resource::<Economy>()
            .init_resource::<Clock>()
            .init_resource::<History>()
            .insert_resource(SimRng(SEED))
            .add_systems(Startup, spawn_people)
            .add_systems(SimTick, (advance_technology, allocate_time, record_history).chain());
    }
}

fn spawn_people(
    mut commands: Commands,
    mut rng: ResMut<SimRng>,
    catalog: Res<Catalog>,
    population: Res<Population>,
) {
    let mut people: Vec<_> = (0..POPULATION)
        .map(|_| {
            let endowments: Vec<f32> = population.endowments.iter().map(|e| e.draw(rng.next_f32())).collect();
            let needs: Vec<(CharId, f32)> = population
                .needs
                .iter()
                .map(|&(c, amount)| (c, amount.draw(rng.next_f32())))
                .collect();
            let preferences = population
                .preferences
                .iter()
                .map(|&(c, curve)| {
                    let taste = match curve {
                        Curve::Log { weight } => Taste::Log { weight: weight.draw(rng.next_f32()) },
                        Curve::Saturating { weight, scale } => {
                            Taste::Saturating { weight: weight.draw(rng.next_f32()), scale }
                        }
                    };
                    (c, taste)
                })
                .collect();
            let leisure = population.leisure.draw(rng.next_f32());
            (endowments, needs, Tastes { preferences, leisure })
        })
        .collect();
    let ease = |p: &(Vec<f32>, Vec<(CharId, f32)>, Tastes)| {
        p.0.iter().sum::<f32>() / p.1.iter().map(|n| n.1).sum::<f32>().max(1e-6)
    };
    people.sort_by(|a, b| ease(b).total_cmp(&ease(a)));

    for (rank, (endowments, needs, tastes)) in people.into_iter().enumerate() {
        commands.spawn((
            Person { rank },
            Endowments(endowments),
            Needs(needs),
            tastes,
            Stock(vec![0.0; catalog.len()]),
            TimeUse::new(catalog.len()),
            NeedsMet::default(),
            HourValue(INITIAL_HOUR_VALUE),
        ));
    }
}

fn advance_technology(mut catalog: ResMut<Catalog>, economy: Res<Economy>, mut clock: ResMut<Clock>) {
    clock.year += 1;
    if economy.auto_growth {
        for good in catalog.goods.iter_mut().filter(|g| g.recipe.improves) {
            good.recipe.rate = (good.recipe.rate * (1.0 + economy.growth_rate)).min(MAX_RATE);
        }
    }
}

fn enjoyment(tastes: &Tastes, need: &[f32], amounts: &[f32]) -> f32 {
    tastes
        .preferences
        .iter()
        .map(|&(c, taste)| taste.value((amounts[c] - need[c]).max(0.0)))
        .sum()
}

fn leisure_value(tastes: &Tastes, hours: f32) -> f32 {
    tastes.leisure * (1.0 + hours).ln()
}

fn next_year_stock(catalog: &Catalog, stock: &[f32], made: &[f32]) -> Vec<f32> {
    catalog
        .goods
        .iter()
        .enumerate()
        .map(|(g, def)| def.depreciation.map_or(0.0, |d| stock[g] * (1.0 - d) + made[g]))
        .collect()
}

fn tool_gain(catalog: &Catalog, tool: GoodId, extra: f32, p: &Situation, next: &[f32], discount_rate: f32) -> f32 {
    let Some(depreciation) = catalog.goods[tool].depreciation else {
        return 0.0;
    };
    let mut with_extra = next.to_vec();
    with_extra[tool] += extra;
    let mut saved = 0.0;
    for (good, def) in catalog.goods.iter().enumerate() {
        if def.recipe.tools.iter().any(|t| t.good == tool) {
            saved += p.last_year.hours[good] / catalog.hours_factor(good, p.stock)
                * (catalog.hours_factor(good, next) - catalog.hours_factor(good, &with_extra));
        }
    }
    saved * p.hour_value / (depreciation + discount_rate)
}

struct DayPlan {
    time: TimeUse,
    made: Vec<f32>,
    needs_met: f32,
    hour_value: f32,
}

struct Situation<'a> {
    endowments: &'a [f32],
    needs: &'a Needs,
    tastes: &'a Tastes,
    stock: &'a [f32],
    last_year: &'a TimeUse,
    hour_value: f32,
}

fn plan_day(catalog: &Catalog, discount_rate: f32, p: Situation) -> DayPlan {
    let n = catalog.len();
    let embodied: Vec<Vec<f32>> = (0..n).map(|g| catalog.embodied_hours(g, p.endowments, p.stock)).collect();
    let unit_hours: Vec<f32> = embodied.iter().map(|h| h.iter().sum()).collect();
    let mut need = vec![0.0; catalog.characteristics.len()];
    for &(c, amount) in &p.needs.0 {
        need[c] += amount;
    }

    let mut made = vec![0.0; n];
    let mut amounts = vec![0.0; catalog.characteristics.len()];
    let mut time = TimeUse::new(n);
    let work = |good: GoodId, made: &mut Vec<f32>, amounts: &mut Vec<f32>, time: &mut TimeUse| {
        let units = SLICE / unit_hours[good];
        made[good] += units;
        for &(c, per_unit) in &catalog.goods[good].provides {
            amounts[c] += units * per_unit;
        }
        for (sector, hours) in time.hours.iter_mut().enumerate() {
            *hours += SLICE * embodied[good][sector] / unit_hours[good];
        }
    };

    let mut slices = SLICES;
    for &(c, amount) in &p.needs.0 {
        let cheapest = (0..n)
            .filter(|&g| catalog.provided(g, c) > 0.0)
            .min_by(|&a, &b| {
                (unit_hours[a] / catalog.provided(a, c)).total_cmp(&(unit_hours[b] / catalog.provided(b, c)))
            });
        let Some(good) = cheapest else { continue };
        while slices > 0 && amounts[c] < amount {
            work(good, &mut made, &mut amounts, &mut time);
            slices -= 1;
        }
    }

    let mut marginal = 0.0;
    for _ in 0..slices {
        let leisure_gain = leisure_value(p.tastes, time.leisure + SLICE) - leisure_value(p.tastes, time.leisure);
        let mut best = (leisure_gain, None);
        let now = enjoyment(p.tastes, &need, &amounts);
        let next = next_year_stock(catalog, p.stock, &made);
        for good in 0..n {
            let extra = SLICE / unit_hours[good];
            let mut after = amounts.clone();
            for &(c, per_unit) in &catalog.goods[good].provides {
                after[c] += extra * per_unit;
            }
            let gain = enjoyment(p.tastes, &need, &after) - now
                + tool_gain(catalog, good, extra, &p, &next, discount_rate);
            if gain > best.0 {
                best = (gain, Some(good));
            }
        }
        marginal = best.0;
        match best.1 {
            Some(good) => work(good, &mut made, &mut amounts, &mut time),
            None => time.leisure += SLICE,
        }
    }

    let needs_met = p
        .needs
        .0
        .iter()
        .map(|&(c, amount)| amounts[c] / amount)
        .fold(1.0, f32::min);

    DayPlan { time, made, needs_met, hour_value: marginal / SLICE }
}

fn allocate_time(
    catalog: Res<Catalog>,
    population: Res<Population>,
    mut people: Query<(
        &Endowments,
        &Needs,
        &Tastes,
        &mut Stock,
        &mut TimeUse,
        &mut NeedsMet,
        &mut HourValue,
    )>,
) {
    people.par_iter_mut().for_each(
        |(endowments, needs, tastes, mut stock, mut time, mut needs_met, mut hour_value)| {
            let plan = plan_day(
                &catalog,
                population.discount_rate,
                Situation {
                    endowments: &endowments.0,
                    needs,
                    tastes,
                    stock: &stock.0,
                    last_year: &time,
                    hour_value: hour_value.0,
                },
            );
            for (good, def) in catalog.goods.iter().enumerate() {
                if let Some(depreciation) = def.depreciation {
                    stock.0[good] = stock.0[good] * (1.0 - depreciation) + plan.made[good];
                }
            }
            *time = plan.time;
            needs_met.0 = plan.needs_met;
            hour_value.0 = plan.hour_value;
        },
    );
}

fn record_history(
    clock: Res<Clock>,
    catalog: Res<Catalog>,
    mut history: ResMut<History>,
    people: Query<(&TimeUse, &NeedsMet, &Stock)>,
) {
    let goods = catalog.len();
    let (mut hours, mut coverage) = (vec![0.0; goods], vec![0.0; goods]);
    let (mut leisure, mut hungry, mut n) = (0.0, 0.0, 0.0);
    for (time, needs_met, stock) in &people {
        for good in 0..goods {
            hours[good] += time.hours[good];
            coverage[good] += stock.0[good].min(1.0);
        }
        leisure += time.leisure;
        if needs_met.0 < 0.999 {
            hungry += 1.0;
        }
        n += 1.0;
    }
    let total_hours = n * WAKING_HOURS;
    history.0.push(Sample {
        year: clock.year,
        rates: catalog.goods.iter().map(|g| g.recipe.rate).collect(),
        shares: hours.iter().map(|h| h / total_hours).collect(),
        leisure_share: leisure / total_hours,
        hungry_share: hungry / n,
        coverage: coverage.iter().map(|c| c / n).collect(),
    });
}

pub fn reset(world: &mut World) {
    let (goods_path, people_path) = {
        let defs = world.resource::<Definitions>();
        (defs.goods_path.clone(), defs.people_path.clone())
    };
    let loaded = Catalog::load(&goods_path)
        .and_then(|catalog| Population::load(&people_path, &catalog).map(|population| (catalog, population)));
    match loaded {
        Ok((catalog, population)) => {
            world.insert_resource(catalog);
            world.insert_resource(population);
            world.resource_mut::<Definitions>().error = None;
        }
        Err(e) => world.resource_mut::<Definitions>().error = Some(e),
    }

    let people: Vec<Entity> = world.query_filtered::<Entity, With<Person>>().iter(world).collect();
    for entity in people {
        world.entity_mut(entity).despawn();
    }
    world.insert_resource(SimRng(SEED));
    world.run_system_once(spawn_people).expect("spawn people");

    world.resource_mut::<Definitions>().version += 1;
    world.insert_resource(Economy::default());
    world.insert_resource(Clock::default());
    world.resource_mut::<History>().0.clear();
}

pub fn write_csv(
    catalog: &Catalog,
    samples: &[Sample],
    out: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut header = vec!["year".to_string()];
    for g in &catalog.goods {
        header.push(format!("{}_rate", g.name));
    }
    for g in &catalog.goods {
        header.push(format!("{}_share", g.sector));
    }
    header.extend(["leisure_share".into(), "hungry_share".into()]);
    for g in catalog.goods.iter().filter(|g| g.depreciation.is_some()) {
        header.push(format!("{}_coverage", g.name));
    }
    writeln!(out, "{}", header.join(","))?;

    for s in samples {
        let mut row = vec![s.year.to_string()];
        row.extend(s.rates.iter().map(|v| format!("{v:.4}")));
        row.extend(s.shares.iter().map(|v| format!("{v:.4}")));
        row.push(format!("{:.4}", s.leisure_share));
        row.push(format!("{:.4}", s.hungry_share));
        for (good, def) in catalog.goods.iter().enumerate() {
            if def.depreciation.is_some() {
                row.push(format!("{:.4}", s.coverage[good]));
            }
        }
        writeln!(out, "{}", row.join(","))?;
    }
    Ok(())
}
