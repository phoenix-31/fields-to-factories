//! Opening each town's market at self-sufficient prices, and clearing it each year.

use super::*;

pub(super) fn open_market(
    mut markets: ResMut<Markets>,
    catalog: Res<Catalog>,
    world_map: Res<WorldMap>,
    people: Query<(&Endowments, &Home)>,
) {
    let n = catalog.len();
    markets.travel_cost = world_map.travel_cost;
    markets.towns = (0..world_map.towns.len())
        .map(|t| {
            let local: Vec<Vec<f32>> = people.iter().filter(|(_, h)| h.town == t).map(|(e, _)| e.0.clone()).collect();
            let all: Vec<Vec<f32>> = people.iter().map(|(e, _)| e.0.clone()).collect();
            Town {
                name: world_map.towns[t].name.clone(),
                at: world_map.town_at(t),
                prices: autarky_prices(&catalog, if local.is_empty() { &all } else { &local }),
                bought: vec![0.0; n],
                sold: vec![0.0; n],
                imported: vec![0.0; n],
            }
        })
        .collect();
}

pub(super) fn clear_market(mut markets: ResMut<Markets>, catalog: Res<Catalog>, people: Query<(&Home, &Trades, &Wage), Without<Child>>) {
    let n = catalog.len();
    let towns = markets.towns.len();
    let (mut bought, mut sold) = (vec![vec![0.0; n]; towns], vec![vec![0.0; n]; towns]);
    let (mut wages, mut count) = (0.0f32, 0.0f32);
    for (home, trades, wage) in &people {
        for g in 0..n {
            bought[home.town][g] += trades.bought[g];
            sold[home.town][g] += trades.sold[g];
        }
        if wage.0 > 0.0 {
            wages += wage.0;
            count += 1.0;
        }
    }
    markets.clear(&catalog, bought, sold, wages / count.max(1.0));
}
