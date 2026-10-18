//! Tests for markets: price changes, merchants shipping between towns, make-or-buy shares, job choice
//! and route planning.

use super::*;

const GOODS: &str = include_str!("../../goods.ron");

fn catalog() -> Catalog {
    Catalog::parse(GOODS).expect("goods.ron parses")
}

fn town(name: &str, at: (f32, f32), prices: Vec<f32>) -> Town {
    let n = prices.len();
    Town { name: name.into(), at, prices, bought: vec![0.0; n], sold: vec![0.0; n], imported: vec![0.0; n] }
}

fn markets(towns: Vec<Town>, travel_cost: f32) -> Markets {
    Markets { towns, trade_cost: 0.3, travel_cost, carried: 0.0 }
}

fn one_good(n: usize, good: GoodId, amount: f32) -> Vec<f32> {
    let mut v = vec![0.0; n];
    v[good] = amount;
    v
}

#[test]
fn excess_demand_raises_the_price() {
    let c = catalog();
    let n = c.len();
    let mut m = markets(vec![town("A", (0.0, 0.0), vec![1.0; n])], 0.02);
    m.clear(&c, vec![one_good(n, 0, 10.0)], vec![one_good(n, 0, 5.0)], 1.0);
    assert!(m.towns[0].prices[0] > 1.0);
    m.clear(&c, vec![one_good(n, 0, 5.0)], vec![one_good(n, 0, 10.0)], 1.0);
    m.clear(&c, vec![one_good(n, 0, 5.0)], vec![one_good(n, 0, 10.0)], 1.0);
    assert!(m.towns[0].prices[0] < 1.0);
    assert_eq!(m.towns[0].prices[1], 1.0);
}

#[test]
fn merchants_ship_from_cheap_to_dear_towns() {
    let c = catalog();
    let n = c.len();
    let towns = vec![town("A", (0.0, 0.0), vec![1.0; n]), town("B", (1.0, 0.0), vec![2.0; n])];
    let mut m = markets(towns, 0.01);
    m.clear(&c, vec![vec![0.0; n], one_good(n, 0, 10.0)], vec![one_good(n, 0, 10.0), vec![0.0; n]], 1.0);
    assert_eq!(m.towns[0].imported[0], -10.0);
    assert_eq!(m.towns[1].imported[0], 10.0);
    assert!((m.carried - 20.0).abs() < 1e-4);
    assert_eq!(m.mismatch(), 0.0);
}

#[test]
fn merchants_stay_home_when_carrying_costs_more_than_the_gap() {
    let c = catalog();
    let n = c.len();
    let towns = vec![town("A", (0.0, 0.0), vec![1.0; n]), town("B", (1.0, 0.0), vec![2.0; n])];
    let mut m = markets(towns, 5.0);
    m.clear(&c, vec![vec![0.0; n], one_good(n, 0, 10.0)], vec![one_good(n, 0, 10.0), vec![0.0; n]], 1.0);
    assert_eq!(m.carried, 0.0);
    assert!(m.mismatch() > 0.99);
}

#[test]
fn the_cheaper_way_gets_the_larger_share() {
    assert!((share_bought(1.0, 1.0) - 0.5).abs() < 1e-6);
    assert!(share_bought(2.0, 1.0) > 0.99);
    assert!(share_bought(1.0, 2.0) < 0.01);
    let shares: Vec<f32> = [0.9, 0.95, 1.0, 1.05, 1.1].iter().map(|&buy| share_bought(1.0, buy)).collect();
    assert!(shares.windows(2).all(|w| w[0] > w[1]));
}

#[test]
fn job_choice_favours_better_pay() {
    let earnings = [0.5, 2.0, 1.0];
    let earning = |g: GoodId| earnings[g];
    assert_eq!(choose_job(&earning, 3, -1.0), Some(1));
    let picks: Vec<_> = (0..100).map(|i| choose_job(&earning, 3, i as f32 / 100.0)).collect();
    let best = picks.iter().filter(|&&p| p == Some(1)).count();
    assert!(best > 90, "best job picked {best} times out of 100");
    assert_eq!(choose_job(&|_| -1.0, 3, 0.5), None);
}

#[test]
fn self_sufficient_people_make_everything_themselves() {
    let c = catalog();
    let n = c.len();
    let prices = vec![1.0; n];
    let access = Access { prices: &prices, trade_cost: 0.3, carrying: vec![0.0; n] };
    let endowments = [1.0; 4];
    let owned = vec![0.0; n];
    let routes = plan_routes(&c, &access, &endowments, &owned, None, None, 10.0);
    assert_eq!(routes.job, None);
    assert_eq!(routes.wage, 0.0);
    let wheat = c.goods.iter().position(|g| g.name == "wheat").unwrap();
    assert!((routes.unit_hours[wheat] - 1.0 / c.direct_rate(wheat, &endowments, &owned)).abs() < 1e-4);
    assert!(routes.flows.iter().all(|f| f.bought.iter().all(|&b| b == 0.0)));
}

#[test]
fn a_well_paid_job_makes_buying_worthwhile() {
    let c = catalog();
    let n = c.len();
    let mut prices = vec![1.0; n];
    let wool = c.goods.iter().position(|g| g.name == "wool").unwrap();
    prices[wool] = 5.0;
    let access = Access { prices: &prices, trade_cost: 0.3, carrying: vec![0.0; n] };
    let routes = plan_routes(&c, &access, &[1.0; 4], &vec![0.0; n], None, BEST_JOB, 10.0);
    assert_eq!(routes.job, Some(wool));
    assert!(routes.wage > 1.0);
    let wheat = c.goods.iter().position(|g| g.name == "wheat").unwrap();
    assert!(routes.flows[wheat].bought[wheat] > 0.5);
}
