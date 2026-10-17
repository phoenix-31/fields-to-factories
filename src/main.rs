//! `cargo run --release` opens the window; `-- --headless 300` prints 300 years as CSV.

mod sim;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use sim::{History, SimPlugin, SimTick};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--headless") {
        let years = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(300);
        run_headless(years);
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
        .add_plugins((EguiPlugin::default(), SimPlugin, ui::UiPlugin))
        .run();
}

fn run_headless(years: u32) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimPlugin));
    app.finish();
    app.cleanup();
    app.update();
    for _ in 0..years {
        app.world_mut().run_schedule(SimTick);
    }
    let history = app.world().resource::<History>();
    sim::write_csv(&history.0, &mut std::io::stdout().lock()).expect("write to stdout");
}
