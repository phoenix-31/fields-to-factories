//! Windowed front end: people grid, control panel and history chart, all driven by the catalog.

use bevy::camera::{CameraOutputMode, Viewport, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};
use egui_plot::{FilledArea, Line, Plot, PlotPoints};

use crate::goods::{Catalog, GoodId, Want};
use crate::sim::*;

const PANEL_WIDTH: f32 = 480.0;
const CELL: f32 = 24.0;
const GAP: f32 = 4.0;

const LEISURE_COLOR: [u8; 3] = [78, 140, 230];
const HUNGRY_COLOR: [u8; 3] = [214, 64, 64];
const COVERAGE_COLOR: [u8; 3] = [235, 235, 235];

#[derive(Resource)]
struct SimSpeed {
    years_per_second: f32,
    paused: bool,
    pending: f32,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimSpeed { years_per_second: 15.0, paused: false, pending: 0.0 })
            .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
            .add_systems(Startup, setup_cameras)
            .add_systems(PostStartup, place_people)
            .add_systems(Update, (drive_sim, rebuild_bars, draw_days).chain())
            .add_systems(EguiPrimaryContextPass, control_panel);
    }
}

fn setup_cameras(mut commands: Commands, mut egui_settings: ResMut<EguiGlobalSettings>) {
    egui_settings.auto_create_primary_context = false;
    commands.spawn(Camera2d);
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera {
            order: 1,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
}

#[derive(Component, Clone, Copy)]
struct DaySegment(Option<GoodId>);

fn place_people(mut commands: Commands, people: Query<(Entity, &Person)>) {
    let pitch = CELL + GAP;
    let origin = Vec2::new(
        -(POPULATION_COLS as f32 - 1.0) * pitch / 2.0,
        (POPULATION_ROWS as f32 - 1.0) * pitch / 2.0,
    );
    for (entity, person) in &people {
        let (col, row) = (person.rank % POPULATION_COLS, person.rank / POPULATION_COLS);
        let pos = origin + Vec2::new(col as f32 * pitch, -(row as f32) * pitch);
        commands
            .entity(entity)
            .insert((Transform::from_translation(pos.extend(0.0)), Visibility::default()));
    }
}

fn rebuild_bars(
    mut commands: Commands,
    source: Res<CatalogSource>,
    catalog: Res<Catalog>,
    people: Query<Entity, With<Person>>,
    mut built: Local<Option<u32>>,
) {
    if *built == Some(source.version) {
        return;
    }
    *built = Some(source.version);
    for entity in &people {
        commands.entity(entity).despawn_related::<Children>().with_children(|bar| {
            for segment in (0..catalog.len()).map(Some).chain([None]) {
                bar.spawn((DaySegment(segment), Sprite::from_color(Color::NONE, Vec2::ZERO), Transform::default()));
            }
        });
    }
}

fn drive_sim(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let years = {
        let mut speed = world.resource_mut::<SimSpeed>();
        if speed.paused {
            return;
        }
        speed.pending += dt * speed.years_per_second;
        let whole = speed.pending.floor();
        speed.pending -= whole;
        (whole as u32).min(10)
    };
    for _ in 0..years {
        world.run_schedule(SimTick);
    }
}

fn srgb(rgb: [u8; 3]) -> Color {
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
}

fn color32(rgb: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

fn draw_days(
    catalog: Res<Catalog>,
    people: Query<(&TimeUse, &Nourishment)>,
    mut segments: Query<(&DaySegment, &ChildOf, &mut Sprite, &mut Transform)>,
) {
    for (segment, child_of, mut sprite, mut transform) in &mut segments {
        let Ok((time, nourishment)) = people.get(child_of.parent()) else {
            continue;
        };
        let (below, hours, color) = match segment.0 {
            Some(good) if good < time.hours.len() => {
                let essential = matches!(catalog.goods[good].want, Want::Essential { .. });
                let color = if essential && nourishment.0 < 0.999 {
                    HUNGRY_COLOR
                } else {
                    catalog.goods[good].color
                };
                (time.hours[..good].iter().sum::<f32>(), time.hours[good], color)
            }
            Some(_) => continue,
            None => (time.hours.iter().sum(), time.leisure, LEISURE_COLOR),
        };
        let height = CELL * hours / WAKING_HOURS;
        let bottom = -CELL / 2.0 + CELL * below / WAKING_HOURS;
        sprite.custom_size = Some(Vec2::new(CELL, height));
        sprite.color = srgb(color);
        transform.translation.y = bottom + height / 2.0;
    }
}

fn swatch(ui: &mut egui::Ui, rgb: [u8; 3], label: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color32(rgb));
    ui.label(label);
}

#[allow(clippy::too_many_arguments)]
fn control_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut catalog: ResMut<Catalog>,
    mut economy: ResMut<Economy>,
    mut speed: ResMut<SimSpeed>,
    source: Res<CatalogSource>,
    clock: Res<Clock>,
    history: Res<History>,
    people: Query<(&LandQuality, &Appetite, &TimeUse, &Stock)>,
    mut camera: Single<&mut Camera, Without<EguiContext>>,
    window: Single<&Window, With<PrimaryWindow>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut root = egui::Ui::new(
        ctx.clone(),
        "root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    let panel = egui::Panel::left("controls")
        .exact_size(PANEL_WIDTH)
        .resizable(false)
        .show(&mut root, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Fields to Factories · stage 2");
                ui.label(format!("Year {}", clock.year));
                ui.add_space(6.0);
                controls(ui, &mut commands, &mut catalog, &mut economy, &mut speed, &source, &history);
                ui.separator();
                readout(ui, &catalog, &history, &people);
                ui.add_space(4.0);
                stacked_chart(ui, &catalog, &history);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Each bar is one person's day, best land top-left. People feed themselves \
                         first, then spend spare hours on whatever is worth most, including tools \
                         that save hours next year.",
                    )
                    .weak(),
                );
            });
        });

    let left = (panel.response.rect.width() * window.scale_factor()) as u32;
    let size = UVec2::new(window.physical_width().saturating_sub(left), window.physical_height());
    camera.viewport = (size.x > 0 && size.y > 0).then(|| Viewport {
        physical_position: UVec2::new(left, 0),
        physical_size: size,
        ..default()
    });
    Ok(())
}

fn controls(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    catalog: &mut Catalog,
    economy: &mut Economy,
    speed: &mut SimSpeed,
    source: &CatalogSource,
    history: &History,
) {
    let names: Vec<String> = catalog.goods.iter().map(|g| g.name.clone()).collect();
    for good in &mut catalog.goods {
        ui.add(
            egui::Slider::new(&mut good.recipe.rate, 0.01..=MAX_RATE)
                .logarithmic(true)
                .text(format!("{} per hour", good.name)),
        );
        if let Some(tool) = &mut good.recipe.tool {
            ui.add(
                egui::Slider::new(&mut tool.multiplier, 1.0..=5.0)
                    .text(format!("× {} with a {}", good.name, names[tool.good])),
            );
        }
    }
    ui.horizontal(|ui| {
        ui.checkbox(&mut economy.auto_growth, "improving goods get better by");
        ui.add(
            egui::DragValue::new(&mut economy.growth_rate)
                .range(0.0..=0.1)
                .speed(0.001)
                .custom_formatter(|v, _| format!("{:.1}%", v * 100.0))
                .custom_parser(|s| s.trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0)),
        );
        ui.label("a year");
    });
    ui.add(egui::Slider::new(&mut speed.years_per_second, 1.0..=60.0).text("years per second"));
    ui.horizontal(|ui| {
        if ui.button(if speed.paused { "▶ Run" } else { "⏸ Pause" }).clicked() {
            speed.paused = !speed.paused;
        }
        if ui
            .button("⟲ Reset")
            .on_hover_text(format!("Back to year 0, reloading {}", source.path.display()))
            .clicked()
        {
            commands.queue(|world: &mut World| reset(world));
        }
        if ui.button("Export CSV").clicked() {
            let written = std::fs::File::create("history.csv")
                .and_then(|mut f| write_csv(catalog, &history.0, &mut f));
            match written {
                Ok(()) => info!("wrote history.csv"),
                Err(e) => error!("could not write history.csv: {e}"),
            }
        }
    });
    if let Some(error) = &source.error {
        ui.colored_label(color32(HUNGRY_COLOR), format!("Kept the old catalog: {error}"));
    }
}

fn readout(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    history: &History,
    people: &Query<(&LandQuality, &Appetite, &TimeUse, &Stock)>,
) {
    let n = people.iter().count() as f32;
    if n == 0.0 {
        return;
    }
    for (id, good) in catalog.goods.iter().enumerate() {
        let Want::Essential { per_appetite, .. } = good.want else {
            continue;
        };
        let (mut needed, mut actual, mut appetite, mut boost) = (0.0, 0.0, 0.0, 0.0);
        for (land, app, time, stock) in people {
            if time.hours.len() != catalog.len() {
                return;
            }
            let rate = catalog.direct_rate(id, land.0, &stock.0);
            needed += (per_appetite * app.0 / rate).min(WAKING_HOURS);
            actual += time.hours[id];
            appetite += per_appetite * app.0;
            boost += catalog.tool_multiplier(id, &stock.0);
        }
        let (needed, actual, appetite, boost) = (needed / n, actual / n, appetite / n, boost / n);
        let tool = good.recipe.tool.map(|t| catalog.goods[t.good].name.as_str());
        let divisor = match tool {
            Some(tool) => format!("({} per hour × {tool} boost)", good.name),
            None => format!("{} per hour", good.name),
        };
        ui.label(egui::RichText::new(format!("hours {} ≈ appetite ÷ {divisor}", good.sector)).strong());
        let rate = match tool {
            Some(_) => format!("({:.3} × {boost:.2})", good.recipe.rate),
            None => format!("{:.3}", good.recipe.rate),
        };
        ui.label(format!(
            "{appetite:.2} ÷ {rate} = {needed:.1} h to have enough, plus {:.1} h for better {}",
            (actual - needed).max(0.0),
            good.name
        ));
    }

    let Some(last) = history.0.last() else {
        return;
    };
    for (id, good) in catalog.goods.iter().enumerate() {
        if good.depreciation.is_some() {
            ui.label(format!("{:.0}% of people have a working {}", last.coverage[id] * 100.0, good.name));
        }
    }
    let shares: Vec<String> = catalog
        .goods
        .iter()
        .zip(&last.shares)
        .map(|(g, s)| format!("{} {:.0}%", g.sector, s * 100.0))
        .chain([
            format!("leisure {:.0}%", last.leisure_share * 100.0),
            format!("hungry {:.0}% of people", last.hungry_share * 100.0),
        ])
        .collect();
    ui.label(shares.join(" · "));

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for good in &catalog.goods {
            swatch(ui, good.color, &good.sector);
        }
        swatch(ui, LEISURE_COLOR, "leisure");
        swatch(ui, HUNGRY_COLOR, "hungry");
        swatch(ui, COVERAGE_COLOR, "tool owners");
    });
}

fn stacked_chart(ui: &mut egui::Ui, catalog: &Catalog, history: &History) {
    let samples = &history.0;
    let xs: Vec<f64> = samples.iter().map(|s| s.year as f64).collect();
    let mut lower = vec![0.0; samples.len()];
    let mut bands = Vec::new();
    for (id, good) in catalog.goods.iter().enumerate() {
        let upper: Vec<f64> = samples
            .iter()
            .zip(&lower)
            .map(|(s, l)| l + s.shares.get(id).copied().unwrap_or(0.0) as f64 * 100.0)
            .collect();
        bands.push((good.sector.clone(), good.color, lower, upper.clone()));
        lower = upper;
    }
    bands.push(("leisure".into(), LEISURE_COLOR, lower, vec![100.0; samples.len()]));

    let hungry: PlotPoints = samples
        .iter()
        .map(|s| [s.year as f64, s.hungry_share as f64 * 100.0])
        .collect();
    let coverage: Vec<(String, PlotPoints)> = catalog
        .goods
        .iter()
        .enumerate()
        .filter(|(_, g)| g.depreciation.is_some())
        .map(|(id, g)| {
            let points = samples
                .iter()
                .map(|s| [s.year as f64, s.coverage.get(id).copied().unwrap_or(0.0) as f64 * 100.0])
                .collect();
            (format!("% with a {}", g.name), points)
        })
        .collect();

    Plot::new("time_use")
        .height(300.0)
        .include_y(0.0)
        .include_y(100.0)
        .include_x(0.0)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .x_axis_label("year")
        .y_axis_label("% of waking hours")
        .show(ui, |plot| {
            for (name, color, lower, upper) in &bands {
                plot.add(FilledArea::new(name.clone(), &xs, lower, upper).fill_color(color32(*color)));
            }
            plot.line(Line::new("hungry (% of people)", hungry).color(color32(HUNGRY_COLOR)).width(2.0));
            for (name, points) in coverage {
                plot.line(Line::new(name, points).color(color32(COVERAGE_COLOR)).width(1.5));
            }
        });
}
