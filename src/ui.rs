//! Windowed front end: people grid, control panel, production tree and history chart.

use bevy::camera::{CameraOutputMode, Viewport, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use bevy_egui::{
    EguiContext, EguiContexts, EguiGlobalSettings, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};
use egui_plot::{FilledArea, Line, Plot, PlotPoints};

use crate::goods::{Catalog, GoodId};
use crate::market::Market;
use crate::people::Population;
use crate::sim::*;

const PANEL_WIDTH: f32 = 480.0;
const CELL: f32 = 24.0;
const GAP: f32 = 4.0;

const LEISURE_COLOR: [u8; 3] = [78, 140, 230];
const TRADING_COLOR: [u8; 3] = [64, 196, 180];
const HUNGRY_COLOR: [u8; 3] = [214, 64, 64];
const SPECIALISATION_COLOR: [u8; 3] = [250, 250, 250];

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
enum DaySegment {
    Good(GoodId),
    Trading,
    Leisure,
}

fn rebuild_bars(
    mut commands: Commands,
    definitions: Res<Definitions>,
    catalog: Res<Catalog>,
    people: Query<(Entity, &Person)>,
    mut built: Local<Option<u32>>,
) {
    if *built == Some(definitions.version) {
        return;
    }
    *built = Some(definitions.version);
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
            .despawn_related::<Children>()
            .with_children(|bar| {
                let segments = (0..catalog.len()).map(DaySegment::Good).chain([DaySegment::Trading, DaySegment::Leisure]);
                for segment in segments {
                    bar.spawn((segment, Sprite::from_color(Color::NONE, Vec2::ZERO), Transform::default()));
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
    population: Res<Population>,
    people: Query<(&TimeUse, &NeedsMet)>,
    mut segments: Query<(&DaySegment, &ChildOf, &mut Sprite, &mut Transform)>,
) {
    for (segment, child_of, mut sprite, mut transform) in &mut segments {
        let Ok((time, needs_met)) = people.get(child_of.parent()) else {
            continue;
        };
        let worked: f32 = time.hours.iter().sum();
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
        let height = CELL * hours / WAKING_HOURS;
        let bottom = -CELL / 2.0 + CELL * below / WAKING_HOURS;
        sprite.custom_size = Some(Vec2::new(CELL, height));
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

#[allow(clippy::too_many_arguments)]
fn control_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut catalog: ResMut<Catalog>,
    mut economy: ResMut<Economy>,
    mut speed: ResMut<SimSpeed>,
    definitions: Res<Definitions>,
    population: Res<Population>,
    clock: Res<Clock>,
    history: Res<History>,
    mut market: ResMut<Market>,
    people: Query<(&Endowments, &Needs, &TimeUse, &Tools)>,
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
                ui.heading("Fields to Factories · stage 4");
                ui.label(format!("Year {}", clock.year));
                ui.add_space(6.0);
                controls(ui, &mut commands, &mut catalog, &mut economy, &mut market, &mut speed, &definitions, &history);
                ui.separator();
                readout(ui, &catalog, &population, &history, &people);
                ui.add_space(6.0);
                production_tree(ui, &catalog, &market, &history);
                ui.add_space(6.0);
                stacked_chart(ui, &catalog, &history);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Each bar is one person's day, best-endowed top-left. People meet their \
                         needs first, then spend spare hours on whatever their own tastes value \
                         most. Every good is made or bought, whichever costs that person fewer hours.",
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

#[allow(clippy::too_many_arguments)]
fn controls(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    catalog: &mut Catalog,
    economy: &mut Economy,
    market: &mut Market,
    speed: &mut SimSpeed,
    definitions: &Definitions,
    history: &History,
) {
    let names: Vec<String> = catalog.goods.iter().map(|g| g.name.clone()).collect();
    egui::CollapsingHeader::new("Production rates and tools").show(ui, |ui| {
        for good in &mut catalog.goods {
            ui.add(
                egui::Slider::new(&mut good.recipe.rate, 0.01..=MAX_RATE)
                    .logarithmic(true)
                    .text(format!("{} per hour", good.name)),
            );
            for tool in &mut good.recipe.tools {
                ui.add(
                    egui::Slider::new(&mut tool.multiplier, 1.0..=5.0)
                        .text(format!("× {} with a {}", good.name, names[tool.good])),
                );
            }
        }
    });
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
    ui.add(
        egui::Slider::new(&mut market.trade_cost, 0.0..=2.0)
            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
            .custom_parser(|s| s.trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0))
            .text("trade cost on top of the price"),
    );
    ui.add(egui::Slider::new(&mut speed.years_per_second, 1.0..=60.0).text("years per second"));
    ui.horizontal(|ui| {
        if ui.button(if speed.paused { "▶ Run" } else { "⏸ Pause" }).clicked() {
            speed.paused = !speed.paused;
        }
        if ui
            .button("⟲ Reset")
            .on_hover_text(format!(
                "Back to year 0, reloading {} and {}",
                definitions.goods_path.display(),
                definitions.people_path.display()
            ))
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
    if let Some(error) = &definitions.error {
        ui.colored_label(color32(HUNGRY_COLOR), format!("Kept the old definitions: {error}"));
    }
}

fn lighten(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| ((c as u16 + 2 * 255) / 3) as u8)
}

fn readout(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    population: &Population,
    history: &History,
    people: &Query<(&Endowments, &Needs, &TimeUse, &Tools)>,
) {
    let n = people.iter().count() as f32;
    if n == 0.0 {
        return;
    }
    for &(c, _) in &population.needs {
        let Some(id) = (0..catalog.len())
            .filter(|&g| catalog.provided(g, c) > 0.0)
            .max_by(|&a, &b| {
                let per_hour = |g: GoodId| catalog.provided(g, c) * catalog.goods[g].recipe.rate;
                per_hour(a).total_cmp(&per_hour(b))
            })
        else {
            continue;
        };
        let good = &catalog.goods[id];
        let (mut needed, mut actual, mut amount, mut boost) = (0.0, 0.0, 0.0, 0.0);
        for (endowments, needs, time, tools) in people {
            if time.hours.len() != catalog.len() || tools.years_left.len() != catalog.len() {
                return;
            }
            let owned = tools.owned();
            let need: f32 = needs.0.iter().filter(|n| n.0 == c).map(|n| n.1).sum();
            let rate = catalog.provided(id, c) * catalog.direct_rate(id, &endowments.0, &owned);
            needed += (need / rate).min(WAKING_HOURS);
            actual += time.hours[id];
            amount += need;
            boost += catalog.tool_multiplier(id, &owned);
        }
        let (needed, actual, amount, boost) = (needed / n, actual / n, amount / n, boost / n);
        let characteristic = &catalog.characteristics[c];
        let has_tools = !good.recipe.tools.is_empty();
        let divisor = if has_tools {
            format!("({} per hour × tool boost)", good.name)
        } else {
            format!("{} per hour", good.name)
        };
        ui.label(
            egui::RichText::new(format!("hours {} ≈ {characteristic} needed ÷ {divisor}", good.sector)).strong(),
        );
        let rate = if has_tools {
            format!("({:.3} × {boost:.2})", good.recipe.rate)
        } else {
            format!("{:.3}", good.recipe.rate)
        };
        ui.label(format!(
            "{amount:.2} ÷ {rate} = {needed:.1} h to meet the need, plus {:.1} h because people enjoy more",
            (actual - needed).max(0.0)
        ));
    }

    let Some(last) = history.0.last() else {
        return;
    };
    ui.horizontal_wrapped(|ui| {
        ui.label("Working tools:");
        for (id, good) in catalog.goods.iter().enumerate() {
            if good.is_tool() {
                ui.colored_label(
                    color32(lighten(good.color)),
                    format!("{} {:.0}%", good.name, last.coverage[id] * 100.0),
                );
            }
        }
    });
    ui.label(format!(
        "Leisure {:.0}% of waking hours · hungry {:.0}% of people",
        last.leisure_share * 100.0,
        last.hungry_share * 100.0
    ));
    let mismatch = if last.traded > 0.01 {
        format!(" · {:.0}% of trade unmatched", last.mismatch * 100.0)
    } else {
        String::new()
    };
    ui.label(format!(
        "Specialisation {:.0}% · {:.0}% of work sold · trading takes {:.0}% of the day{mismatch}",
        last.specialisation * 100.0,
        last.traded * 100.0,
        last.trading_share * 100.0
    ))
    .on_hover_text(
        "Specialisation: on average, the share of a person's working hours spent in their main sector.\n\
         Work sold: value of goods sold, in hours, as a share of all hours worked.\n\
         Unmatched: goods bought with no seller, or sold with no buyer, this year; merchants absorb the gap \
         and prices move to close it.",
    );
}

fn columns(catalog: &Catalog) -> Vec<usize> {
    fn depth(catalog: &Catalog, good: GoodId, memo: &mut [Option<usize>]) -> usize {
        if let Some(d) = memo[good] {
            return d;
        }
        let recipe = &catalog.goods[good].recipe;
        let upstream = recipe.inputs.iter().map(|&(i, _)| i).chain(recipe.tools.iter().map(|t| t.good));
        let d = upstream.map(|u| depth(catalog, u, memo) + 1).max().unwrap_or(0);
        memo[good] = Some(d);
        d
    }
    let mut memo = vec![None; catalog.len()];
    let depths: Vec<usize> = (0..catalog.len()).map(|g| depth(catalog, g, &mut memo)).collect();
    let last = depths.iter().copied().max().unwrap_or(0);

    let users = |good: GoodId| {
        catalog.goods.iter().enumerate().filter(move |(_, other)| {
            other.recipe.inputs.iter().any(|&(i, _)| i == good) || other.recipe.tools.iter().any(|t| t.good == good)
        })
    };
    let mut cols = vec![last; catalog.len()];
    let mut order: Vec<GoodId> = (0..catalog.len()).collect();
    order.sort_by_key(|&g| std::cmp::Reverse(depths[g]));
    for good in order {
        if let Some(col) = users(good).map(|(user, _)| cols[user]).min() {
            cols[good] = col.saturating_sub(1);
        }
    }
    for (good, def) in catalog.goods.iter().enumerate() {
        if def.recipe.inputs.is_empty() && def.recipe.tools.is_empty() && users(good).next().is_some() {
            cols[good] = 0;
        }
    }
    cols
}

fn production_tree(ui: &mut egui::Ui, catalog: &Catalog, market: &Market, history: &History) {
    let cols = columns(catalog);
    let col_count = cols.iter().copied().max().unwrap_or(0) + 1;
    let mut rows = vec![0usize; catalog.len()];
    let mut per_col = vec![0usize; col_count];
    for (good, &col) in cols.iter().enumerate() {
        rows[good] = per_col[col];
        per_col[col] += 1;
    }
    let max_rows = per_col.iter().copied().max().unwrap_or(1);

    let (node_h, row_gap) = (46.0, 12.0);
    let width = ui.available_width();
    let col_w = width / col_count as f32;
    let node_w = (col_w - 16.0).max(40.0);
    let height = max_rows as f32 * (node_h + row_gap);
    let (response, painter) = ui.allocate_painter(egui::vec2(width, height), egui::Sense::hover());
    let origin = response.rect.min;
    let node_rect = |good: GoodId| {
        let col_rows = per_col[cols[good]] as f32;
        let top = (height - col_rows * (node_h + row_gap)) / 2.0 + rows[good] as f32 * (node_h + row_gap);
        egui::Rect::from_min_size(
            origin + egui::vec2(cols[good] as f32 * col_w + (col_w - node_w) / 2.0, top + row_gap / 2.0),
            egui::vec2(node_w, node_h),
        )
    };

    let edge = egui::Color32::from_gray(150);
    for (good, def) in catalog.goods.iter().enumerate() {
        let to = node_rect(good).left_center();
        for &(input, _) in &def.recipe.inputs {
            let from = node_rect(input).right_center();
            painter.line_segment([from, to], egui::Stroke::new(1.2, edge));
            let dir = (to - from).normalized() * 7.0;
            painter.arrow(to - dir, dir, egui::Stroke::new(1.2, edge));
        }
        for tool in &def.recipe.tools {
            let from = node_rect(tool.good).right_center();
            painter.extend(egui::Shape::dashed_line(&[from, to], egui::Stroke::new(1.2, edge), 4.0, 3.0));
            let dir = (to - from).normalized() * 7.0;
            painter.arrow(to - dir, dir, egui::Stroke::new(1.2, edge));
        }
    }

    let shares = history.0.last().map(|s| s.shares.clone()).unwrap_or_default();
    for (good, def) in catalog.goods.iter().enumerate() {
        let rect = node_rect(good);
        painter.rect_filled(rect, 4.0, color32(def.color));
        let [r, g, b] = def.color;
        let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        let text = if luminance > 140.0 { egui::Color32::from_gray(20) } else { egui::Color32::from_gray(240) };
        let name_size = if painter.layout_no_wrap(def.name.clone(), egui::FontId::proportional(11.0), text).size().x
            > node_w - 6.0
        {
            9.0
        } else {
            11.0
        };
        painter.text(
            rect.center() - egui::vec2(0.0, 13.0),
            egui::Align2::CENTER_CENTER,
            &def.name,
            egui::FontId::proportional(name_size),
            text,
        );
        let share = shares.get(good).copied().unwrap_or(0.0) * 100.0;
        painter.text(
            rect.center() + egui::vec2(0.0, 1.0),
            egui::Align2::CENTER_CENTER,
            format!("{share:.1}% of hours"),
            egui::FontId::proportional(10.0),
            text,
        );
        let price = market.prices.get(good).copied().unwrap_or(0.0);
        painter.text(
            rect.center() + egui::vec2(0.0, 14.0),
            egui::Align2::CENTER_CENTER,
            format!("price {price:.2} h"),
            egui::FontId::proportional(10.0),
            text,
        );
    }
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
    let trading: Vec<f64> = samples
        .iter()
        .zip(&lower)
        .map(|(s, l)| l + s.trading_share as f64 * 100.0)
        .collect();
    bands.push(("trading".into(), TRADING_COLOR, lower, trading.clone()));
    bands.push(("leisure".into(), LEISURE_COLOR, trading, vec![100.0; samples.len()]));

    let hungry: PlotPoints = samples
        .iter()
        .map(|s| [s.year as f64, s.hungry_share as f64 * 100.0])
        .collect();
    let specialisation: PlotPoints = samples
        .iter()
        .map(|s| [s.year as f64, s.specialisation as f64 * 100.0])
        .collect();
    let coverage: Vec<(String, [u8; 3], PlotPoints)> = catalog
        .goods
        .iter()
        .enumerate()
        .filter(|(_, g)| g.is_tool())
        .map(|(id, g)| {
            let points = samples
                .iter()
                .map(|s| [s.year as f64, s.coverage.get(id).copied().unwrap_or(0.0) as f64 * 100.0])
                .collect();
            (format!("% with a {}", g.name), lighten(g.color), points)
        })
        .collect();

    Plot::new("time_use")
        .height(260.0)
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
            for (name, color, points) in coverage {
                plot.line(Line::new(name, points).color(color32(color)).width(2.0));
            }
            plot.line(
                Line::new("specialisation (%)", specialisation)
                    .color(color32(SPECIALISATION_COLOR))
                    .style(egui_plot::LineStyle::dashed_loose())
                    .width(2.0),
            );
        });

    ui.horizontal_wrapped(|ui| {
        for good in &catalog.goods {
            swatch(ui, good.color, &good.sector);
        }
        swatch(ui, TRADING_COLOR, "trading");
        swatch(ui, LEISURE_COLOR, "leisure");
        swatch(ui, HUNGRY_COLOR, "hungry");
        swatch(ui, SPECIALISATION_COLOR, "specialisation (dashed)");
    });
}
