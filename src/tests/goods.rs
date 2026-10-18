//! Tests for the goods catalog: loading `goods.ron`, validation, the sector tree and tool arithmetic.

use super::*;

const GOODS: &str = include_str!("../../goods.ron");

fn catalog() -> Catalog {
    Catalog::parse(GOODS).expect("goods.ron parses")
}

fn id(catalog: &Catalog, name: &str) -> GoodId {
    catalog.goods.iter().position(|g| g.name == name).expect("good exists")
}

fn sector(catalog: &Catalog, path: &str) -> SectorId {
    (0..catalog.sectors.len()).find(|&s| catalog.sector_path(s) == path).expect("sector exists")
}

fn parse(goods: &[&str]) -> Result<Catalog, String> {
    Catalog::parse(&format!("#![enable(implicit_some)]\n[{}]", goods.join(",")))
}

const GRAIN: &str = r#"(name: "grain", sector: ["farming"], color: (0, 0, 0), recipe: (rate: 1.0))"#;
const BREAD: &str =
    r#"(name: "bread", sector: ["baking"], color: (0, 0, 0), recipe: (rate: 1.0, inputs: [("grain", 1.0)]))"#;

#[test]
fn default_goods_load() {
    let c = catalog();
    assert_eq!(c.len(), 11);
    assert_eq!(c.resources, ["land", "pasture", "forest", "ore"]);
    assert_eq!(c.characteristics, ["nourishment", "warmth", "comfort"]);
}

#[test]
fn build_order_puts_inputs_first() {
    let c = catalog();
    assert_eq!(c.build_order.len(), c.len());
    for (pos, &good) in c.build_order.iter().enumerate() {
        for &(input, _) in &c.goods[good].recipe.inputs {
            assert!(c.build_order[..pos].contains(&input), "{} comes before its input", c.goods[good].name);
        }
    }
}

#[test]
fn sector_tree_covers_every_good_once() {
    let c = catalog();
    let roots: Vec<&str> = c.sectors.iter().filter(|s| s.parent.is_none()).map(|s| s.name.as_str()).collect();
    assert_eq!(roots, ["primary", "secondary"]);
    let leaves: usize = c.sectors.iter().filter(|s| s.children.is_empty()).map(|s| s.goods.len()).sum();
    assert_eq!(leaves, c.len());
    let agriculture = &c.sectors[sector(&c, "primary/agriculture")];
    assert_eq!(agriculture.goods, [id(&c, "wheat"), id(&c, "wool")]);
    assert_eq!(c.goods[id(&c, "wheat")].sector, "wheat farming");
}

#[test]
fn group_colour_is_the_average_of_its_goods() {
    let c = catalog();
    let textiles = &c.sectors[sector(&c, "secondary/textiles")];
    let [a, b] = [id(&c, "cloth"), id(&c, "clothing")].map(|g| c.goods[g].color);
    let expected = [0, 1, 2].map(|i| ((a[i] as u32 + b[i] as u32) / 2) as u8);
    assert_eq!(textiles.color, expected);
}

#[test]
fn a_sector_cannot_be_both_a_good_and_a_group() {
    let text = GOODS.replace(r#"["primary", "forestry"]"#, r#"["primary", "agriculture"]"#);
    let err = Catalog::parse(&text).err().expect("rejected");
    assert!(err.contains("already used"), "{err}");
}

#[test]
fn rejects_bad_definitions() {
    let cases = [
        (vec![GRAIN, GRAIN], "defined twice"),
        (vec![BREAD], "unknown good"),
        (vec![r#"(name: "grain", sector: [], color: (0, 0, 0), recipe: (rate: 1.0))"#], "needs a sector"),
        (vec![r#"(name: "grain", sector: ["x"], color: (0, 0, 0), recipe: (rate: 0.0))"#], "positive rate"),
        (
            vec![
                r#"(name: "a", sector: ["a"], color: (0, 0, 0), recipe: (rate: 1.0, inputs: [("b", 1.0)]))"#,
                r#"(name: "b", sector: ["b"], color: (0, 0, 0), recipe: (rate: 1.0, inputs: [("a", 1.0)]))"#,
            ],
            "input to itself",
        ),
        (
            vec![
                r#"(name: "hoe", sector: ["h"], color: (0, 0, 0), recipe: (rate: 1.0))"#,
                r#"(name: "grain", sector: ["g"], color: (0, 0, 0), recipe: (rate: 1.0, tools: [(good: "hoe", multiplier: 2.0)]))"#,
            ],
            "must have a lifetime",
        ),
        (
            vec![
                r#"(name: "hoe", sector: ["h"], color: (0, 0, 0), recipe: (rate: 1.0), lifetime: (2.0, 3.0))"#,
                r#"(name: "grain", sector: ["g"], color: (0, 0, 0), recipe: (rate: 1.0, tools: [(good: "hoe", multiplier: 0.5)]))"#,
            ],
            "at least 1",
        ),
    ];
    for (goods, expected) in cases {
        let err = parse(&goods).err().unwrap_or_else(|| panic!("should reject: {expected}"));
        assert!(err.contains(expected), "expected \"{expected}\", got \"{err}\"");
    }
    assert!(parse(&[GRAIN, BREAD]).is_ok());
}

#[test]
fn the_best_owned_tool_does_the_work() {
    let c = catalog();
    let (wheat, wooden, iron) = (id(&c, "wheat"), id(&c, "wooden plough"), id(&c, "iron plough"));
    let mut owned = vec![0.0; c.len()];
    assert_eq!(c.hours_factor(wheat, &owned), 1.0);
    owned[wooden] = 1.0;
    assert!((c.tool_multiplier(wheat, &owned) - 1.5).abs() < 1e-5);
    owned[iron] = 1.0;
    assert!((c.tool_multiplier(wheat, &owned) - 2.5).abs() < 1e-5);
    owned[wooden] = 0.0;
    assert!((c.hours_factor(wheat, &owned) - 0.4).abs() < 1e-5);
}

#[test]
fn direct_rate_scales_with_the_resource_used() {
    let c = catalog();
    let wheat = id(&c, "wheat");
    let none = vec![0.0; c.len()];
    let poor = c.direct_rate(wheat, &[0.5, 1.0, 1.0, 1.0], &none);
    let rich = c.direct_rate(wheat, &[1.5, 1.0, 1.0, 1.0], &none);
    assert!((rich / poor - 3.0).abs() < 1e-5);
    assert!((poor - c.goods[wheat].recipe.rate * 0.5).abs() < 1e-6);
}
