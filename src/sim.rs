//! People splitting a 16-hour day across the catalog's goods and leisure. Needs are met first;
//! spare hours go wherever the person's own tastes say they add the most, including tools.
//! Each good can be made or bought, whichever costs that person fewer hours.

use std::path::PathBuf;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::goods::{Catalog, CharId, GoodId};
use crate::market::{Market, Routes, autarky_prices, plan_routes};
use crate::people::{Curve, Population, Taste};

pub const WAKING_HOURS: f32 = 16.0;
const SLICE: f32 = 0.25;
const SLICES: usize = (WAKING_HOURS / SLICE) as usize;

pub const POPULATION_COLS: usize = 25;
pub const POPULATION_ROWS: usize = 20;
const POPULATION: usize = POPULATION_COLS * POPULATION_ROWS;

const INITIAL_HOUR_VALUE: f32 = 0.3;
pub const DEFAULT_TRADE_COST: f32 = 0.3;
const JOB_REVIEW_CHANCE: f32 = 0.2;
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
    pub trading: f32,
    pub leisure: f32,
}

impl TimeUse {
    fn new(goods: usize) -> Self {
        Self { hours: vec![0.0; goods], trading: 0.0, leisure: 0.0 }
    }
}

#[derive(Component)]
pub struct Tools {
    pub years_left: Vec<f32>,
    pub progress: Vec<f32>,
}

impl Tools {
    fn new(goods: usize) -> Self {
        Self { years_left: vec![0.0; goods], progress: vec![0.0; goods] }
    }

    pub fn owned(&self) -> Vec<f32> {
        self.years_left.iter().map(|&y| if y > 0.0 { 1.0 } else { 0.0 }).collect()
    }
}

#[derive(Component)]
pub struct Trades {
    pub bought: Vec<f32>,
    pub sold: Vec<f32>,
}

#[derive(Component, Default)]
pub struct Wage(pub f32);

#[derive(Component, Default)]
pub struct Job(pub Option<GoodId>);

#[derive(Component, Default)]
pub struct NeedsMet(pub f32);

#[derive(Component)]
pub struct HourValue(pub f32);

#[derive(Resource)]
pub struct Economy {
    pub progress: bool,
    pub tech_speed: f32,
}

impl Default for Economy {
    fn default() -> Self {
        Self { progress: true, tech_speed: 1.0 }
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
    pub prices: Vec<f32>,
    pub shares: Vec<f32>,
    pub trading_share: f32,
    pub leisure_share: f32,
    pub hungry_share: f32,
    pub coverage: Vec<f32>,
    pub specialisation: f32,
    pub traded: f32,
    pub mismatch: f32,
}

#[derive(Resource, Default)]
pub struct History(pub Vec<Sample>);

pub struct Rng(u64);

#[derive(Resource)]
pub struct WorldRng(Rng);

#[derive(Component)]
pub struct PersonRng(Rng);

impl Rng {
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
    pub trade_cost: f32,
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
            .insert_resource(Market::new(vec![1.0; self.catalog.len()], self.trade_cost))
            .init_resource::<Economy>()
            .init_resource::<Clock>()
            .init_resource::<History>()
            .insert_resource(WorldRng(Rng(SEED)))
            .add_systems(Startup, (spawn_people, open_market).chain())
            .add_systems(
                SimTick,
                (advance_technology, allocate_time, replace_tools, clear_market, record_history).chain(),
            );
    }
}

fn spawn_people(
    mut commands: Commands,
    mut rng: ResMut<WorldRng>,
    catalog: Res<Catalog>,
    population: Res<Population>,
) {
    let mut people: Vec<_> = (0..POPULATION)
        .map(|_| {
            let endowments: Vec<f32> = population.endowments.iter().map(|e| e.draw(rng.0.next_f32())).collect();
            let needs: Vec<(CharId, f32)> = population
                .needs
                .iter()
                .map(|&(c, amount)| (c, amount.draw(rng.0.next_f32())))
                .collect();
            let preferences = population
                .preferences
                .iter()
                .map(|&(c, curve)| {
                    let taste = match curve {
                        Curve::Log { weight } => Taste::Log { weight: weight.draw(rng.0.next_f32()) },
                        Curve::Saturating { weight, scale } => {
                            Taste::Saturating { weight: weight.draw(rng.0.next_f32()), scale }
                        }
                    };
                    (c, taste)
                })
                .collect();
            let leisure = population.leisure.draw(rng.0.next_f32());
            (endowments, needs, Tastes { preferences, leisure })
        })
        .collect();
    let ease = |p: &(Vec<f32>, Vec<(CharId, f32)>, Tastes)| {
        p.0.iter().sum::<f32>() / p.1.iter().map(|n| n.1).sum::<f32>().max(1e-6)
    };
    people.sort_by(|a, b| ease(b).total_cmp(&ease(a)));

    let n = catalog.len();
    for (rank, (endowments, needs, tastes)) in people.into_iter().enumerate() {
        commands.spawn((
            Person { rank },
            Endowments(endowments),
            Needs(needs),
            tastes,
            Tools::new(n),
            Trades { bought: vec![0.0; n], sold: vec![0.0; n] },
            Wage::default(),
            Job::default(),
            TimeUse::new(n),
            NeedsMet::default(),
            HourValue(INITIAL_HOUR_VALUE),
            PersonRng(Rng(SEED ^ (rank as u64).wrapping_mul(0xA24B_AED4_963E_E407))),
        ));
    }
}

fn open_market(mut market: ResMut<Market>, catalog: Res<Catalog>, people: Query<&Endowments>) {
    let endowments: Vec<Vec<f32>> = people.iter().map(|e| e.0.clone()).collect();
    *market = Market::new(autarky_prices(&catalog, &endowments), market.trade_cost);
}

fn advance_technology(mut catalog: ResMut<Catalog>, economy: Res<Economy>, mut clock: ResMut<Clock>) {
    clock.year += 1;
    if economy.progress {
        for good in &mut catalog.goods {
            let growth = 1.0 + good.recipe.improves * economy.tech_speed;
            good.recipe.rate = (good.recipe.rate * growth).min(MAX_RATE);
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

fn annuity(years: f32, rate: f32) -> f32 {
    if rate <= 0.0 { years } else { (1.0 - (1.0 + rate).powf(-years)) / rate }
}

struct Situation<'a> {
    catalog: &'a Catalog,
    routes: &'a Routes,
    needs: &'a Needs,
    tastes: &'a Tastes,
    tools: &'a Tools,
    owned: &'a [f32],
    last_year: &'a TimeUse,
    hour_value: f32,
    discount_rate: f32,
}

fn tool_gain(p: &Situation, tool: GoodId, extra: f32, made: &[f32]) -> f32 {
    let catalog = p.catalog;
    let Some(lifetime) = catalog.goods[tool].lifetime else {
        return 0.0;
    };
    let next: Vec<f32> = (0..catalog.len())
        .map(|t| {
            let complete = p.tools.progress[t] + made[t] >= 1.0;
            if p.tools.years_left[t] > 1.0 || complete { 1.0 } else { 0.0 }
        })
        .collect();
    let room = 1.0 - p.tools.progress[tool] - made[tool];
    if next[tool] > 0.0 || room <= 0.0 {
        return 0.0;
    }
    let mut with_tool = next.clone();
    with_tool[tool] = 1.0;
    let saved: f32 = catalog
        .goods
        .iter()
        .enumerate()
        .filter(|(_, def)| def.recipe.tools.iter().any(|t| t.good == tool))
        .map(|(good, _)| {
            p.last_year.hours[good] / catalog.hours_factor(good, p.owned)
                * (catalog.hours_factor(good, &next) - catalog.hours_factor(good, &with_tool))
        })
        .sum();
    saved * p.hour_value * annuity(lifetime.mean(), p.discount_rate) * extra.min(room)
}

struct DayPlan {
    time: TimeUse,
    made: Vec<f32>,
    bought: Vec<f32>,
    sold: Vec<f32>,
    needs_met: f32,
    hour_value: f32,
}

fn plan_day(p: &Situation) -> DayPlan {
    let catalog = p.catalog;
    let n = catalog.len();
    let unit_hours = &p.routes.unit_hours;
    let mut need = vec![0.0; catalog.characteristics.len()];
    for &(c, amount) in &p.needs.0 {
        need[c] += amount;
    }

    let mut plan = DayPlan {
        time: TimeUse::new(n),
        made: vec![0.0; n],
        bought: vec![0.0; n],
        sold: vec![0.0; n],
        needs_met: 1.0,
        hour_value: 0.0,
    };
    let mut amounts = vec![0.0; catalog.characteristics.len()];
    let work = |good: GoodId, plan: &mut DayPlan, amounts: &mut Vec<f32>| {
        let units = SLICE / unit_hours[good];
        let flow = &p.routes.flows[good];
        plan.made[good] += units;
        for &(c, per_unit) in &catalog.goods[good].provides {
            amounts[c] += units * per_unit;
        }
        plan.time.trading += units * flow.trading;
        for sector in 0..n {
            plan.time.hours[sector] += units * flow.hours[sector];
            plan.bought[sector] += units * flow.bought[sector];
            plan.sold[sector] += units * flow.sold[sector];
        }
    };

    let mut slices = SLICES;
    for &(c, amount) in &p.needs.0 {
        let cheapest = (0..n)
            .filter(|&g| catalog.provided(g, c) > 0.0 && unit_hours[g].is_finite())
            .min_by(|&a, &b| {
                (unit_hours[a] / catalog.provided(a, c)).total_cmp(&(unit_hours[b] / catalog.provided(b, c)))
            });
        let Some(good) = cheapest else { continue };
        while slices > 0 && amounts[c] < amount {
            work(good, &mut plan, &mut amounts);
            slices -= 1;
        }
    }

    let mut marginal = 0.0;
    for _ in 0..slices {
        let leisure = leisure_value(p.tastes, plan.time.leisure + SLICE) - leisure_value(p.tastes, plan.time.leisure);
        let mut best = (leisure, None);
        let now = enjoyment(p.tastes, &need, &amounts);
        for good in (0..n).filter(|&g| unit_hours[g].is_finite()) {
            let extra = SLICE / unit_hours[good];
            let mut after = amounts.clone();
            for &(c, per_unit) in &catalog.goods[good].provides {
                after[c] += extra * per_unit;
            }
            let gain = enjoyment(p.tastes, &need, &after) - now + tool_gain(p, good, extra, &plan.made);
            if gain > best.0 {
                best = (gain, Some(good));
            }
        }
        marginal = best.0;
        match best.1 {
            Some(good) => work(good, &mut plan, &mut amounts),
            None => plan.time.leisure += SLICE,
        }
    }

    plan.needs_met = p.needs.0.iter().map(|&(c, amount)| amounts[c] / amount).fold(1.0, f32::min);
    plan.hour_value = marginal / SLICE;
    plan
}

fn allocate_time(
    catalog: Res<Catalog>,
    market: Res<Market>,
    population: Res<Population>,
    mut people: Query<(
        &Endowments,
        &Needs,
        &Tastes,
        &mut Tools,
        &mut Trades,
        &mut Wage,
        &mut Job,
        &mut PersonRng,
        &mut TimeUse,
        &mut NeedsMet,
        &mut HourValue,
    )>,
) {
    people.par_iter_mut().for_each(
        |(
            endowments,
            needs,
            tastes,
            mut tools,
            mut trades,
            mut wage,
            mut job,
            mut rng,
            mut time,
            mut needs_met,
            mut hour_value,
        )| {
            let owned = tools.owned();
            let review = (rng.0.next_f32() < JOB_REVIEW_CHANCE).then(|| rng.0.next_f32());
            let routes = plan_routes(&catalog, &market, &endowments.0, &owned, job.0, review);
            job.0 = routes.job;
            let plan = plan_day(&Situation {
                catalog: &catalog,
                routes: &routes,
                needs,
                tastes,
                tools: &tools,
                owned: &owned,
                last_year: &time,
                hour_value: hour_value.0,
                discount_rate: population.discount_rate,
            });
            for (good, def) in catalog.goods.iter().enumerate() {
                if def.is_tool() {
                    tools.progress[good] += plan.made[good];
                }
            }
            trades.bought = plan.bought;
            trades.sold = plan.sold;
            wage.0 = routes.wage;
            *time = plan.time;
            needs_met.0 = plan.needs_met;
            hour_value.0 = plan.hour_value;
        },
    );
}

fn replace_tools(catalog: Res<Catalog>, mut people: Query<(&mut Tools, &mut PersonRng)>) {
    for (mut tools, mut rng) in &mut people {
        for (good, def) in catalog.goods.iter().enumerate() {
            let Some(lifetime) = def.lifetime else { continue };
            tools.years_left[good] = (tools.years_left[good] - 1.0).max(0.0);
            if tools.years_left[good] <= 0.0 && tools.progress[good] >= 1.0 {
                tools.progress[good] -= 1.0;
                tools.years_left[good] = lifetime.draw(rng.0.next_f32()).round().max(1.0);
            }
            tools.progress[good] = tools.progress[good].min(1.0);
        }
    }
}

fn clear_market(mut market: ResMut<Market>, catalog: Res<Catalog>, people: Query<(&Trades, &Wage)>) {
    let n = catalog.len();
    let (mut bought, mut sold, mut wages, mut count) = (vec![0.0; n], vec![0.0; n], 0.0f32, 0.0f32);
    for (trades, wage) in &people {
        for g in 0..n {
            bought[g] += trades.bought[g];
            sold[g] += trades.sold[g];
        }
        if wage.0 > 0.0 {
            wages += wage.0;
            count += 1.0;
        }
    }
    market.adjust(bought, sold, wages / count.max(1.0));
}

fn record_history(
    clock: Res<Clock>,
    catalog: Res<Catalog>,
    market: Res<Market>,
    mut history: ResMut<History>,
    people: Query<(&TimeUse, &NeedsMet, &Tools)>,
) {
    let goods = catalog.len();
    let (mut hours, mut coverage) = (vec![0.0; goods], vec![0.0; goods]);
    let (mut trading, mut leisure, mut hungry, mut specialisation, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (time, needs_met, tools) in &people {
        for good in 0..goods {
            hours[good] += time.hours[good];
            coverage[good] += if tools.years_left[good] > 0.0 { 1.0 } else { 0.0 };
        }
        let worked: f32 = time.hours.iter().sum();
        if worked > 0.0 {
            specialisation += time.hours.iter().copied().fold(0.0, f32::max) / worked;
        }
        trading += time.trading;
        leisure += time.leisure;
        if needs_met.0 < 0.999 {
            hungry += 1.0;
        }
        n += 1.0;
    }
    let total_hours = n * WAKING_HOURS;
    let worked_hours: f32 = hours.iter().sum();
    let traded_hours: f32 = market.sold.iter().zip(&market.prices).map(|(s, p)| s * p).sum();
    history.0.push(Sample {
        year: clock.year,
        rates: catalog.goods.iter().map(|g| g.recipe.rate).collect(),
        prices: market.prices.clone(),
        shares: hours.iter().map(|h| h / total_hours).collect(),
        trading_share: trading / total_hours,
        leisure_share: leisure / total_hours,
        hungry_share: hungry / n,
        coverage: coverage.iter().map(|c| c / n).collect(),
        specialisation: specialisation / n,
        traded: (traded_hours / worked_hours.max(1e-6)).min(1.0),
        mismatch: market.mismatch(),
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
    world.insert_resource(WorldRng(Rng(SEED)));
    world.run_system_once(spawn_people).expect("spawn people");
    world.run_system_once(open_market).expect("open market");

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
        header.push(format!("{}_price", g.name));
    }
    for g in &catalog.goods {
        header.push(format!("{}_share", g.sector));
    }
    header.extend(["trading_share", "leisure_share", "hungry_share", "specialisation", "traded", "mismatch"].map(String::from));
    for g in catalog.goods.iter().filter(|g| g.is_tool()) {
        header.push(format!("{}_coverage", g.name));
    }
    writeln!(out, "{}", header.join(","))?;

    for s in samples {
        let mut row = vec![s.year.to_string()];
        row.extend(s.rates.iter().map(|v| format!("{v:.4}")));
        row.extend(s.prices.iter().map(|v| format!("{v:.4}")));
        row.extend(s.shares.iter().map(|v| format!("{v:.4}")));
        for v in [s.trading_share, s.leisure_share, s.hungry_share, s.specialisation, s.traded, s.mismatch] {
            row.push(format!("{v:.4}"));
        }
        for (good, def) in catalog.goods.iter().enumerate() {
            if def.is_tool() {
                row.push(format!("{:.4}", s.coverage[good]));
            }
        }
        writeln!(out, "{}", row.join(","))?;
    }
    Ok(())
}
