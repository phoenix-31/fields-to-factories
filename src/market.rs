//! Market towns: local prices, the time it takes to trade and carry goods, merchants shipping
//! between towns, and how each person decides to make or buy.

use bevy::prelude::*;

use crate::goods::{Catalog, CharId, GoodId};

const PRICE_SPEED: f32 = 0.2;
const JOB_SPREAD: f32 = 0.1;
const BUY_SPREAD: f32 = 0.1;
const ROUTE_PASSES: usize = 4;
pub const BEST_JOB: Option<f32> = Some(-1.0);

pub struct Town {
    pub name: String,
    pub at: (f32, f32),
    pub prices: Vec<f32>,
    pub bought: Vec<f32>,
    pub sold: Vec<f32>,
    pub imported: Vec<f32>,
}

#[derive(Resource)]
pub struct Markets {
    pub towns: Vec<Town>,
    pub trade_cost: f32,
    pub travel_cost: f32,
    pub carried: f32,
}

pub struct Access<'a> {
    pub prices: &'a [f32],
    pub trade_cost: f32,
    pub carrying: Vec<f32>,
}

impl Access<'_> {
    fn trading_hours(&self, good: GoodId, wage: f32) -> f32 {
        self.trade_cost * self.prices[good] / wage + self.carrying[good]
    }

    fn full_cost(&self, good: GoodId, wage: f32) -> f32 {
        self.prices[good] * (1.0 + self.trade_cost) + self.carrying[good] * wage
    }
}

impl Markets {
    pub fn access(&self, catalog: &Catalog, town: usize, distance: f32) -> Access<'_> {
        Access {
            prices: &self.towns[town].prices,
            trade_cost: self.trade_cost,
            carrying: catalog.goods.iter().map(|g| self.travel_cost * g.bulk * distance).collect(),
        }
    }

    pub fn mismatch(&self) -> f32 {
        let (mut gap, mut volume) = (0.0, 0.0);
        for town in &self.towns {
            for g in 0..town.prices.len() {
                let (b, s, p) = (town.bought[g], town.sold[g] + town.imported[g], town.prices[g]);
                gap += (b - s).abs() * p;
                volume += (town.bought[g] + town.sold[g]) * p;
            }
        }
        if volume > 1e-6 { gap / volume } else { 0.0 }
    }

    pub fn clear(&mut self, catalog: &Catalog, bought: Vec<Vec<f32>>, sold: Vec<Vec<f32>>, mean_wage: f32) {
        let n = catalog.len();
        let towns = self.towns.len();
        let mut excess: Vec<Vec<f32>> = (0..towns).map(|t| (0..n).map(|g| bought[t][g] - sold[t][g]).collect()).collect();
        let mut imported = vec![vec![0.0; n]; towns];
        self.carried = 0.0;
        for g in 0..n {
            loop {
                let mut best: Option<(usize, usize, f32)> = None;
                for from in (0..towns).filter(|&t| excess[t][g] < -1e-6) {
                    for to in (0..towns).filter(|&t| excess[t][g] > 1e-6) {
                        let carry = self.travel_cost
                            * catalog.goods[g].bulk
                            * crate::map::WorldMap::distance(self.towns[from].at, self.towns[to].at);
                        let margin = self.towns[to].prices[g] - self.towns[from].prices[g] - carry;
                        if margin > 0.0 && best.is_none_or(|b| margin > b.2) {
                            best = Some((from, to, margin));
                        }
                    }
                }
                let Some((from, to, _)) = best else { break };
                let qty = (-excess[from][g]).min(excess[to][g]);
                excess[from][g] += qty;
                excess[to][g] -= qty;
                imported[from][g] -= qty;
                imported[to][g] += qty;
                self.carried += qty * self.towns[to].prices[g];
            }
        }
        for (t, town) in self.towns.iter_mut().enumerate() {
            for g in 0..n {
                let volume = bought[t][g] + sold[t][g];
                if volume > 1e-6 {
                    town.prices[g] *= (PRICE_SPEED * excess[t][g] / volume).clamp(-1.0, 1.0).exp();
                }
            }
            town.bought = bought[t].clone();
            town.sold = sold[t].clone();
            town.imported = imported[t].clone();
        }
        if mean_wage > 1e-6 {
            for town in &mut self.towns {
                for price in &mut town.prices {
                    *price /= mean_wage;
                }
            }
        }
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
    market: &Access,
    endowments: &[f32],
    owned: &[f32],
    job: Option<GoodId>,
    review: Option<f32>,
    work_hours: f32,
) -> Routes {
    let n = catalog.len();
    let direct: Vec<f32> = (0..n).map(|g| 1.0 / catalog.direct_rate(g, endowments, owned)).collect();
    let (mut hours, mut money, mut buy_share) = (vec![0.0; n], vec![0.0; n], vec![0.0f32; n]);
    let mut wage = 0.0;
    let current = job;
    let mut job = None;

    for _ in 0..ROUTE_PASSES {
        for &g in &catalog.build_order {
            let (mut h, mut m) = (direct[g], 0.0);
            for &(input, qty) in &catalog.goods[g].recipe.inputs {
                let share = buy_share[input];
                if share > 0.0 {
                    m += qty * share * market.prices[input];
                    h += qty * share * market.trading_hours(input, wage);
                }
                h += qty * (1.0 - share) * hours[input];
                m += qty * (1.0 - share) * money[input];
            }
            hours[g] = h;
            money[g] = m;
            buy_share[g] = if wage > 1e-6 { share_bought(h * wage + m, market.full_cost(g, wage)) } else { 0.0 };
        }
        let earning = |g: GoodId| (market.prices[g] - money[g]) / (hours[g] + market.carrying[g]);
        let with_tools = |g: GoodId| {
            catalog.goods[g]
                .recipe
                .tools
                .iter()
                .filter(|t| owned[t.good] <= 0.0)
                .map(|t| {
                    let mut kit = owned.to_vec();
                    kit[t.good] = 1.0;
                    let h = hours[g] - direct[g] + 1.0 / catalog.direct_rate(g, endowments, &kit);
                    let life = catalog.goods[t.good].lifetime.map_or(1.0, |l| l.mean());
                    let upkeep = market.full_cost(t.good, wage) / life / work_hours.max(1.0);
                    (market.prices[g] - money[g]) / (h + market.carrying[g]) - upkeep
                })
                .fold(earning(g), f32::max)
        };
        let chosen = match (review, current.filter(|&j| earning(j) > 1e-6)) {
            (None, Some(j)) => Some(j),
            (None, None) if current.is_none() => None,
            (draw, _) => job.filter(|&j| earning(j) > 1e-6).or_else(|| choose_job(&with_tools, n, draw.unwrap_or(0.5))),
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
            let share = buy_share[input];
            if share > 0.0 {
                flow.bought[input] += qty * share;
                flow.trading += qty * share * market.trading_hours(input, wage);
            }
            let sub = make[input].clone();
            flow.add(&sub, qty * (1.0 - share));
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
        flow.trading += market.carrying[j];
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
            (Some(m), Some(b)) => {
                let share = share_bought(m.total_hours(), b.total_hours());
                let mut mix = Flow::zero(n);
                mix.add(&m, 1.0 - share);
                mix.add(&b, share);
                mix
            }
            (Some(m), None) => m,
            (None, Some(b)) => b,
            (None, None) => unavailable(g),
        });
    }
    let unit_hours = flows.iter().map(Flow::total_hours).collect();
    Routes { flows, unit_hours, wage, job }
}

fn share_bought(make: f32, buy: f32) -> f32 {
    let scale = BUY_SPREAD * make.min(buy).max(1e-6);
    1.0 / (1.0 + ((buy - make) / scale).clamp(-30.0, 30.0).exp())
}

fn choose_job(earning: &dyn Fn(GoodId) -> f32, n: usize, draw: f32) -> Option<GoodId> {
    let best = (0..n).map(earning).fold(0.0, f32::max);
    if best <= 1e-6 {
        return None;
    }
    if draw < 0.0 {
        return (0..n).max_by(|&a, &b| earning(a).total_cmp(&earning(b)));
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

pub fn hours_for_needs(catalog: &Catalog, routes: &Routes, needs: &[(CharId, f32)]) -> f32 {
    needs
        .iter()
        .map(|&(c, amount)| {
            (0..catalog.len())
                .filter(|&g| catalog.provided(g, c) > 0.0)
                .map(|g| amount * routes.unit_hours[g] / catalog.provided(g, c))
                .fold(f32::INFINITY, f32::min)
        })
        .sum()
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

#[cfg(test)]
#[path = "tests/market.rs"]
mod tests;
