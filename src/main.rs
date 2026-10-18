//! `cargo run --release` opens the window; `-- --headless 300` prints 300 years as CSV.
//! `--goods`, `--people` and `--map <file>` pick the definitions (default `goods.ron`, `people.ron`, `map.ron`);
//! `--trade-cost <x>` sets the starting trade cost (0.3 = 30% of the price, paid in time);
//! `--people-count <n>` overrides the number of people; `--show-map` prints the terrain and towns as text.

mod goods;
mod map;
mod market;
mod people;
mod sim;
mod ui;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use goods::Catalog;
use sim::{History, SimPlugin, SimTick};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg_after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1));
    let goods_path = PathBuf::from(arg_after("--goods").map_or("goods.ron", |s| s.as_str()));
    let people_path = PathBuf::from(arg_after("--people").map_or("people.ron", |s| s.as_str()));
    let map_path = PathBuf::from(arg_after("--map").map_or("map.ron", |s| s.as_str()));
    let (catalog, population, map) = sim::load_definitions(&goods_path, &people_path, &map_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    if args.iter().any(|a| a == "--show-map") {
        print!("{}", map.render(&catalog));
        return;
    }
    let trade_cost = arg_after("--trade-cost").and_then(|s| s.parse().ok()).unwrap_or(sim::DEFAULT_TRADE_COST);
    let count = arg_after("--people-count").and_then(|s| s.parse().ok());
    let sim = SimPlugin { catalog, population, map, goods_path, people_path, map_path, trade_cost, count };

    if args.iter().any(|a| a == "--headless") {
        let years = arg_after("--headless").and_then(|s| s.parse().ok()).unwrap_or(300);
        run_headless(sim, years);
        return;
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fields to Factories".into(),
                resolution: (1360, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((EguiPlugin::default(), sim, ui::UiPlugin))
        .run();
}

fn run_headless(sim: SimPlugin, years: u32) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, sim));
    app.finish();
    app.cleanup();
    app.update();
    for _ in 0..years {
        app.world_mut().run_schedule(SimTick);
    }
    let world = app.world();
    let towns: Vec<String> = world.resource::<market::Markets>().towns.iter().map(|t| t.name.clone()).collect();
    sim::write_csv(world.resource::<Catalog>(), &towns, &world.resource::<History>().0, &mut std::io::stdout().lock())
        .expect("write to stdout");
}
