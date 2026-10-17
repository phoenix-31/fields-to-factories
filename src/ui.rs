//! Windowed front end: people grid, control panel and history chart.

use bevy::camera::{CameraOutputMode, Viewport, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};
use egui_plot::{FilledArea, Line, Plot, PlotPoints};

use crate::sim::*;

const PANEL_WIDTH: f32 = 480.0;
const CELL: f32 = 24.0;
const GAP: f32 = 4.0;

const FARM_COLOR: [u8; 3] = [92, 170, 74];
const CRAFT_COLOR: [u8; 3] = [230, 158, 52];
const LEISURE_COLOR: [u8; 3] = [78, 140, 230];
const HUNGRY_COLOR: [u8; 3] = [214, 64, 64];

#[derive(Resource)]
struct SimSpeed {
    years_per_second: f32,
    paused: bool,
    pending: f32,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimSpeed {
            years_per_second: 15.0,
            paused: false,
            pending: 0.0,
        })
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
        .add_systems(Startup, setup_cameras)
        .add_systems(PostStartup, attach_sprites)
        .add_systems(Update, (drive_sim, draw_days).chain())
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
enum DaySegment {
    Farm,
    Craft,
    Leisure,
}

fn attach_sprites(mut commands: Commands, people: Query<(Entity, &Person)>) {
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
            .insert((Transform::from_translation(pos.extend(0.0)), Visibility::default()))
            .with_children(|bar| {
                for (segment, color) in [
                    (DaySegment::Farm, FARM_COLOR),
                    (DaySegment::Craft, CRAFT_COLOR),
                    (DaySegment::Leisure, LEISURE_COLOR),
                ] {
                    bar.spawn((segment, Sprite::from_color(srgb(color), Vec2::ZERO), Transform::default()));
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

fn draw_days(
    people: Query<(&TimeUse, &Nourishment)>,
    mut segments: Query<(&DaySegment, &ChildOf, &mut Sprite, &mut Transform)>,
) {
    for (segment, child_of, mut sprite, mut transform) in &mut segments {
        let Ok((time, nourishment)) = people.get(child_of.parent()) else {
            continue;
        };
        let (below, hours, color) = match segment {
            DaySegment::Farm => {
                let hungry = nourishment.0 < 0.999;
                (0.0, time.farm, if hungry { HUNGRY_COLOR } else { FARM_COLOR })
            }
            DaySegment::Craft => (time.farm, time.craft, CRAFT_COLOR),
            DaySegment::Leisure => (time.farm + time.craft, time.leisure, LEISURE_COLOR),
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
    ui.painter()
        .rect_filled(rect, 2.0, color32(rgb));
    ui.label(label);
}

fn color32(rgb: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

#[allow(clippy::too_many_arguments)]
fn control_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut economy: ResMut<Economy>,
    mut speed: ResMut<SimSpeed>,
    clock: Res<Clock>,
    history: Res<History>,
    people: Query<(&LandQuality, &Appetite, &TimeUse)>,
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
                panel_contents(ui, &mut commands, &mut economy, &mut speed, &clock, &history, &people)
            });
        });

    let left = (panel.response.rect.width() * window.scale_factor()) as u32;
    let size = UVec2::new(
        window.physical_width().saturating_sub(left),
        window.physical_height(),
    );
    camera.viewport = (size.x > 0 && size.y > 0).then(|| Viewport {
        physical_position: UVec2::new(left, 0),
        physical_size: size,
        ..default()
    });
    Ok(())
}

fn panel_contents(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    economy: &mut Economy,
    speed: &mut SimSpeed,
    clock: &Clock,
    history: &History,
    people: &Query<(&LandQuality, &Appetite, &TimeUse)>,
) {
    ui.heading("Fields to Factories · stage 1");
    ui.label(format!("Year {}", clock.year));
    ui.add_space(6.0);

    ui.add(
        egui::Slider::new(&mut economy.farm_productivity, 0.04..=MAX_FARM_PRODUCTIVITY)
            .logarithmic(true)
            .text("food per hour"),
    );
    ui.add(
        egui::Slider::new(&mut economy.craft_productivity, 0.2..=10.0)
            .logarithmic(true)
            .text("goods per hour"),
    );
    ui.horizontal(|ui| {
        ui.checkbox(&mut economy.auto_growth, "farming improves by");
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
        if ui.button("⟲ Reset").clicked() {
            commands.queue(|world: &mut World| reset(world));
        }
        if ui.button("Export CSV").clicked() {
            match std::fs::File::create("history.csv").map(|mut f| write_csv(&history.0, &mut f)) {
                Ok(Ok(())) => info!("wrote history.csv"),
                Ok(Err(e)) | Err(e) => error!("could not write history.csv: {e}"),
            }
        }
    });

    ui.separator();

    let (mut needed, mut actual, mut appetite, mut n) = (0.0, 0.0, 0.0, 0.0);
    for (land, app, time) in people {
        needed += (app.0 / (economy.farm_productivity * land.0)).min(WAKING_HOURS);
        actual += time.farm;
        appetite += app.0;
        n += 1.0;
    }
    if n > 0.0 {
        let (needed, actual, appetite) = (needed / n, actual / n, appetite / n);
        ui.label(egui::RichText::new("hours farming ≈ appetite ÷ food per hour").strong());
        ui.label(format!(
            "{appetite:.2} ÷ {:.3} = {needed:.1} h to eat enough, plus {:.1} h growing better food",
            economy.farm_productivity,
            (actual - needed).max(0.0)
        ));
        ui.label(format!("Farming takes {:.0}% of the waking day", actual / WAKING_HOURS * 100.0));
    }
    if let Some(last) = history.0.last() {
        ui.label(format!(
            "Crafting {:.0}% · Leisure {:.0}% · Hungry {:.0}% of people",
            last.craft_share * 100.0,
            last.leisure_share * 100.0,
            last.hungry_share * 100.0
        ));
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        swatch(ui, FARM_COLOR, "farming");
        swatch(ui, CRAFT_COLOR, "crafting");
        swatch(ui, LEISURE_COLOR, "leisure");
        swatch(ui, HUNGRY_COLOR, "hungry");
    });

    ui.add_space(4.0);
    stacked_chart(ui, history);

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(
            "Each bar is one person's day, best land top-left. Appetite never grows, \
             so every gain in food per hour frees hours for everything else.",
        )
        .weak(),
    );
}

fn stacked_chart(ui: &mut egui::Ui, history: &History) {
    let samples = &history.0;
    let xs: Vec<f64> = samples.iter().map(|s| s.year as f64).collect();
    let zero = vec![0.0; samples.len()];
    let farm: Vec<f64> = samples.iter().map(|s| s.farm_share as f64 * 100.0).collect();
    let craft: Vec<f64> = samples
        .iter()
        .zip(&farm)
        .map(|(s, f)| f + s.craft_share as f64 * 100.0)
        .collect();
    let hundred = vec![100.0; samples.len()];
    let hungry: PlotPoints = samples
        .iter()
        .map(|s| [s.year as f64, s.hungry_share as f64 * 100.0])
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
            plot.add(FilledArea::new("farming", &xs, &zero, &farm).fill_color(color32(FARM_COLOR)));
            plot.add(FilledArea::new("crafting", &xs, &farm, &craft).fill_color(color32(CRAFT_COLOR)));
            plot.add(
                FilledArea::new("leisure", &xs, &craft, &hundred).fill_color(color32(LEISURE_COLOR)),
            );
            plot.line(Line::new("hungry (% of people)", hungry).color(color32(HUNGRY_COLOR)).width(2.0));
        });
}
