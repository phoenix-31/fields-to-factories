//! Windowed front end: the map of people and towns, and the panels around it.

mod charts;
mod panel;

use bevy::camera::{CameraOutputMode, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy_egui::{EguiGlobalSettings, EguiPrimaryContextPass, PrimaryEguiContext, egui};

use crate::goods::{Catalog, GoodId};
use crate::map::WorldMap;
use crate::people::Population;
use crate::sim::*;

const PANEL_WIDTH: f32 = 460.0;
const CELL: f32 = 22.0;
const MARK_WIDTH: f32 = 6.0;
const MARK_HEIGHT: f32 = 13.0;
const GAP: f32 = 6.0;

const LEISURE_COLOR: [u8; 3] = [78, 140, 230];
const TRADING_COLOR: [u8; 3] = [64, 196, 180];
const HUNGRY_COLOR: [u8; 3] = [214, 64, 64];
const SPECIALISATION_COLOR: [u8; 3] = [250, 250, 250];

#[derive(Resource, Default)]
struct ShownTown(usize);

#[derive(Resource, Default, PartialEq, Clone, Copy)]
enum MapView {
    #[default]
    Days,
    MainWork,
}

#[derive(Component)]
struct MapFeature;

#[derive(Resource)]
struct SimSpeed {
    years_per_second: f32,
    paused: bool,
    pending: f32,
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimSpeed { years_per_second: 5.0, paused: false, pending: 0.0 })
            .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
            .init_resource::<ShownTown>()
            .init_resource::<MapView>()
            .init_resource::<panel::PanelState>()
            .add_systems(Startup, setup_cameras)
            .add_systems(Update, (drive_sim, rebuild_bars, move_marks, draw_days).chain())
            .add_systems(EguiPrimaryContextPass, panel::layout);
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
    Good(GoodId),
    Trading,
    Leisure,
}

fn cell_position(col: usize, row: usize) -> Vec2 {
    map_position(col as f32, row as f32)
}

fn map_position(x: f32, y: f32) -> Vec2 {
    let pitch = CELL + GAP;
    Vec2::new(
        (x - (POPULATION_COLS as f32 - 1.0) / 2.0) * pitch,
        ((POPULATION_ROWS as f32 - 1.0) / 2.0 - y) * pitch,
    )
}

fn terrain_color(elevation: f32) -> Color {
    let stops = [(0.0, [96.0, 128.0, 52.0]), (0.5, [36.0, 84.0, 44.0]), (1.0, [130.0, 128.0, 138.0])];
    let (lo, hi) = if elevation < 0.5 { (stops[0], stops[1]) } else { (stops[1], stops[2]) };
    let t = (elevation - lo.0) / (hi.0 - lo.0);
    let c: Vec<f32> = (0..3).map(|i| lo.1[i] + (hi.1[i] - lo.1[i]) * t).collect();
    Color::srgb_u8(c[0] as u8, c[1] as u8, c[2] as u8)
}

#[allow(clippy::too_many_arguments)]
fn rebuild_bars(
    mut commands: Commands,
    definitions: Res<Definitions>,
    catalog: Res<Catalog>,
    world_map: Res<WorldMap>,
    people: Query<(Entity, &Person)>,
    features: Query<Entity, With<MapFeature>>,
    mut shown: ResMut<ShownTown>,
    mut built: Local<Option<u32>>,
) {
    if *built == Some(definitions.version) {
        return;
    }
    *built = Some(definitions.version);
    shown.0 = shown.0.min(world_map.towns.len() - 1);
    for entity in &features {
        commands.entity(entity).despawn();
    }
    for row in 0..POPULATION_ROWS {
        for col in 0..POPULATION_COLS {
            let elevation = world_map.elevation[row * POPULATION_COLS + col];
            commands.spawn((
                MapFeature,
                Sprite::from_color(terrain_color(elevation), Vec2::splat(CELL + GAP)),
                Transform::from_translation(cell_position(col, row).extend(-1.0)),
            ));
        }
    }
    for town in &world_map.towns {
        let at = cell_position(town.at.0, town.at.1);
        commands.spawn((
            MapFeature,
            Sprite::from_color(Color::WHITE, Vec2::splat(12.0)),
            Transform::from_translation(at.extend(2.0)).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        ));
        commands.spawn((
            MapFeature,
            Text2d::new(town.name.clone()),
            TextFont::from_font_size(15.0),
            TextColor(Color::WHITE),
            Transform::from_translation((at + Vec2::new(0.0, CELL)).extend(3.0)),
        ));
    }
    for (entity, person) in &people {
        let pos = map_position(person.x, person.y);
        commands
            .entity(entity)
            .insert((Transform::from_translation(pos.extend(person.y * 0.01)), Visibility::default()))
            .despawn_related::<Children>()
            .with_children(|bar| {
                let segments = (0..catalog.len()).map(DaySegment::Good).chain([DaySegment::Trading, DaySegment::Leisure]);
                for segment in segments {
                    bar.spawn((segment, Sprite::from_color(Color::NONE, Vec2::ZERO), Transform::default()));
                }
            });
    }
}

fn move_marks(mut people: Query<(&Person, &mut Transform), Changed<Person>>) {
    for (person, mut transform) in &mut people {
        transform.translation = map_position(person.x, person.y).extend(person.y * 0.01);
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
    population: Res<Population>,
    view: Res<MapView>,
    people: Query<(&TimeUse, &NeedsMet)>,
    mut segments: Query<(&DaySegment, &ChildOf, &mut Sprite, &mut Transform)>,
) {
    for (segment, child_of, mut sprite, mut transform) in &mut segments {
        let Ok((time, needs_met)) = people.get(child_of.parent()) else {
            continue;
        };
        let worked: f32 = time.hours.iter().sum();
        if *view == MapView::MainWork {
            let main = (0..time.hours.len()).max_by(|&a, &b| time.hours[a].total_cmp(&time.hours[b]));
            let shown = matches!(*segment, DaySegment::Good(g) if Some(g) == main);
            let color = main.map_or(LEISURE_COLOR, |g| catalog.goods[g].color);
            let size = if shown { Vec2::splat(MARK_WIDTH + 2.0) } else { Vec2::ZERO };
            sprite.custom_size = Some(size);
            sprite.color = srgb(color);
            transform.translation.y = 0.0;
            continue;
        }
        let (below, hours, color) = match *segment {
            DaySegment::Good(good) if good < time.hours.len() => {
                let meets_need = population.needs.iter().any(|&(c, _)| catalog.provided(good, c) > 0.0);
                let color = if meets_need && needs_met.0 < 0.999 {
                    HUNGRY_COLOR
                } else {
                    catalog.goods[good].color
                };
                (time.hours[..good].iter().sum::<f32>(), time.hours[good], color)
            }
            DaySegment::Good(_) => continue,
            DaySegment::Trading => (worked, time.trading, TRADING_COLOR),
            DaySegment::Leisure => (worked + time.trading, time.leisure, LEISURE_COLOR),
        };
        let height = MARK_HEIGHT * hours / WAKING_HOURS;
        let bottom = -MARK_HEIGHT / 2.0 + MARK_HEIGHT * below / WAKING_HOURS;
        sprite.custom_size = Some(Vec2::new(MARK_WIDTH, height));
        sprite.color = srgb(color);
        transform.translation.y = bottom + height / 2.0;
    }
}

fn swatch(ui: &mut egui::Ui, rgb: [u8; 3], label: &str) {
    let text_color = ui.visuals().text_color();
    let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(13.0), text_color);
    let size = egui::vec2(16.0 + galley.size().x, galley.size().y.max(12.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let square = egui::Rect::from_min_size(rect.left_center() - egui::vec2(0.0, 6.0), egui::vec2(12.0, 12.0));
    ui.painter().rect_filled(square, 2.0, color32(rgb));
    ui.painter().galley(rect.min + egui::vec2(16.0, 0.0), galley, text_color);
}

fn lighten(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| ((c as u16 + 2 * 255) / 3) as u8)
}

