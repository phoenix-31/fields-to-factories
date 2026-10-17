//! Self-sufficient people splitting a 16-hour day between farming, crafting and
//! leisure. Food is a satiable need, so hours farming ≈ appetite ÷ food per hour.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

pub const WAKING_HOURS: f32 = 16.0;
const SLICE: f32 = 0.25;

pub const POPULATION_COLS: usize = 25;
pub const POPULATION_ROWS: usize = 20;
const POPULATION: usize = POPULATION_COLS * POPULATION_ROWS;

const FOOD_URGENCY: f32 = 20.0;
const FOOD_LUXURY: f32 = 1.5;
const LEISURE_WEIGHT: f32 = 1.0;

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

#[derive(Component, Default, Clone, Copy)]
pub struct TimeUse {
    pub farm: f32,
    pub craft: f32,
    pub leisure: f32,
}

#[derive(Component, Default)]
pub struct Nourishment(pub f32);

#[derive(Resource, Clone)]
pub struct Economy {
    pub farm_productivity: f32,
    pub craft_productivity: f32,
    pub auto_growth: bool,
    pub growth_rate: f32,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            farm_productivity: 0.08,
            craft_productivity: 1.0,
            auto_growth: true,
            growth_rate: 0.015,
        }
    }
}

pub const MAX_FARM_PRODUCTIVITY: f32 = 20.0;

#[derive(Resource, Default)]
pub struct Clock {
    pub year: u32,
}

#[derive(Clone, Copy)]
pub struct Sample {
    pub year: u32,
    pub farm_productivity: f32,
    pub farm_share: f32,
    pub craft_share: f32,
    pub leisure_share: f32,
    pub hungry_share: f32,
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

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Economy>()
            .init_resource::<Clock>()
            .init_resource::<History>()
            .insert_resource(SimRng(42))
            .add_systems(Startup, spawn_people)
            .add_systems(
                SimTick,
                (advance_technology, allocate_time, record_history).chain(),
            );
    }
}

fn spawn_people(mut commands: Commands, mut rng: ResMut<SimRng>) {
    let mut people: Vec<(f32, f32)> = (0..POPULATION)
        .map(|_| (rng.range(0.8, 1.2), rng.range(0.9, 1.1)))
        .collect();
    people.sort_by(|a, b| (b.0 / b.1).total_cmp(&(a.0 / a.1)));

    for (rank, (land, appetite)) in people.into_iter().enumerate() {
        commands.spawn((
            Person { rank },
            LandQuality(land),
            Appetite(appetite),
            TimeUse::default(),
            Nourishment::default(),
        ));
    }
}

fn advance_technology(mut economy: ResMut<Economy>, mut clock: ResMut<Clock>) {
    clock.year += 1;
    if economy.auto_growth {
        economy.farm_productivity = (economy.farm_productivity * (1.0 + economy.growth_rate))
            .min(MAX_FARM_PRODUCTIVITY);
    }
}

fn food_utility(food: f32, appetite: f32) -> f32 {
    let surplus = (food - appetite).max(0.0);
    FOOD_URGENCY * food.min(appetite) + FOOD_LUXURY * (1.0 - (-surplus / appetite).exp())
}

fn goods_utility(goods: f32) -> f32 {
    (1.0 + goods).ln()
}

fn leisure_utility(hours: f32) -> f32 {
    LEISURE_WEIGHT * (1.0 + hours).ln()
}

fn allocate_time(
    economy: Res<Economy>,
    mut people: Query<(&LandQuality, &Appetite, &mut TimeUse, &mut Nourishment)>,
) {
    people
        .par_iter_mut()
        .for_each(|(land, appetite, mut time, mut nourishment)| {
            let food_rate = economy.farm_productivity * land.0;
            let goods_rate = economy.craft_productivity;
            let (mut food, mut goods) = (0.0, 0.0);
            let mut day = TimeUse::default();

            for _ in 0..(WAKING_HOURS / SLICE) as usize {
                let farm_gain = food_utility(food + SLICE * food_rate, appetite.0)
                    - food_utility(food, appetite.0);
                let craft_gain = goods_utility(goods + SLICE * goods_rate) - goods_utility(goods);
                let leisure_gain = leisure_utility(day.leisure + SLICE) - leisure_utility(day.leisure);

                if farm_gain >= craft_gain && farm_gain >= leisure_gain {
                    day.farm += SLICE;
                    food += SLICE * food_rate;
                } else if craft_gain >= leisure_gain {
                    day.craft += SLICE;
                    goods += SLICE * goods_rate;
                } else {
                    day.leisure += SLICE;
                }
            }

            *time = day;
            nourishment.0 = food / appetite.0;
        });
}

fn record_history(
    clock: Res<Clock>,
    economy: Res<Economy>,
    mut history: ResMut<History>,
    people: Query<(&TimeUse, &Nourishment)>,
) {
    let (mut farm, mut craft, mut leisure, mut hungry, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (time, nourishment) in &people {
        farm += time.farm;
        craft += time.craft;
        leisure += time.leisure;
        if nourishment.0 < 0.999 {
            hungry += 1.0;
        }
        n += 1.0;
    }
    let total_hours = n * WAKING_HOURS;
    history.0.push(Sample {
        year: clock.year,
        farm_productivity: economy.farm_productivity,
        farm_share: farm / total_hours,
        craft_share: craft / total_hours,
        leisure_share: leisure / total_hours,
        hungry_share: hungry / n,
    });
}

pub fn reset(world: &mut World) {
    world.insert_resource(Economy::default());
    world.insert_resource(Clock::default());
    world.resource_mut::<History>().0.clear();
}

pub fn write_csv(samples: &[Sample], out: &mut impl std::io::Write) -> std::io::Result<()> {
    writeln!(out, "year,farm_productivity,farm_share,craft_share,leisure_share,hungry_share")?;
    for s in samples {
        writeln!(
            out,
            "{},{:.4},{:.4},{:.4},{:.4},{:.4}",
            s.year, s.farm_productivity, s.farm_share, s.craft_share, s.leisure_share, s.hungry_share
        )?;
    }
    Ok(())
}
