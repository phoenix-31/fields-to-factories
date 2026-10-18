//! Whole-simulation tests: short headless runs checked for accounting, determinism and sensible outcomes.

use std::path::PathBuf;

use super::*;
use crate::people::Population;

const GOODS: &str = include_str!("../../goods.ron");
const PEOPLE: &str = include_str!("../../people.ron");
const MAP: &str = include_str!("../../map.ron");

fn plugin(people: &str, count: usize) -> SimPlugin {
    let catalog = Catalog::parse(GOODS).expect("goods.ron parses");
    let population = Population::parse(people, &catalog).expect("people parse");
    let map = WorldMap::parse(MAP, &catalog).expect("map.ron parses");
    SimPlugin {
        catalog,
        population,
        map,
        goods_path: PathBuf::from("goods.ron"),
        people_path: PathBuf::from("people.ron"),
        map_path: PathBuf::from("map.ron"),
        trade_cost: DEFAULT_TRADE_COST,
        count: Some(count),
    }
}

fn run(people: &str, count: usize, years: u32) -> App {
    let mut app = headless_app(plugin(people, count));
    for _ in 0..years {
        app.world_mut().run_schedule(SimTick);
    }
    app
}

fn history(app: &App) -> &[Sample] {
    &app.world().resource::<History>().0
}

#[test]
fn every_waking_hour_is_accounted_for() {
    let app = run(PEOPLE, 300, 30);
    assert_eq!(history(&app).len(), 30);
    for s in history(&app) {
        let total: f32 = s.shares.iter().sum::<f32>() + s.trading_share + s.leisure_share;
        assert!((total - 1.0).abs() < 1e-3, "year {}: hours add up to {total}", s.year);
    }
}

#[test]
fn runs_are_repeatable() {
    let csv = |app: &App| {
        let world = app.world();
        let towns: Vec<String> = world.resource::<Markets>().towns.iter().map(|t| t.name.clone()).collect();
        let mut out = Vec::new();
        write_csv(world.resource::<Catalog>(), &towns, history(app), &mut out).expect("write csv");
        out
    };
    assert_eq!(csv(&run(PEOPLE, 200, 15)), csv(&run(PEOPLE, 200, 15)));
}

#[test]
fn prices_stay_positive_and_finite() {
    let app = run(PEOPLE, 300, 40);
    for s in history(&app) {
        assert!(s.prices.iter().flatten().all(|p| p.is_finite() && *p > 0.0), "year {}", s.year);
    }
}

#[test]
fn people_stay_put_without_migration() {
    let still = PEOPLE.replace("chance: 0.05", "chance: 0.0");
    let app = run(&still, 300, 20);
    let first = history(&app)[0].town_people.clone();
    for s in history(&app) {
        assert_eq!(s.moved, 0.0);
        assert_eq!(s.town_people, first);
    }
    assert_eq!(first.iter().sum::<f32>(), 300.0);
}

#[test]
fn migration_keeps_everyone_on_the_map() {
    let mut app = run(PEOPLE, 300, 30);
    assert!(history(&app).iter().any(|s| s.moved > 0.0));
    let world = app.world_mut();
    let mut people = world.query::<&Person>();
    assert_eq!(people.iter(world).count(), 300);
    for p in people.iter(world) {
        assert!((0.0..=(POPULATION_COLS - 1) as f32).contains(&p.x));
        assert!((0.0..=(POPULATION_ROWS - 1) as f32).contains(&p.y));
    }
}

#[test]
fn hunger_falls_as_ploughs_spread() {
    let app = run(PEOPLE, 300, 40);
    let (first, last) = (&history(&app)[0], history(&app).last().unwrap());
    assert!(first.hungry_share > 0.3, "{}", first.hungry_share);
    assert!(last.hungry_share < first.hungry_share / 4.0, "{}", last.hungry_share);
    let catalog = app.world().resource::<Catalog>();
    let iron = catalog.goods.iter().position(|g| g.name == "iron plough").unwrap();
    assert!(last.coverage[iron] > 0.5);
}

#[test]
fn tool_savings_never_exceed_one_tool() {
    let mut app = run(PEOPLE, 300, 40);
    let world = app.world_mut();
    let mut tools = world.query::<&Tools>();
    for t in tools.iter(world) {
        for good in 0..t.progress.len() {
            assert!((0.0..=1.0).contains(&t.progress[good]));
            assert!(t.years_left[good] <= t.life[good].max(0.0));
        }
    }
}

#[test]
fn annuities_discount_later_years() {
    assert_eq!(annuity(10.0, 0.0), 10.0);
    assert!((annuity(1.0, 0.05) - 1.0 / 1.05).abs() < 1e-5);
    assert!(annuity(30.0, 0.05) < 30.0 && annuity(30.0, 0.05) > annuity(20.0, 0.05));
}
