//! The production tree diagram and the history chart.

use bevy_egui::egui;
use egui_plot::{FilledArea, HoverPosition, Line, Plot, PlotPoints};

use super::{
    CHILD_COLOR, Expanded, HUNGRY_COLOR, POPULATION_COLOR, LEISURE_COLOR, SPECIALISATION_COLOR, TRADING_COLOR, color32, lighten, sector_swatch, swatch,
    visible_sectors,
};
use crate::goods::{Catalog, GoodId, SectorId};
use crate::sim::History;

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

pub(super) fn production_tree(ui: &mut egui::Ui, catalog: &Catalog, prices: &[f32], history: &History) {
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
        let price = prices.get(good).copied().unwrap_or(0.0);
        painter.text(
            rect.center() + egui::vec2(0.0, 14.0),
            egui::Align2::CENTER_CENTER,
            format!("price {price:.2} h"),
            egui::FontId::proportional(10.0),
            text,
        );
    }
}

struct Band {
    sector: Option<SectorId>,
    name: String,
    color: [u8; 3],
    lower: Vec<f64>,
    upper: Vec<f64>,
}

pub(super) fn stacked_chart(ui: &mut egui::Ui, catalog: &Catalog, history: &History, expanded: &mut Expanded) {
    let samples = &history.0;
    let xs: Vec<f64> = samples.iter().map(|s| s.year as f64).collect();
    let mut lower = vec![0.0; samples.len()];
    let mut bands = Vec::new();
    for sector in visible_sectors(catalog, expanded) {
        let goods = &catalog.sectors[sector].goods;
        let upper: Vec<f64> = samples
            .iter()
            .zip(&lower)
            .map(|(s, l)| l + goods.iter().map(|&g| s.shares.get(g).copied().unwrap_or(0.0) as f64).sum::<f64>() * 100.0)
            .collect();
        let def = &catalog.sectors[sector];
        bands.push(Band { sector: Some(sector), name: def.name.clone(), color: def.color, lower, upper: upper.clone() });
        lower = upper;
    }
    let trading: Vec<f64> = samples
        .iter()
        .zip(&lower)
        .map(|(s, l)| l + s.trading_share as f64 * 100.0)
        .collect();
    bands.push(Band { sector: None, name: "trading & carrying".into(), color: TRADING_COLOR, lower, upper: trading.clone() });
    bands.push(Band {
        sector: None,
        name: "leisure".into(),
        color: LEISURE_COLOR,
        lower: trading,
        upper: vec![100.0; samples.len()],
    });
    let band_at = |x: f64, y: f64| {
        let i = xs.iter().position(|&year| year >= x.round()).unwrap_or(xs.len().saturating_sub(1));
        bands.iter().find(|b| b.lower.get(i).is_some_and(|&l| l <= y) && b.upper.get(i).is_some_and(|&u| y < u)).map(|b| (b, i))
    };

    let plot = year_plot("time_use", "% of waking hours")
        .height(260.0)
        .label_formatter(|hover| {
            let position = match hover {
                HoverPosition::NearDataPoint { position, .. } | HoverPosition::Elsewhere { position } => *position,
            };
            let (band, i) = band_at(position.x, position.y)?;
            let hint = match band.sector.map(|s| &catalog.sectors[s]) {
                Some(s) if !s.children.is_empty() => "\nclick to split it up",
                Some(s) if s.parent.is_some() => "\nclick to group it again",
                _ => "",
            };
            Some(format!("{}\n{:.1}% of hours in year {:.0}{hint}", band.name, band.upper[i] - band.lower[i], xs[i]))
        })
        .show(ui, |plot| {
            for band in &bands {
                plot.add(FilledArea::new(band.name.clone(), &xs, &band.lower, &band.upper).fill_color(color32(band.color)));
            }
        });
    if plot.response.clicked()
        && let Some(pointer) = plot.response.interact_pointer_pos()
    {
        let at = plot.transform.value_from_position(pointer);
        let target = band_at(at.x, at.y).and_then(|(b, _)| b.sector).and_then(|s| {
            let def = &catalog.sectors[s];
            if def.children.is_empty() { def.parent } else { Some(s) }
        });
        if let Some(sector) = target {
            expanded.toggle(sector);
        }
    }

    ui.horizontal_wrapped(|ui| {
        sector_legend(ui, catalog, expanded);
        swatch(ui, TRADING_COLOR, "trading & carrying");
        swatch(ui, LEISURE_COLOR, "leisure");
    });
}

pub(super) fn people_chart(ui: &mut egui::Ui, catalog: &Catalog, history: &History) {
    let samples = &history.0;
    let series = |value: &dyn Fn(&crate::sim::Sample) -> f32| -> PlotPoints {
        samples.iter().map(|s| [s.year as f64, value(s) as f64 * 100.0]).collect()
    };
    let mut lines: Vec<(String, [u8; 3], PlotPoints, bool)> = vec![
        ("needs unmet".into(), HUNGRY_COLOR, series(&|s| s.hungry_share), false),
        ("specialisation".into(), SPECIALISATION_COLOR, series(&|s| s.specialisation), true),
    ];
    for (id, good) in catalog.goods.iter().enumerate().filter(|(_, g)| g.is_tool()) {
        let article = if good.name.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
        lines.push((format!("own {article} {}", good.name), lighten(good.color), series(&|s| s.coverage.get(id).copied().unwrap_or(0.0)), false));
    }
    let names: Vec<(String, [u8; 3], bool)> = lines.iter().map(|l| (l.0.clone(), l.1, l.3)).collect();
    year_plot("people", "% of people")
        .height(180.0)
        .label_formatter(|hover| match hover {
            HoverPosition::NearDataPoint { plot_name, position, .. } => {
                Some(format!("{plot_name}\n{:.0}% in year {:.0}", position.y, position.x))
            }
            HoverPosition::Elsewhere { .. } => None,
        })
        .show(ui, |plot| {
            for (name, color, points, dashed) in lines {
                let line = Line::new(name, points).color(color32(color)).width(2.0);
                plot.line(if dashed { line.style(egui_plot::LineStyle::dashed_loose()) } else { line });
            }
        });
    ui.horizontal_wrapped(|ui| {
        for (name, color, dashed) in &names {
            swatch(ui, *color, &if *dashed { format!("{name} (dashed)") } else { name.clone() });
        }
    });
}

pub(super) fn population_chart(ui: &mut egui::Ui, history: &History) {
    let samples = &history.0;
    let series = |value: &dyn Fn(&crate::sim::Sample) -> f32| -> PlotPoints {
        samples.iter().map(|s| [s.year as f64, value(s) as f64]).collect()
    };
    let lines = [
        ("people", POPULATION_COLOR, series(&|s| s.population)),
        ("children", CHILD_COLOR, series(&|s| s.children)),
        ("births a year", LEISURE_COLOR, series(&|s| s.births)),
        ("deaths a year", HUNGRY_COLOR, series(&|s| s.deaths)),
    ];
    let names: Vec<(&str, [u8; 3])> = lines.iter().map(|l| (l.0, l.1)).collect();
    year_plot("population", "people")
        .height(180.0)
        .y_axis_min_width(30.0)
        .label_formatter(|hover| match hover {
            HoverPosition::NearDataPoint { plot_name, position, .. } => {
                Some(format!("{plot_name}\n{:.0} in year {:.0}", position.y, position.x))
            }
            HoverPosition::Elsewhere { .. } => None,
        })
        .show(ui, |plot| {
            for (name, color, points) in lines {
                plot.line(Line::new(name, points).color(color32(color)).width(2.0));
            }
        });
    ui.horizontal_wrapped(|ui| {
        for (name, color) in &names {
            swatch(ui, *color, name);
        }
    });
}

fn year_plot<'a>(id: &str, y_label: &str) -> Plot<'a> {
    Plot::new(id)
        .height(240.0)
        .include_y(0.0)
        .include_y(100.0)
        .include_x(0.0)
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_double_click_reset(false)
        .x_axis_label("year")
        .x_grid_spacer(egui_plot::uniform_grid_spacer(|input| {
            let span = input.bounds.1 - input.bounds.0;
            let step = [10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0]
                .into_iter()
                .find(|s| span / s <= 8.0)
                .unwrap_or(2000.0);
            [step / 5.0, step, step * 5.0]
        }))
        .y_axis_label(y_label.to_string())
}

pub(super) fn sector_legend(ui: &mut egui::Ui, catalog: &Catalog, expanded: &mut Expanded) {
    fn entry(ui: &mut egui::Ui, catalog: &Catalog, expanded: &Expanded, sector: SectorId, clicked: &mut Option<SectorId>) {
        let def = &catalog.sectors[sector];
        let open = expanded.0.contains(&sector);
        let state = (!def.children.is_empty()).then_some(open);
        let response = sector_swatch(ui, def.color, &def.name, state);
        if state.is_some() && response.on_hover_text(if open { "Click to group these again" } else { "Click to split this up" }).clicked() {
            *clicked = Some(sector);
        }
        if state == Some(true) {
            for &child in &def.children {
                entry(ui, catalog, expanded, child, clicked);
            }
        }
    }
    let mut clicked = None;
    for root in (0..catalog.sectors.len()).filter(|&s| catalog.sectors[s].parent.is_none()) {
        entry(ui, catalog, expanded, root, &mut clicked);
    }
    if let Some(sector) = clicked {
        expanded.toggle(sector);
    }
}
