//! `cargo run --release` opens the window; `-- --headless 300` prints 300 years as CSV.
//! `--goods <file>` and `--people <file>` pick the definitions (default `goods.ron`, `people.ron`).

mod goods;
mod people;
mod sim;
mod ui;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use goods::Catalog;
use people::Population;
use sim::{History, SimPlugin, SimTick};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg_after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1));
    let goods_path = PathBuf::from(arg_after("--goods").map_or("goods.ron", |s| s.as_str()));
    let people_path = PathBuf::from(arg_after("--people").map_or("people.ron", |s| s.as_str()));
    let loaded = Catalog::load(&goods_path)
        .and_then(|catalog| Population::load(&people_path, &catalog).map(|population| (catalog, population)));
    let (catalog, population) = loaded.unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let sim = SimPlugin { catalog, population, goods_path, people_path };

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
    sim::write_csv(world.resource::<Catalog>(), &world.resource::<History>().0, &mut std::io::stdout().lock())
        .expect("write to stdout");
}
