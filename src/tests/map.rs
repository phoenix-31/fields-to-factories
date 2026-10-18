//! Tests for the map: loading `map.ron`, validation, neighbourhoods, crowding and nearest towns.

use super::*;

const GOODS: &str = include_str!("../../goods.ron");
const MAP: &str = include_str!("../../map.ron");

fn catalog() -> Catalog {
    Catalog::parse(GOODS).expect("goods.ron parses")
}

fn map() -> WorldMap {
    WorldMap::parse(MAP, &catalog()).expect("map.ron parses")
}

#[test]
fn default_map_loads() {
    let m = map();
    assert_eq!(m.towns.len(), 4);
    assert_eq!(m.elevation.len(), COLS * ROWS);
    assert_eq!(m.fields.len(), 4);
    for field in &m.fields {
        assert!(field.iter().all(|v| (0.0..=1.0).contains(v)));
    }
}

#[test]
fn rejects_bad_definitions() {
    let cases = [
        (MAP.replace("at: (16, 15)", "at: (40, 15)"), "off the"),
        (MAP.replace(r#"name: "Crossways""#, r#"name: "Rivermouth""#), "defined twice"),
        (MAP.replace(r#"("ore", High)"#, r#"("gold", High)"#), "no recipe uses"),
        (MAP.replace("crowding: 1.0", "crowding: -1.0"), "crowding"),
    ];
    let c = catalog();
    for (text, expected) in cases {
        let err = WorldMap::parse(&text, &c).err().unwrap_or_else(|| panic!("should reject: {expected}"));
        assert!(err.contains(expected), "expected \"{expected}\", got \"{err}\"");
    }
}

#[test]
fn neighbourhoods_stop_at_the_edge() {
    assert_eq!(WorldMap::neighbourhood(0).count(), 4);
    assert_eq!(WorldMap::neighbourhood(COLS + 1).count(), 9);
    assert_eq!(WorldMap::neighbourhood(COLS * ROWS - 1).count(), 4);
}

#[test]
fn crowding_thins_resources_past_capacity() {
    let m = map();
    let spot = 5 * COLS + 5;
    let empty = vec![0.0; COLS * ROWS];
    assert_eq!(m.crowding_factor(spot, &empty, 0.0), 1.0);

    let mean_weight = m.settle_weights.iter().sum::<f32>() / m.settle_weights.len() as f32;
    let full: Vec<f32> = m.settle_weights.iter().map(|w| m.capacity * w / mean_weight).collect();
    assert!((m.crowding_factor(spot, &full, 0.0) - 1.0).abs() < 1e-4);
    let double: Vec<f32> = full.iter().map(|c| 2.0 * c).collect();
    assert!((m.crowding_factor(spot, &double, 0.0) - 0.5).abs() < 1e-4);
}

#[test]
fn everyone_has_a_nearest_town() {
    let m = map();
    for (t, town) in m.towns.iter().enumerate() {
        let (nearest, distance) = m.nearest_town((town.at.0 as f32, town.at.1 as f32));
        assert_eq!((nearest, distance), (t, 0.0));
    }
}

#[test]
fn settling_stays_on_the_map() {
    let m = map();
    for i in 0..100 {
        let u = i as f32 / 100.0;
        let (x, y) = m.settle(u, 1.0 - u, u);
        assert!((0.0..=(COLS - 1) as f32).contains(&x) && (0.0..=(ROWS - 1) as f32).contains(&y));
    }
}
