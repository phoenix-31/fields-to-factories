//! The simulation: its plugin and yearly schedule, shared resources and constants, and resetting from the
//! definition files. People split a 16-hour day across the catalog's goods and leisure.

mod history;
mod person;
mod place;
mod plan;
mod trade;

pub use history::*;
pub use person::*;
pub use place::*;
use plan::*;
use trade::*;

use std::path::PathBuf;

use bevy::ecs::query::QueryData;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::goods::{Catalog, CharId, GoodId};
use crate::map::{self, WorldMap};
use crate::market::{BEST_JOB, Markets, Routes, Town, autarky_prices, hours_for_needs, plan_routes};
use crate::people::{Curve, Population, Taste};

pub const WAKING_HOURS: f32 = 16.0;
const SLICE: f32 = 0.25;
const SLICES: usize = (WAKING_HOURS / SLICE) as usize;

pub const POPULATION_COLS: usize = map::COLS;
pub const POPULATION_ROWS: usize = map::ROWS;

const INITIAL_HOUR_VALUE: f32 = 0.3;
pub const DEFAULT_TRADE_COST: f32 = 0.3;
const JOB_REVIEW_CHANCE: f32 = 0.2;
const PAID_FOR: f32 = 0.99;
const SEED: u64 = 42;
pub const MAX_RATE: f32 = 20.0;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimTick;

#[derive(Resource, Clone, Copy)]
pub struct CountOverride(pub Option<usize>);

#[derive(Resource)]
pub struct Definitions {
    pub goods_path: PathBuf,
    pub people_path: PathBuf,
    pub map_path: PathBuf,
    pub version: u32,
    pub error: Option<String>,
}

#[derive(Resource, Default)]
pub struct Clock {
    pub year: u32,
}

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
    pub map: WorldMap,
    pub goods_path: PathBuf,
    pub people_path: PathBuf,
    pub map_path: PathBuf,
    pub trade_cost: f32,
    pub count: Option<usize>,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.catalog.clone())
            .insert_resource(self.population.clone())
            .insert_resource(self.map.clone())
            .insert_resource(CountOverride(self.count))
            .insert_resource(Definitions {
                goods_path: self.goods_path.clone(),
                people_path: self.people_path.clone(),
                map_path: self.map_path.clone(),
                version: 0,
                error: None,
            })
            .insert_resource(Markets {
                towns: Vec::new(),
                trade_cost: self.trade_cost,
                travel_cost: self.map.travel_cost,
                carried: 0.0,
            })
            .init_resource::<Clock>()
            .init_resource::<History>()
            .init_resource::<Migrants>()
            .init_resource::<Occupancy>()
            .insert_resource(WorldRng(Rng(SEED)))
            .add_systems(Startup, (spawn_people, open_market).chain())
            .add_systems(
                SimTick,
                (tick_clock, count_occupancy, allocate_time, replace_tools, clear_market, migrate, record_history)
                    .chain(),
            );
    }
}

fn tick_clock(mut clock: ResMut<Clock>) {
    clock.year += 1;
}

pub fn headless_app(sim: SimPlugin) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, sim));
    app.finish();
    app.cleanup();
    app.update();
    app
}

pub fn reset(world: &mut World) {
    let (goods_path, people_path, map_path) = {
        let defs = world.resource::<Definitions>();
        (defs.goods_path.clone(), defs.people_path.clone(), defs.map_path.clone())
    };
    match load_definitions(&goods_path, &people_path, &map_path) {
        Ok((catalog, population, world_map)) => {
            world.insert_resource(catalog);
            world.insert_resource(population);
            world.insert_resource(world_map);
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
    world.insert_resource(Clock::default());
    world.resource_mut::<History>().0.clear();
}

pub fn load_definitions(
    goods: &std::path::Path,
    people: &std::path::Path,
    map: &std::path::Path,
) -> Result<(Catalog, Population, WorldMap), String> {
    let catalog = Catalog::load(goods)?;
    let population = Population::load(people, &catalog)?;
    let world_map = WorldMap::load(map, &catalog)?;
    Ok((catalog, population, world_map))
}

#[cfg(test)]
#[path = "../tests/sim.rs"]
mod tests;
