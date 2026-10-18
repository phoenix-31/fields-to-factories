//! Each person's year: choosing a job and routes, meeting needs first, then spending spare quarter-hours
//! on whatever their tastes value most, including tools and saving for the next one.

use super::*;

pub(super) fn effective(endowments: &[f32], crowding: f32) -> Vec<f32> {
    endowments.iter().map(|e| e * crowding).collect()
}

pub(super) fn enjoyment(tastes: &Tastes, need: &[f32], amounts: &[f32]) -> f32 {
    tastes
        .preferences
        .iter()
        .map(|&(c, taste)| taste.value((amounts[c] - need[c]).max(0.0)))
        .sum()
}

pub(super) fn leisure_value(tastes: &Tastes, hours: f32) -> f32 {
    tastes.leisure * (1.0 + hours).ln()
}

pub(super) fn annuity(years: f32, rate: f32) -> f32 {
    if rate <= 0.0 { years } else { (1.0 - (1.0 + rate).powf(-years)) / rate }
}

pub(super) struct Situation<'a> {
    catalog: &'a Catalog,
    routes: &'a Routes,
    needs: &'a Needs,
    tastes: &'a Tastes,
    tools: &'a Tools,
    owned: &'a [f32],
    last_year: &'a TimeUse,
    work_hours: f32,
    work_slices: usize,
    hour_value: f32,
    discount_rate: f32,
}

pub(super) fn tool_gain(p: &Situation, tool: GoodId, extra: f32, made: &[f32]) -> f32 {
    let catalog = p.catalog;
    let Some(lifetime) = catalog.goods[tool].lifetime else {
        return 0.0;
    };
    let next: Vec<f32> = (0..catalog.len())
        .map(|t| {
            let complete = p.tools.progress[t] + made[t] >= PAID_FOR;
            if p.tools.years_left[t] > 1.0 || complete { 1.0 } else { 0.0 }
        })
        .collect();
    let years_left = p.tools.years_left[tool];
    let due = if years_left > 1.0 { 1.0 - (years_left - 1.0) / p.tools.life[tool].max(years_left) } else { 1.0 };
    let room = due - p.tools.progress[tool] - made[tool];
    if room <= 1.0 - PAID_FOR {
        return 0.0;
    }
    let mut without = next.clone();
    without[tool] = 0.0;
    let mut with_tool = next;
    with_tool[tool] = 1.0;
    let job_share = |good: GoodId| {
        p.routes.job.map_or(0.0, |j| {
            let flow = &p.routes.flows[j];
            flow.hours[good] / flow.total_hours().max(1e-6)
        })
    };
    let saved: f32 = catalog
        .goods
        .iter()
        .enumerate()
        .filter(|(_, def)| def.recipe.tools.iter().any(|t| t.good == tool))
        .map(|(good, _)| {
            let hours = p.last_year.hours[good].max(p.work_hours * job_share(good));
            hours / catalog.hours_factor(good, p.owned)
                * (catalog.hours_factor(good, &without) - catalog.hours_factor(good, &with_tool))
        })
        .sum();
    saved * p.hour_value * annuity(lifetime.mean(), p.discount_rate) * extra.min(room)
}

pub(super) struct DayPlan {
    time: TimeUse,
    made: Vec<f32>,
    bought: Vec<f32>,
    sold: Vec<f32>,
    needs_met: f32,
    hour_value: f32,
}

pub(super) fn plan_day(p: &Situation) -> DayPlan {
    let catalog = p.catalog;
    let n = catalog.len();
    let unit_hours = &p.routes.unit_hours;
    let mut need = vec![0.0; catalog.characteristics.len()];
    for &(c, amount) in &p.needs.0 {
        need[c] += amount;
    }

    let mut plan = DayPlan {
        time: TimeUse::new(n),
        made: vec![0.0; n],
        bought: vec![0.0; n],
        sold: vec![0.0; n],
        needs_met: 1.0,
        hour_value: 0.0,
    };
    let mut amounts = vec![0.0; catalog.characteristics.len()];
    let work = |good: GoodId, plan: &mut DayPlan, amounts: &mut Vec<f32>| {
        let units = SLICE / unit_hours[good];
        let flow = &p.routes.flows[good];
        plan.made[good] += units;
        for &(c, per_unit) in &catalog.goods[good].provides {
            amounts[c] += units * per_unit;
        }
        plan.time.trading += units * flow.trading;
        for sector in 0..n {
            plan.time.hours[sector] += units * flow.hours[sector];
            plan.bought[sector] += units * flow.bought[sector];
            plan.sold[sector] += units * flow.sold[sector];
        }
    };

    let short = hours_for_needs(catalog, p.routes, &p.needs.0) > p.work_slices as f32 * SLICE;
    let best_tool = |plan: &DayPlan| {
        (0..n)
            .filter(|&t| catalog.goods[t].is_tool() && unit_hours[t].is_finite())
            .map(|t| (t, tool_gain(p, t, SLICE / unit_hours[t], &plan.made)))
            .filter(|&(_, gain)| gain > p.hour_value * SLICE)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(t, _)| t)
    };

    let mut slices = p.work_slices;
    for &(c, amount) in &p.needs.0 {
        let cheapest = (0..n)
            .filter(|&g| catalog.provided(g, c) > 0.0 && unit_hours[g].is_finite())
            .min_by(|&a, &b| {
                (unit_hours[a] / catalog.provided(a, c)).total_cmp(&(unit_hours[b] / catalog.provided(b, c)))
            });
        let Some(good) = cheapest else { continue };
        while slices > 0 && amounts[c] < amount {
            let next = if short { best_tool(&plan).unwrap_or(good) } else { good };
            work(next, &mut plan, &mut amounts);
            slices -= 1;
        }
    }

    let mut marginal = 0.0;
    for _ in 0..slices {
        let leisure = leisure_value(p.tastes, plan.time.leisure + SLICE) - leisure_value(p.tastes, plan.time.leisure);
        let mut best = (leisure, None);
        let now = enjoyment(p.tastes, &need, &amounts);
        for good in (0..n).filter(|&g| unit_hours[g].is_finite()) {
            let extra = SLICE / unit_hours[good];
            let mut after = amounts.clone();
            for &(c, per_unit) in &catalog.goods[good].provides {
                after[c] += extra * per_unit;
            }
            let gain = enjoyment(p.tastes, &need, &after) - now + tool_gain(p, good, extra, &plan.made);
            if gain > best.0 {
                best = (gain, Some(good));
            }
        }
        marginal = best.0;
        match best.1 {
            Some(good) => work(good, &mut plan, &mut amounts),
            None => plan.time.leisure += SLICE,
        }
    }

    plan.time.leisure += (SLICES - p.work_slices) as f32 * SLICE;
    plan.needs_met = p.needs.0.iter().map(|&(c, amount)| amounts[c] / amount).fold(1.0, f32::min);
    plan.hour_value = if plan.needs_met < 0.999 {
        leisure_value(p.tastes, SLICE) / SLICE / plan.needs_met.max(0.1)
    } else {
        marginal / SLICE
    };
    plan
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(super) struct Planner {
    home: &'static Home,
    endowments: &'static Endowments,
    crowding: &'static Crowding,
    needs: &'static Needs,
    tastes: &'static Tastes,
    tools: &'static mut Tools,
    trades: &'static mut Trades,
    wage: &'static mut Wage,
    job: &'static mut Job,
    rng: &'static mut PersonRng,
    time: &'static mut TimeUse,
    needs_met: &'static mut NeedsMet,
    hour_value: &'static mut HourValue,
    age: &'static Age,
    dependents: &'static Dependents,
}

pub(super) fn allocate_time(
    catalog: Res<Catalog>,
    markets: Res<Markets>,
    population: Res<Population>,
    mut people: Query<Planner, Without<Child>>,
) {
    let life = population.life;
    people.par_iter_mut().for_each(|mut p| {
        let owned = p.tools.owned();
        let review = (p.rng.0.next_f32() < JOB_REVIEW_CHANCE).then(|| p.rng.0.next_f32());
        let access = markets.access(&catalog, p.home.town, p.home.distance);
        let work_hours = p.time.hours.iter().sum::<f32>() + p.time.trading;
        let endowments = effective(&p.endowments.0, p.crowding.0);
        let routes = plan_routes(&catalog, &access, &endowments, &owned, p.job.0, review, work_hours);
        p.job.0 = routes.job;
        let household = Needs(p.needs.0.iter().zip(&p.dependents.needs).map(|(&(c, own), &kids)| (c, own + kids)).collect());
        let work_slices = if p.age.0 >= life.old_at { (SLICES as f32 * life.old_work).round() as usize } else { SLICES };
        let plan = plan_day(&Situation {
            catalog: &catalog,
            routes: &routes,
            needs: &household,
            tastes: p.tastes,
            tools: &p.tools,
            owned: &owned,
            last_year: &p.time,
            work_hours,
            work_slices,
            hour_value: p.hour_value.0,
            discount_rate: population.discount_rate,
        });
        for (good, def) in catalog.goods.iter().enumerate() {
            if def.is_tool() {
                p.tools.progress[good] += plan.made[good];
            }
        }
        p.trades.bought = plan.bought;
        p.trades.sold = plan.sold;
        p.wage.0 = routes.wage;
        *p.time = plan.time;
        p.needs_met.0 = plan.needs_met;
        p.hour_value.0 = plan.hour_value;
    });
}
