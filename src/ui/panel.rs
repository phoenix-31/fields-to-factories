//! The egui layout: a top bar for running the simulation, a tabbed side panel, a map key
//! overlay, hover details for each person, and a "How it works" window.

use bevy::camera::Viewport;
use bevy::ecs::query::QueryData;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContext, EguiContexts, egui};

use super::charts::{people_chart, production_tree, sector_legend, stacked_chart};
use super::{
    CELL, Expanded, GAP, HUNGRY_COLOR, LEISURE_COLOR, MapView, PANEL_WIDTH, ShownTown, SimSpeed, TRADING_COLOR, color32,
    lighten, swatch,
};
use crate::goods::{Catalog, GoodId};
use crate::map::WorldMap;
use crate::market::Markets;
use crate::people::Population;
use crate::sim::*;

#[derive(Default, PartialEq, Clone, Copy)]
pub(super) enum Tab {
    #[default]
    Overview,
    Production,
    Towns,
    Settings,
}

#[derive(Resource, Default)]
pub(super) struct PanelState {
    tab: Tab,
    help: bool,
}

#[derive(SystemParam)]
pub(super) struct Model<'w> {
    catalog: ResMut<'w, Catalog>,
    economy: ResMut<'w, Economy>,
    markets: ResMut<'w, Markets>,
    population: ResMut<'w, Population>,
    world_map: ResMut<'w, WorldMap>,
    definitions: Res<'w, Definitions>,
    clock: Res<'w, Clock>,
    history: Res<'w, History>,
}

#[derive(SystemParam)]
pub(super) struct View<'w> {
    speed: ResMut<'w, SimSpeed>,
    shown: ResMut<'w, ShownTown>,
    map_view: ResMut<'w, MapView>,
    expanded: ResMut<'w, Expanded>,
    state: ResMut<'w, PanelState>,
}

#[derive(QueryData)]
pub(super) struct PersonView {
    person: &'static Person,
    home: &'static Home,
    endowments: &'static Endowments,
    needs: &'static Needs,
    time: &'static TimeUse,
    tools: &'static Tools,
    wage: &'static Wage,
    job: &'static Job,
    needs_met: &'static NeedsMet,
    moves: &'static Moves,
    crowding: &'static Crowding,
}

pub(super) fn layout(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut model: Model,
    mut view: View,
    people: Query<PersonView>,
    mut camera: Single<(&mut Camera, &GlobalTransform), Without<EguiContext>>,
    window: Single<&Window, With<PrimaryWindow>>,
) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    let mut root = egui::Ui::new(
        ctx.clone(),
        "root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::top("top_bar").show(&mut root, |ui| top_bar(ui, &mut commands, &model, &mut view));
    egui::Panel::left("side")
        .exact_size(PANEL_WIDTH)
        .resizable(false)
        .show(&mut root, |ui| side_panel(ui, &mut commands, &mut model, &mut view, &people));

    let map_rect = root.available_rect_before_wrap();
    let scale = window.scale_factor();
    let position = (map_rect.min.to_vec2() * scale).max(egui::Vec2::ZERO);
    let size = map_rect.size() * scale;
    camera.0.viewport = (size.x >= 1.0 && size.y >= 1.0).then(|| Viewport {
        physical_position: UVec2::new(position.x as u32, position.y as u32),
        physical_size: UVec2::new(size.x as u32, size.y as u32),
        ..default()
    });

    let key = map_key(&ctx, map_rect, &model.catalog, &mut view.map_view, &mut view.expanded);
    if view.state.help {
        how_it_works(&ctx, &mut view.state.help);
    }
    if let Some(pointer) = ctx.pointer_hover_pos()
        && map_rect.contains(pointer)
        && !key.contains(pointer)
        && !(view.state.help && ctx.is_pointer_over_egui())
    {
        if let Ok(world) = camera.0.viewport_to_world_2d(camera.1, Vec2::new(pointer.x, pointer.y))
            && let Some(at) = hover_details(&ctx, pointer, world, &model, &people)
            && let Ok(mark) = camera.0.world_to_viewport(camera.1, at.extend(0.0))
        {
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "hovered".into()))
                .circle_stroke(egui::pos2(mark.x, mark.y), 9.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
        }
    }
    Ok(())
}

fn top_bar(ui: &mut egui::Ui, commands: &mut Commands, model: &Model, view: &mut View) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Fields to Factories").size(18.0).strong());
        ui.separator();
        ui.label(egui::RichText::new(format!("Year {}", model.clock.year)).size(16.0).monospace());
        ui.separator();

        let speed = &mut view.speed;
        if ui
            .button(if speed.paused { "▶ Run" } else { "⏸ Pause" })
            .on_hover_text("Start or stop the clock. Each tick is one year.")
            .clicked()
        {
            speed.paused = !speed.paused;
        }
        if ui.button("+1 year").on_hover_text("Advance exactly one year, useful while paused.").clicked() {
            commands.queue(|world: &mut World| world.run_schedule(SimTick));
        }
        ui.add(egui::Slider::new(&mut speed.years_per_second, 1.0..=30.0).suffix(" yrs/s").show_value(true))
            .on_hover_text("How many simulated years pass each real second.");
        ui.separator();
        if ui
            .button("⟲ Reset")
            .on_hover_text(format!(
                "Back to year 0. Reloads {}, {} and {}, so edits to them take effect.",
                model.definitions.goods_path.display(),
                model.definitions.people_path.display(),
                model.definitions.map_path.display()
            ))
            .clicked()
        {
            commands.queue(|world: &mut World| reset(world));
        }
        if ui.button("Export CSV").on_hover_text("Write every year so far to history.csv in the working folder.").clicked() {
            let towns: Vec<String> = model.markets.towns.iter().map(|t| t.name.clone()).collect();
            let written = std::fs::File::create("history.csv")
                .and_then(|mut f| write_csv(&model.catalog, &towns, &model.history.0, &mut f));
            match written {
                Ok(()) => info!("wrote history.csv"),
                Err(e) => error!("could not write history.csv: {e}"),
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.toggle_value(&mut view.state.help, "? How it works");
        });
    });
    ui.add_space(4.0);
}

fn side_panel(ui: &mut egui::Ui, commands: &mut Commands, model: &mut Model, view: &mut View, people: &Query<PersonView>) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        for (tab, label, tip) in [
            (Tab::Overview, "Overview", "Headline numbers and how the day is spent over time"),
            (Tab::Production, "Production", "What's made from what, and prices in each town"),
            (Tab::Towns, "Towns", "How each market town and its surroundings make a living"),
            (Tab::Settings, "Settings", "Technology, trade and recipe controls"),
        ] {
            ui.selectable_value(&mut view.state.tab, tab, egui::RichText::new(label).size(15.0)).on_hover_text(tip);
        }
    });
    ui.separator();
    if let Some(error) = &model.definitions.error {
        ui.colored_label(color32(HUNGRY_COLOR), format!("Reset kept the old definitions: {error}"));
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match view.state.tab {
        Tab::Overview => overview(ui, model, people, &mut view.expanded),
        Tab::Production => production(ui, model, view),
        Tab::Towns => towns(ui, model, view, people),
        Tab::Settings => settings(ui, commands, model),
    });
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).weak());
}

fn stat(ui: &mut egui::Ui, value: String, label: &str, tip: &str) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_min_width(PANEL_WIDTH / 2.0 - 52.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(value).size(22.0).strong());
                ui.label(egui::RichText::new(label).weak());
            });
        })
        .response
        .on_hover_text(tip);
}

fn overview(ui: &mut egui::Ui, model: &Model, people: &Query<PersonView>, expanded: &mut Expanded) {
    note(
        ui,
        &format!(
            "{} people scattered over a map split their 16 waking hours between making things, \
             trading, and leisure. As tools and technology improve, farming takes less of the day; \
             the question is where the freed time goes.",
            people.iter().count()
        ),
    );
    ui.add_space(6.0);
    let Some(last) = model.history.0.last() else {
        note(ui, "Press Run to start.");
        return;
    };
    let staples = staples(model);
    let pct = |v: f32| format!("{:.0}%", v * 100.0);
    egui::Grid::new("stats").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
        if !staples.is_empty() {
            let share: f32 = staples.iter().map(|&g| last.shares[g]).sum();
            let sectors: Vec<&str> = staples.iter().map(|&g| model.catalog.goods[g].sector.as_str()).collect();
            stat(
                ui,
                pct(share),
                &format!("of the day spent {}", sectors.join(" and ")),
                "Share of everyone's waking hours spent making the goods that meet the first need, including what's made to sell.",
            );
        }
        stat(ui, pct(last.leisure_share), "of the day is leisure", "Share of everyone's waking hours not spent working, trading or carrying.");
        ui.end_row();
        stat(ui, pct(last.hungry_share), "can't meet their needs", "Share of people who couldn't get enough nourishment or warmth this year, even working every hour.");
        stat(ui, pct(last.specialisation), "specialisation", "On average, the share of a person's working hours spent in their single biggest sector. 100% means everyone does one thing.");
        ui.end_row();
        stat(ui, pct(last.traded), "of work is sold", "Value of goods sold at market, in hours of work, as a share of all hours worked.");
        stat(ui, pct(last.trading_share), "spent trading & carrying", "Share of waking hours spent at market and carrying goods to and from town.");
        ui.end_row();
        stat(ui, format!("{:.1}%", last.moved * 100.0), "moved home this year", "Share of people who moved to a place where meeting their needs takes fewer hours.");
        ui.end_row();
    });
    ui.add_space(4.0);
    let unmatched = if last.traded > 0.01 { pct(last.mismatch) } else { "–".into() };
    ui.label(format!("Unmatched trade: {unmatched} · carried between towns: {}", pct(last.carried)))
        .on_hover_text(
            "Unmatched: goods bought with no seller, or sold with no buyer, this year. Merchants absorb the \
             gap and prices move to close it.\nCarried between towns: share of trade that merchants shipped \
             from one town to another.",
        );

    ui.add_space(10.0);
    ui.heading("Where the day goes");
    note(
        ui,
        "Everyone's waking hours, stacked to 100%. Click a band to split it into its parts, and a part to \
         group it again.",
    );
    stacked_chart(ui, &model.catalog, &model.history, expanded);

    ui.add_space(10.0);
    ui.heading("How people are doing");
    note(ui, "Shares of people: those who can't meet their needs, and those owning each tool. Specialisation is the share of a person's work in their biggest sector, on average.");
    people_chart(ui, &model.catalog, &model.history);

    ui.add_space(6.0);
    egui::CollapsingHeader::new("Why farming shrinks").show(ui, |ui| farming_equation(ui, model, people));
}

fn staples(model: &Model) -> Vec<GoodId> {
    let catalog = &model.catalog;
    model.population.needs.first().map_or(Vec::new(), |&(c, _)| {
        (0..catalog.len()).filter(|&g| catalog.provided(g, c) > 0.0).collect()
    })
}

fn farming_equation(ui: &mut egui::Ui, model: &Model, people: &Query<PersonView>) {
    let catalog = &model.catalog;
    let n = people.iter().count() as f32;
    for &(c, _) in &model.population.needs {
        let Some(id) = (0..catalog.len())
            .filter(|&g| catalog.provided(g, c) > 0.0 && catalog.goods[g].recipe.inputs.is_empty())
            .max_by(|&a, &b| {
                let per_hour = |g: GoodId| catalog.provided(g, c) * catalog.goods[g].recipe.rate;
                per_hour(a).total_cmp(&per_hour(b))
            })
        else {
            continue;
        };
        let good = &catalog.goods[id];
        let (mut needed, mut actual, mut amount, mut boost) = (0.0, 0.0, 0.0, 0.0);
        for p in people {
            if p.time.hours.len() != catalog.len() || p.tools.years_left.len() != catalog.len() {
                return;
            }
            let owned = p.tools.owned();
            let need: f32 = p.needs.0.iter().filter(|x| x.0 == c).map(|x| x.1).sum();
            let rate = catalog.provided(id, c) * catalog.direct_rate(id, &p.endowments.0, &owned);
            needed += (need / rate).min(WAKING_HOURS);
            actual += p.time.hours[id];
            amount += need;
            boost += catalog.tool_multiplier(id, &owned);
        }
        let (needed, actual, amount, boost) = (needed / n, actual / n, amount / n, boost / n);
        let characteristic = &catalog.characteristics[c];
        ui.label(egui::RichText::new(format!("hours {} ≈ {characteristic} needed ÷ ({} per hour × tool boost)", good.sector, good.name)).strong());
        ui.label(format!(
            "{amount:.2} ÷ ({:.3} × {boost:.2}) = {needed:.1} h a day to meet the need on your own land, \
             against {actual:.1} h actually spent (some farm for others, some buy food).",
            good.recipe.rate
        ));
        note(ui, "The need doesn't grow, so every gain in food per hour or in tools frees hours for everything else.");
    }
}

fn production(ui: &mut egui::Ui, model: &Model, view: &mut View) {
    note(
        ui,
        "Raw materials on the left, what people consume on the right. Solid arrows are inputs; \
         dashed arrows are tools that make a recipe faster. Each box shows the share of everyone's \
         hours spent making that good, and its price in the chosen town.",
    );
    ui.add_space(6.0);
    let towns = &model.markets.towns;
    if towns.is_empty() {
        return;
    }
    view.shown.0 = view.shown.0.min(towns.len() - 1);
    ui.horizontal(|ui| {
        ui.label("Prices at");
        egui::ComboBox::from_id_salt("town")
            .selected_text(&towns[view.shown.0].name)
            .show_ui(ui, |ui| {
                for (t, town) in towns.iter().enumerate() {
                    ui.selectable_value(&mut view.shown.0, t, &town.name);
                }
            })
            .response
            .on_hover_text("Prices differ by town because goods cost time to carry.");
    });
    ui.add_space(4.0);
    production_tree(ui, &model.catalog, &towns[view.shown.0].prices, &model.history);
    note(ui, "Prices are in hours of an average person's work.");

    if let Some(last) = model.history.0.last() {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Tools in use").strong());
        for (id, good) in model.catalog.goods.iter().enumerate().filter(|(_, g)| g.is_tool()) {
            ui.colored_label(color32(lighten(good.color)), format!("{:.0}% of people own a working {}", last.coverage[id] * 100.0, good.name))
                .on_hover_text("Tools last a random number of years, then break and must be replaced.");
        }
    }
}

fn towns(ui: &mut egui::Ui, model: &Model, view: &mut View, people: &Query<PersonView>) {
    note(
        ui,
        "Everyone trades at their nearest town. Wages are what an hour of someone's job earns, in \
         hours of average work, so 1.0 is average. Select a town to show its prices on the \
         Production tab.",
    );
    ui.add_space(6.0);
    let catalog = &model.catalog;
    let food = staples(model).first().copied();
    egui::Grid::new("towns").striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
        ui.label(egui::RichText::new("Town").strong());
        ui.label(egui::RichText::new("People").strong()).on_hover_text("People whose nearest town this is.");
        ui.label(egui::RichText::new("Wage").strong()).on_hover_text("Average wage of people here who sell at market.");
        ui.label(egui::RichText::new("Main work").strong()).on_hover_text("The two sectors taking most working hours here.");
        if let Some(food) = food {
            let name = &catalog.goods[food].name;
            let mut title = name.clone();
            title[..1].make_ascii_uppercase();
            ui.label(egui::RichText::new(title).strong()).on_hover_text(format!("Price of {name} here, in hours of average work."));
        }
        ui.end_row();
        for (t, town) in model.markets.towns.iter().enumerate() {
            let (mut count, mut wage, mut traders) = (0usize, 0.0, 0.0);
            let mut hours = vec![0.0; catalog.len()];
            for p in people.iter().filter(|p| p.home.town == t && p.time.hours.len() == catalog.len()) {
                count += 1;
                if p.wage.0 > 0.0 {
                    wage += p.wage.0;
                    traders += 1.0;
                }
                for (h, x) in hours.iter_mut().zip(&p.time.hours) {
                    *h += x;
                }
            }
            let total: f32 = hours.iter().sum::<f32>().max(1e-6);
            let mut order: Vec<GoodId> = (0..catalog.len()).collect();
            order.sort_by(|&a, &b| hours[b].total_cmp(&hours[a]));
            let main = order
                .iter()
                .take(2)
                .map(|&g| format!("{} {:.0}%", catalog.goods[g].sector, hours[g] / total * 100.0))
                .collect::<Vec<_>>()
                .join("\n");
            ui.radio_value(&mut view.shown.0, t, &town.name);
            ui.label(count.to_string());
            ui.label(if traders > 0.0 { format!("{:.2}", wage / traders) } else { "–".into() });
            ui.label(main);
            if let Some(food) = food {
                ui.label(format!("{:.2} h", town.prices[food]));
            }
            ui.end_row();
        }
    });
}

fn settings(ui: &mut egui::Ui, commands: &mut Commands, model: &mut Model) {
    let percent = |v: f64, _| format!("{:.0}%", v * 100.0);
    let parse_percent = |s: &str| s.trim_end_matches('%').trim().parse::<f64>().ok().map(|v| v / 100.0);

    ui.heading("Technology");
    note(ui, "Each recipe improves at its own yearly rate, set in goods.ron. This scales all of them.");
    ui.horizontal(|ui| {
        ui.checkbox(&mut model.economy.progress, "Technology improves")
            .on_hover_text("Untick to freeze every recipe at today's rates.");
        ui.add(egui::DragValue::new(&mut model.economy.tech_speed).range(0.0..=5.0).speed(0.02).suffix("× normal speed"))
            .on_hover_text("1× uses the rates in goods.ron; 2× doubles every rate of improvement.");
    });

    ui.add_space(10.0);
    ui.heading("Trade");
    note(ui, "What it costs to use a market, paid in time rather than money.");
    ui.add(
        egui::Slider::new(&mut model.markets.trade_cost, 0.0..=2.0)
            .custom_formatter(percent)
            .custom_parser(parse_percent)
            .text("time at market"),
    )
    .on_hover_text("Hours spent bargaining and waiting, as a share of the price. 0% is free trade; 200% makes trade pointless.");
    ui.add(egui::Slider::new(&mut model.markets.travel_cost, 0.0..=0.2).text("carrying cost"))
        .on_hover_text("Hours to carry one unit of bulk one cell. Heavier goods (higher bulk) cost more to move.");

    ui.add_space(10.0);
    ui.heading("Recipes");
    note(ui, "Live changes to how fast each good is made, and how much each tool helps. Reset restores goods.ron.");
    let names: Vec<String> = model.catalog.goods.iter().map(|g| g.name.clone()).collect();
    for good in &mut model.catalog.goods {
        egui::CollapsingHeader::new(format!("{} ({})", good.name, good.sector)).show(ui, |ui| {
            ui.add(egui::Slider::new(&mut good.recipe.rate, 0.01..=MAX_RATE).logarithmic(true).text("per hour"))
                .on_hover_text("Units made per hour of work, before land quality and tools.");
            for tool in &mut good.recipe.tools {
                ui.add(egui::Slider::new(&mut tool.multiplier, 1.0..=5.0).text(format!("× with a {}", names[tool.good])))
                    .on_hover_text("How many times faster the work goes with this tool.");
            }
        });
    }

    ui.add_space(10.0);
    ui.heading("Migration");
    note(
        ui,
        "Each year some people look at other places and move if they could meet their needs in fewer \
         hours there. Crowding stops everyone piling into the best valley.",
    );
    let migration = &mut model.population.migration;
    ui.add(egui::Slider::new(&mut migration.chance, 0.0..=0.5).custom_formatter(percent).custom_parser(parse_percent).text("look each year"))
        .on_hover_text("Share of people who consider moving each year. 0% turns migration off.");
    ui.add(egui::Slider::new(&mut migration.look_at, 1..=20).text("places compared"))
        .on_hover_text("How many random places each of them looks at, drawn from the settlement pattern.");
    ui.add(egui::Slider::new(&mut migration.min_gain, 0.0..=0.5).custom_formatter(percent).custom_parser(parse_percent).text("gain needed to move"))
        .on_hover_text("How much fewer hours a new place must need to meet their needs before they bother moving.");
    ui.add(egui::Slider::new(&mut model.world_map.crowding, 0.0..=4.0).text("crowding"))
        .on_hover_text(
            "How fast resources thin out when more people live round a spot than the settlement pattern \
             expects. 0 means no limit; at 1, twice as crowded leaves each person two thirds as much.",
        );

    ui.add_space(10.0);
    ui.heading("People");
    note(
        ui,
        "How many people there are, and where they first settle, is set in people.ron (count) and map.ron \
         (settlement). Changes apply on Reset; --people-count overrides the count from the command line.",
    );

    ui.add_space(10.0);
    ui.heading("Definition files");
    note(ui, "Edit these and press Reset to reload them without recompiling.");
    for (path, what) in [
        (&model.definitions.goods_path, "goods and recipes"),
        (&model.definitions.people_path, "needs, tastes and resource ranges"),
        (&model.definitions.map_path, "terrain, towns and carrying cost"),
    ] {
        ui.label(format!("{} — {what}", path.display()));
    }
    if ui.button("⟲ Reset and reload").clicked() {
        commands.queue(|world: &mut World| reset(world));
    }
}

fn map_key(
    ctx: &egui::Context,
    map_rect: egui::Rect,
    catalog: &Catalog,
    map_view: &mut MapView,
    expanded: &mut Expanded,
) -> egui::Rect {
    egui::Area::new("map_key".into())
        .fixed_pos(map_rect.min + egui::vec2(12.0, 12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(300.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Map").strong());
                    ui.selectable_value(map_view, MapView::Days, "Each day")
                        .on_hover_text("Each person as a bar of their day: work at the bottom, then trading, then leisure.");
                    ui.selectable_value(map_view, MapView::MainWork, "Main work")
                        .on_hover_text("Each person coloured by the sector they spend most hours in.");
                });
                egui::CollapsingHeader::new("Colour key").default_open(false).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        sector_legend(ui, catalog, expanded);
                        if *map_view == MapView::Days {
                            swatch(ui, TRADING_COLOR, "trading & carrying");
                            swatch(ui, LEISURE_COLOR, "leisure");
                            swatch(ui, HUNGRY_COLOR, "needs unmet");
                        }
                    });
                    note(ui, "Click a group with an arrow to split it up; the chart does the same.");
                    note(ui, "Ground: yellow-green valleys, dark green forest, grey hills. White diamonds are market towns.");
                });
                note(ui, "Hover over a person for details.");
            });
        })
        .response
        .rect
}

fn hover_details(
    ctx: &egui::Context,
    pointer: egui::Pos2,
    world: Vec2,
    model: &Model,
    people: &Query<PersonView>,
) -> Option<Vec2> {
    let pitch = CELL + GAP;
    let x = world.x / pitch + (POPULATION_COLS as f32 - 1.0) / 2.0;
    let y = (POPULATION_ROWS as f32 - 1.0) / 2.0 - world.y / pitch;
    let p = people
        .iter()
        .map(|p| (p.person.x - x).powi(2) + (p.person.y - y).powi(2))
        .zip(people.iter())
        .filter(|(d, _)| *d < 0.4)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)?;
    let catalog = &model.catalog;
    if p.time.hours.len() != catalog.len() {
        return None;
    }
    egui::Area::new("person".into())
        .fixed_pos(pointer + egui::vec2(18.0, 18.0))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(280.0);
                let town = model.markets.towns.get(p.home.town).map_or("?", |t| t.name.as_str());
                ui.label(egui::RichText::new(format!("Person at {:.1}, {:.1}", p.person.x, p.person.y)).strong());
                ui.label(format!("Trades at {town}, {:.1} cells away", p.home.distance));
                let resources: Vec<String> = catalog
                    .resources
                    .iter()
                    .zip(&p.endowments.0)
                    .map(|(name, q)| format!("{name} {q:.2}"))
                    .collect();
                ui.label(format!("Resources: {}", resources.join(" · ")));
                if p.crowding.0 < 0.995 {
                    ui.label(format!("Crowded: resources at {:.0}%", p.crowding.0 * 100.0));
                }
                if p.moves.0 > 0 {
                    ui.label(format!("Moved {} time{}", p.moves.0, if p.moves.0 == 1 { "" } else { "s" }));
                }
                match p.job.0 {
                    Some(job) => ui.label(format!("Sells {} · wage {:.2}", catalog.goods[job].name, p.wage.0)),
                    None => ui.label("Self-sufficient: not yet trading"),
                };
                if p.needs_met.0 < 0.999 {
                    ui.colored_label(color32(HUNGRY_COLOR), format!("Needs unmet: only {:.0}% met", p.needs_met.0 * 100.0));
                }
                ui.separator();
                let mut day: Vec<(String, f32)> = catalog
                    .goods
                    .iter()
                    .zip(&p.time.hours)
                    .filter(|(_, h)| **h >= 0.05)
                    .map(|(g, h)| (g.sector.clone(), *h))
                    .collect();
                day.sort_by(|a, b| b.1.total_cmp(&a.1));
                day.push(("trading & carrying".into(), p.time.trading));
                day.push(("leisure".into(), p.time.leisure));
                egui::Grid::new("day").spacing([12.0, 1.0]).show(ui, |ui| {
                    for (label, hours) in day {
                        ui.label(label);
                        ui.label(format!("{hours:.1} h"));
                        ui.end_row();
                    }
                });
                let tools: Vec<String> = catalog
                    .goods
                    .iter()
                    .zip(&p.tools.years_left)
                    .filter(|(g, y)| g.is_tool() && **y > 0.0)
                    .map(|(g, y)| format!("{} ({:.0} years left)", g.name, y))
                    .collect();
                if !tools.is_empty() {
                    ui.separator();
                    ui.label(format!("Tools: {}", tools.join(", ")));
                }
            });
        });
    Some(super::map_position(p.person.x, p.person.y))
}

fn how_it_works(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("How it works")
        .open(open)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_width(440.0)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                ui.heading("The question");
                ui.label(
                    "In the past most waking hours went on getting food. Today almost none do. This \
                     simulation grows that shift from simple rules, and shows where the freed time goes.",
                );
                ui.add_space(6.0);
                ui.heading("Each person, each year");
                ui.label("• Meets their needs first: enough nourishment, then warmth.");
                ui.label("• Spends spare quarter-hours on whatever their own tastes value most: better food, clothes, furniture, leisure, or tools that save time later.");
                ui.label("• Makes or buys each good, whichever costs them fewer hours, given their land, pasture, forest and ore.");
                ui.label("• Earns money at one job, the good they sell, and now and then reconsiders it.");
                ui.label("• Now and then looks at other places and moves if meeting their needs there would take clearly fewer hours.");
                ui.add_space(6.0);
                ui.heading("Markets and towns");
                ui.label(
                    "Everyone trades at their nearest town. Buying costs time at market and time to carry \
                     goods home; heavy goods cost more to carry. Each town's prices rise when more is \
                     wanted than offered, and merchants ship goods between towns when the price gap \
                     pays for the trip.",
                );
                ui.add_space(6.0);
                ui.heading("Moving and crowding");
                ui.label(
                    "A place's real wage is how many hours it takes to meet your needs there, buying or \
                     making each good as you would. People drift towards places where that is lower. But \
                     land is shared: where more people live than the land supports, each gets less of it, \
                     so the best valleys fill up rather than taking everyone.",
                );
                ui.add_space(6.0);
                ui.heading("Reading the map");
                ui.label(
                    "Each small mark is one person, placed where they live; people settle more densely \
                     on good farmland and near towns. In \"Each day\" view, their mark is a bar of their 16 \
                     waking hours, work at the bottom and leisure at the top. In \"Main work\" view, it \
                     takes the colour of their biggest sector, so regions stand out. Hover over anyone \
                     for details.",
                );
                ui.add_space(6.0);
                ui.heading("Changing the world");
                ui.label(
                    "goods.ron, people.ron and map.ron define everything: recipes, tastes, terrain and \
                     towns. Edit them and press Reset. The Settings tab changes things live.",
                );
            });
        });
}
