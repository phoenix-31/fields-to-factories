//! The production tree diagram and the history chart.

use bevy_egui::egui;
use egui_plot::{FilledArea, Line, Plot, PlotPoints};

use super::{HUNGRY_COLOR, LEISURE_COLOR, SPECIALISATION_COLOR, TRADING_COLOR, color32, lighten, swatch};
use crate::goods::{Catalog, GoodId};
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

pub(super) fn stacked_chart(ui: &mut egui::Ui, catalog: &Catalog, history: &History) {
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
        .x_grid_spacer(egui_plot::uniform_grid_spacer(|input| {
            let span = input.bounds.1 - input.bounds.0;
            let step = [10.0, 20.0, 50.0, 100.0].into_iter().find(|s| span / s <= 8.0).unwrap_or(200.0);
            [step / 5.0, step, step * 5.0]
        }))
        .y_axis_label("% of waking hours")
        .show(ui, |plot| {
            for (name, color, lower, upper) in &bands {
                plot.add(FilledArea::new(name.clone(), &xs, lower, upper).fill_color(color32(*color)));
            }
            plot.line(Line::new("needs unmet (% of people)", hungry).color(color32(HUNGRY_COLOR)).width(2.0));
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
        swatch(ui, HUNGRY_COLOR, "needs unmet");
        swatch(ui, SPECIALISATION_COLOR, "specialisation (dashed)");
    });
}
