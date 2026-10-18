//! Tests for the population definition: loading `people.ron`, defaults, validation and taste curves.

use super::*;

const GOODS: &str = include_str!("../../goods.ron");
const PEOPLE: &str = include_str!("../../people.ron");

fn catalog() -> Catalog {
    Catalog::parse(GOODS).expect("goods.ron parses")
}

#[test]
fn default_people_load() {
    let p = Population::parse(PEOPLE, &catalog()).expect("people.ron parses");
    assert_eq!(p.endowments.len(), 4);
    assert_eq!(p.needs.len(), 2);
    assert_eq!(p.count.mean(), 1500.0);
}

#[test]
fn migration_has_defaults() {
    let text = PEOPLE.replace("migration: (chance: 0.05, look_at: 6, min_gain: 0.1),", "");
    let p = Population::parse(&text, &catalog()).expect("parses without migration");
    assert_eq!((p.migration.chance, p.migration.look_at, p.migration.min_gain), (0.05, 6, 0.1));
}

#[test]
fn rejects_bad_definitions() {
    let cases = [
        (PEOPLE.replace("chance: 0.05", "chance: 1.5"), "migration chance"),
        (PEOPLE.replace("count: 1500", "count: 0"), "count must be"),
        (PEOPLE.replace(r#"("ore", (0.1, 1.6)),"#, ""), "no endowment"),
        (PEOPLE.replace(r#"("ore", (0.1, 1.6))"#, r#"("ore", (0.0, 1.6))"#), "above zero"),
        (PEOPLE.replace(r#"of: "warmth""#, r#"of: "glamour""#), "no good provides"),
    ];
    let c = catalog();
    for (text, expected) in cases {
        let err = Population::parse(&text, &c).err().unwrap_or_else(|| panic!("should reject: {expected}"));
        assert!(err.contains(expected), "expected \"{expected}\", got \"{err}\"");
    }
}

#[test]
fn spreads_draw_within_their_range() {
    let range = Spread::Range(0.4, 1.6);
    assert_eq!(range.draw(0.0), 0.4);
    assert_eq!(range.draw(1.0), 1.6);
    assert!((range.mean() - 1.0).abs() < 1e-6);
    assert_eq!(Spread::Fixed(2.0).draw(0.7), 2.0);
}

#[test]
fn saturating_tastes_level_off_and_log_tastes_keep_rising() {
    let saturating = Taste::Saturating { weight: 1.5, scale: 1.0 };
    assert_eq!(saturating.value(0.0), 0.0);
    assert!(saturating.value(10.0) < 1.5 && saturating.value(10.0) > 1.49);
    let log = Taste::Log { weight: 1.0 };
    assert!(log.value(100.0) > log.value(10.0) && log.value(10.0) > log.value(1.0));
}
