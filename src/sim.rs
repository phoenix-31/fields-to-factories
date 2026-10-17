//! People splitting a 16-hour day across the catalog's goods and leisure. Essential needs
//! come first; spare hours go wherever they add the most value, including tools for next year.

use std::path::PathBuf;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::goods::{Catalog, GoodId, Want};

pub const WAKING_HOURS: f32 = 16.0;
const SLICE: f32 = 0.25;
const SLICES: usize = (WAKING_HOURS / SLICE) as usize;

pub const POPULATION_COLS: usize = 25;
pub const POPULATION_ROWS: usize = 20;
const POPULATION: usize = POPULATION_COLS * POPULATION_ROWS;

const LEISURE_WEIGHT: f32 = 1.0;
const DISCOUNT_RATE: f32 = 0.05;
const INITIAL_HOUR_VALUE: f32 = 0.3;
pub const MAX_RATE: f32 = 20.0;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimTick;

#[derive(Component)]
pub struct Person {
    pub rank: usize,
}

#[derive(Component)]
pub struct LandQuality(pub f32);

#[derive(Component)]
pub struct Appetite(pub f32);

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
pub struct Nourishment(pub f32);

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
pub struct CatalogSource {
    pub path: PathBuf,
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

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

pub struct SimPlugin {
    pub catalog: Catalog,
    pub goods_path: PathBuf,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.catalog.clone())
            .insert_resource(CatalogSource { path: self.goods_path.clone(), version: 0, error: None })
            .init_resource::<Economy>()
            .init_resource::<Clock>()
            .init_resource::<History>()
            .insert_resource(SimRng(42))
            .add_systems(Startup, spawn_people)
            .add_systems(SimTick, (advance_technology, allocate_time, record_history).chain());
    }
}

fn spawn_people(mut commands: Commands, mut rng: ResMut<SimRng>, catalog: Res<Catalog>) {
    let mut people: Vec<(f32, f32)> = (0..POPULATION)
        .map(|_| (rng.range(0.8, 1.2), rng.range(0.9, 1.1)))
        .collect();
    people.sort_by(|a, b| (b.0 / b.1).total_cmp(&(a.0 / a.1)));

    for (rank, (land, appetite)) in people.into_iter().enumerate() {
        commands.spawn((
            Person { rank },
            LandQuality(land),
            Appetite(appetite),
            Stock(vec![0.0; catalog.len()]),
            TimeUse::new(catalog.len()),
            Nourishment::default(),
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

fn essential_utility(amount: f32, need: f32, urgency: f32, luxury: f32) -> f32 {
    let surplus = (amount - need).max(0.0);
    urgency * amount.min(need) + luxury * (1.0 - (-surplus / need).exp())
}

fn leisure_utility(hours: f32) -> f32 {
    LEISURE_WEIGHT * (1.0 + hours).ln()
}

/// Value of adding `extra` to next year's stock of `tool`: the hours it saves each year in every
/// recipe that uses it, priced at last year's value of an hour, over its discounted lifetime.
fn tool_gain(
    catalog: &Catalog,
    tool: GoodId,
    stock: &[f32],
    made: &[f32],
    extra: f32,
    last_year: &TimeUse,
    hour_value: f32,
) -> f32 {
    let Some(depreciation) = catalog.goods[tool].depreciation else {
        return 0.0;
    };
    let next = stock[tool] * (1.0 - depreciation) + made[tool];
    let mut saved = 0.0;
    for (good, def) in catalog.goods.iter().enumerate() {
        if let Some(t) = def.recipe.tool
            && t.good == tool
        {
            saved += last_year.hours[good]
                * t.boost(stock[tool])
                * (1.0 / t.boost(next) - 1.0 / t.boost(next + extra));
        }
    }
    saved * hour_value / (depreciation + DISCOUNT_RATE)
}

struct DayPlan {
    time: TimeUse,
    made: Vec<f32>,
    nourishment: f32,
    hour_value: f32,
}

fn plan_day(
    catalog: &Catalog,
    land: f32,
    appetite: f32,
    stock: &[f32],
    last_year: &TimeUse,
    hour_value: f32,
) -> DayPlan {
    let n = catalog.len();
    let embodied: Vec<Vec<f32>> = (0..n).map(|g| catalog.embodied_hours(g, land, stock)).collect();
    let unit_hours: Vec<f32> = embodied.iter().map(|h| h.iter().sum()).collect();
    let mut made = vec![0.0; n];
    let mut time = TimeUse::new(n);

    let work = |good: GoodId, made: &mut Vec<f32>, time: &mut TimeUse| {
        made[good] += SLICE / unit_hours[good];
        for (sector, hours) in time.hours.iter_mut().enumerate() {
            *hours += SLICE * embodied[good][sector] / unit_hours[good];
        }
    };

    let mut slices = SLICES;
    for (good, def) in catalog.goods.iter().enumerate() {
        if let Want::Essential { per_appetite, .. } = def.want {
            while slices > 0 && made[good] < per_appetite * appetite {
                work(good, &mut made, &mut time);
                slices -= 1;
            }
        }
    }

    let mut marginal = 0.0;
    for _ in 0..slices {
        let mut best = (leisure_utility(time.leisure + SLICE) - leisure_utility(time.leisure), None);
        for (good, def) in catalog.goods.iter().enumerate() {
            let (now, extra) = (made[good], SLICE / unit_hours[good]);
            let gain = match def.want {
                Want::Essential { per_appetite, urgency, luxury } => {
                    let need = per_appetite * appetite;
                    essential_utility(now + extra, need, urgency, luxury)
                        - essential_utility(now, need, urgency, luxury)
                }
                Want::Diminishing { weight } => weight * ((1.0 + now + extra).ln() - (1.0 + now).ln()),
                Want::Tool => tool_gain(catalog, good, stock, &made, extra, last_year, hour_value),
            };
            if gain > best.0 {
                best = (gain, Some(good));
            }
        }
        marginal = best.0;
        match best.1 {
            Some(good) => work(good, &mut made, &mut time),
            None => time.leisure += SLICE,
        }
    }

    let nourishment = catalog
        .goods
        .iter()
        .zip(&made)
        .filter_map(|(def, &amount)| match def.want {
            Want::Essential { per_appetite, .. } => Some(amount / (per_appetite * appetite)),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min)
        .min(1.0);

    DayPlan { time, made, nourishment, hour_value: marginal / SLICE }
}

fn allocate_time(
    catalog: Res<Catalog>,
    mut people: Query<(
        &LandQuality,
        &Appetite,
        &mut Stock,
        &mut TimeUse,
        &mut Nourishment,
        &mut HourValue,
    )>,
) {
    people
        .par_iter_mut()
        .for_each(|(land, appetite, mut stock, mut time, mut nourishment, mut hour_value)| {
            let plan = plan_day(&catalog, land.0, appetite.0, &stock.0, &time, hour_value.0);
            for (good, def) in catalog.goods.iter().enumerate() {
                if let Some(depreciation) = def.depreciation {
                    stock.0[good] = stock.0[good] * (1.0 - depreciation) + plan.made[good];
                }
            }
            *time = plan.time;
            nourishment.0 = plan.nourishment;
            hour_value.0 = plan.hour_value;
        });
}

fn record_history(
    clock: Res<Clock>,
    catalog: Res<Catalog>,
    mut history: ResMut<History>,
    people: Query<(&TimeUse, &Nourishment, &Stock)>,
) {
    let goods = catalog.len();
    let (mut hours, mut coverage) = (vec![0.0; goods], vec![0.0; goods]);
    let (mut leisure, mut hungry, mut n) = (0.0, 0.0, 0.0);
    for (time, nourishment, stock) in &people {
        for good in 0..goods {
            hours[good] += time.hours[good];
            coverage[good] += stock.0[good].min(1.0);
        }
        leisure += time.leisure;
        if nourishment.0 < 0.999 {
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

/// Back to year zero, reloading the catalog file so edits to it take effect.
pub fn reset(world: &mut World) {
    let path = world.resource::<CatalogSource>().path.clone();
    match Catalog::load(&path) {
        Ok(catalog) => {
            world.insert_resource(catalog);
            world.resource_mut::<CatalogSource>().error = None;
        }
        Err(e) => world.resource_mut::<CatalogSource>().error = Some(e),
    }
    world.resource_mut::<CatalogSource>().version += 1;
    let goods = world.resource::<Catalog>().len();

    let mut people = world.query::<(&mut Stock, &mut TimeUse, &mut Nourishment, &mut HourValue)>();
    for (mut stock, mut time, mut nourishment, mut hour_value) in people.iter_mut(world) {
        stock.0 = vec![0.0; goods];
        *time = TimeUse::new(goods);
        nourishment.0 = 0.0;
        hour_value.0 = INITIAL_HOUR_VALUE;
    }
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
