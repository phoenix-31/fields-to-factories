//! The marketplace: prices, the time it takes to trade, and how each person decides to make or buy.

use bevy::prelude::*;

use crate::goods::{Catalog, GoodId};

const PRICE_SPEED: f32 = 0.2;
const JOB_SPREAD: f32 = 0.1;
const ROUTE_PASSES: usize = 4;

#[derive(Resource, Clone)]
pub struct Market {
    pub prices: Vec<f32>,
    pub trade_cost: f32,
    pub bought: Vec<f32>,
    pub sold: Vec<f32>,
}

impl Market {
    pub fn new(prices: Vec<f32>, trade_cost: f32) -> Self {
        let n = prices.len();
        Self { prices, trade_cost, bought: vec![0.0; n], sold: vec![0.0; n] }
    }

    fn trading_hours(&self, good: GoodId, wage: f32) -> f32 {
        self.trade_cost * self.prices[good] / wage
    }

    pub fn mismatch(&self) -> f32 {
        let (mut gap, mut volume) = (0.0, 0.0);
        for ((b, s), p) in self.bought.iter().zip(&self.sold).zip(&self.prices) {
            gap += (b - s).abs() * p;
            volume += (b + s) * p;
        }
        if volume > 1e-6 { gap / volume } else { 0.0 }
    }

    fn full_cost(&self, good: GoodId) -> f32 {
        self.prices[good] * (1.0 + self.trade_cost)
    }

    pub fn adjust(&mut self, bought: Vec<f32>, sold: Vec<f32>, mean_wage: f32) {
        for g in 0..self.prices.len() {
            let (b, s) = (bought[g], sold[g]);
            if b + s > 1e-6 {
                self.prices[g] *= (PRICE_SPEED * (b - s) / (b + s)).exp();
            }
        }
        if mean_wage > 1e-6 {
            for price in &mut self.prices {
                *price /= mean_wage;
            }
        }
        self.bought = bought;
        self.sold = sold;
    }
}

#[derive(Clone)]
pub struct Flow {
    pub hours: Vec<f32>,
    pub trading: f32,
    pub bought: Vec<f32>,
    pub sold: Vec<f32>,
}

impl Flow {
    fn zero(n: usize) -> Self {
        Self { hours: vec![0.0; n], trading: 0.0, bought: vec![0.0; n], sold: vec![0.0; n] }
    }

    fn add(&mut self, other: &Flow, scale: f32) {
        for (a, b) in self.hours.iter_mut().zip(&other.hours) {
            *a += scale * b;
        }
        self.trading += scale * other.trading;
        for (a, b) in self.bought.iter_mut().zip(&other.bought) {
            *a += scale * b;
        }
        for (a, b) in self.sold.iter_mut().zip(&other.sold) {
            *a += scale * b;
        }
    }

    pub fn total_hours(&self) -> f32 {
        self.hours.iter().sum::<f32>() + self.trading
    }
}

pub struct Routes {
    pub flows: Vec<Flow>,
    pub unit_hours: Vec<f32>,
    pub wage: f32,
    pub job: Option<GoodId>,
}

pub fn plan_routes(
    catalog: &Catalog,
    market: &Market,
    endowments: &[f32],
    owned: &[f32],
    job: Option<GoodId>,
    review: Option<f32>,
) -> Routes {
    let n = catalog.len();
    let direct: Vec<f32> = (0..n).map(|g| 1.0 / catalog.direct_rate(g, endowments, owned)).collect();
    let (mut hours, mut money, mut buy_input) = (vec![0.0; n], vec![0.0; n], vec![false; n]);
    let mut wage = 0.0;
    let current = job;
    let mut job = None;

    for _ in 0..ROUTE_PASSES {
        for &g in &catalog.build_order {
            let (mut h, mut m) = (direct[g], 0.0);
            for &(input, qty) in &catalog.goods[g].recipe.inputs {
                if buy_input[input] {
                    m += qty * market.prices[input];
                    h += qty * market.trading_hours(input, wage);
                } else {
                    h += qty * hours[input];
                    m += qty * money[input];
                }
            }
            hours[g] = h;
            money[g] = m;
            buy_input[g] = wage > 1e-6 && market.full_cost(g) < h * wage + m;
        }
        let earning = |g: GoodId| (market.prices[g] - money[g]) / hours[g];
        let chosen = match (review, current.filter(|&j| earning(j) > 1e-6)) {
            (None, Some(j)) => Some(j),
            (None, None) if current.is_none() => None,
            (draw, _) => job.filter(|&j| earning(j) > 1e-6).or_else(|| choose_job(&earning, n, draw.unwrap_or(0.5))),
        };
        (wage, job) = match chosen {
            Some(j) => (earning(j), Some(j)),
            None => (0.0, None),
        };
    }

    let mut make: Vec<Flow> = vec![Flow::zero(n); n];
    for &g in &catalog.build_order {
        let mut flow = Flow::zero(n);
        flow.hours[g] = direct[g];
        for &(input, qty) in &catalog.goods[g].recipe.inputs {
            if buy_input[input] {
                flow.bought[input] += qty;
                flow.trading += qty * market.trading_hours(input, wage);
            } else {
                let sub = make[input].clone();
                flow.add(&sub, qty);
            }
        }
        make[g] = flow;
    }

    let earn = |amount: f32| -> Option<Flow> {
        let j = job?;
        let net = market.prices[j] - money[j];
        if net <= 1e-6 {
            return None;
        }
        let mut flow = make[j].clone();
        flow.sold[j] += 1.0;
        let mut scaled = Flow::zero(n);
        scaled.add(&flow, amount / net);
        Some(scaled)
    };

    let unavailable = |g: GoodId| {
        let mut f = Flow::zero(n);
        f.hours[g] = f32::INFINITY;
        f
    };
    let mut flows = Vec::with_capacity(n);
    for g in 0..n {
        let made = if money[g] > 1e-9 {
            earn(money[g]).map(|e| {
                let mut f = make[g].clone();
                f.add(&e, 1.0);
                f
            })
        } else {
            Some(make[g].clone())
        };
        let bought = earn(market.prices[g]).map(|mut e| {
            e.bought[g] += 1.0;
            e.trading += market.trading_hours(g, wage);
            e
        });
        flows.push(match (made, bought) {
            (Some(m), Some(b)) if b.total_hours() < m.total_hours() => b,
            (Some(m), _) => m,
            (None, Some(b)) => b,
            (None, None) => unavailable(g),
        });
    }
    let unit_hours = flows.iter().map(Flow::total_hours).collect();
    Routes { flows, unit_hours, wage, job }
}

fn choose_job(earning: &dyn Fn(GoodId) -> f32, n: usize, draw: f32) -> Option<GoodId> {
    let best = (0..n).map(earning).fold(0.0, f32::max);
    if best <= 1e-6 {
        return None;
    }
    let weights: Vec<f32> = (0..n)
        .map(|g| {
            let e = earning(g);
            if e > 0.0 { ((e - best) / (JOB_SPREAD * best)).exp() } else { 0.0 }
        })
        .collect();
    let mut target = draw * weights.iter().sum::<f32>();
    for (g, w) in weights.iter().enumerate() {
        target -= w;
        if target <= 0.0 && *w > 0.0 {
            return Some(g);
        }
    }
    weights.iter().rposition(|&w| w > 0.0)
}

pub fn autarky_prices(catalog: &Catalog, endowments: &[Vec<f32>]) -> Vec<f32> {
    let n = catalog.len();
    let mut prices = vec![0.0; n];
    for e in endowments {
        let owned = vec![0.0; n];
        let mut hours = vec![0.0; n];
        for &g in &catalog.build_order {
            hours[g] = 1.0 / catalog.direct_rate(g, e, &owned)
                + catalog.goods[g].recipe.inputs.iter().map(|&(i, q)| q * hours[i]).sum::<f32>();
        }
        for (p, h) in prices.iter_mut().zip(hours) {
            *p += h / endowments.len() as f32;
        }
    }
    prices
}
