//! Yearly samples of how working-age people spend their time, prices, tools, towns, births and deaths,
//! and writing them out as CSV.

use super::*;

#[derive(Clone)]
pub struct Sample {
    pub year: u32,
    pub rates: Vec<f32>,
    pub prices: Vec<Vec<f32>>,
    pub shares: Vec<f32>,
    pub town_shares: Vec<Vec<f32>>,
    pub trading_share: f32,
    pub leisure_share: f32,
    pub hungry_share: f32,
    pub coverage: Vec<f32>,
    pub specialisation: f32,
    pub traded: f32,
    pub carried: f32,
    pub mismatch: f32,
    pub moved: f32,
    pub town_people: Vec<f32>,
    pub population: f32,
    pub children: f32,
    pub births: f32,
    pub deaths: f32,
    pub old: f32,
    pub old_cared_for: f32,
    pub old_short: f32,
}

#[derive(Resource, Default)]
pub struct History(pub Vec<Sample>);

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn record_history(
    clock: Res<Clock>,
    catalog: Res<Catalog>,
    markets: Res<Markets>,
    migrants: Res<Migrants>,
    lineage: Res<Lineage>,
    population_def: Res<Population>,
    mut history: ResMut<History>,
    people: Query<(&TimeUse, &NeedsMet, &Tools, &Home, Has<Child>, &Age, &Care)>,
) {
    let goods = catalog.len();
    let towns = markets.towns.len();
    let (mut town_hours, mut town_people) = (vec![vec![0.0; goods]; towns], vec![0.0f32; towns]);
    let mut town_workers = vec![0.0f32; towns];
    let (mut hours, mut coverage) = (vec![0.0; goods], vec![0.0; goods]);
    let (mut trading, mut leisure, mut hungry, mut specialisation, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0f32);
    let (mut population, mut children) = (0.0, 0.0);
    let (mut old, mut old_cared_for, mut old_short) = (0.0, 0.0, 0.0);
    for (time, needs_met, tools, home, child, age, care) in &people {
        if age.0 >= population_def.life.old_at {
            old += 1.0;
            if care.by.is_some() {
                old_cared_for += 1.0;
            }
            if needs_met.0 < 0.999 {
                old_short += 1.0;
            }
        }
        town_people[home.town] += 1.0;
        population += 1.0;
        if child {
            children += 1.0;
            continue;
        }
        town_workers[home.town] += 1.0;
        for good in 0..goods {
            hours[good] += time.hours[good];
            town_hours[home.town][good] += time.hours[good];
            coverage[good] += if tools.years_left[good] > 0.0 { 1.0 } else { 0.0 };
        }
        let worked: f32 = time.hours.iter().sum();
        if worked > 0.0 {
            specialisation += time.hours.iter().copied().fold(0.0, f32::max) / worked;
        }
        trading += time.trading;
        leisure += time.leisure;
        if needs_met.0 < 0.999 {
            hungry += 1.0;
        }
        n += 1.0;
    }
    let n = f32::max(n, 1.0);
    let total_hours = n * WAKING_HOURS;
    let worked_hours: f32 = hours.iter().sum();
    let traded_hours: f32 = markets
        .towns
        .iter()
        .map(|t| t.sold.iter().zip(&t.prices).map(|(s, p)| s * p).sum::<f32>())
        .sum();
    history.0.push(Sample {
        year: clock.year,
        rates: catalog.goods.iter().map(|g| g.recipe.rate).collect(),
        prices: markets.towns.iter().map(|t| t.prices.clone()).collect(),
        shares: hours.iter().map(|h| h / total_hours).collect(),
        town_shares: town_hours
            .iter()
            .zip(&town_workers)
            .map(|(h, p)| h.iter().map(|x| x / (p * WAKING_HOURS).max(1e-6)).collect())
            .collect(),
        trading_share: trading / total_hours,
        leisure_share: leisure / total_hours,
        hungry_share: hungry / n,
        coverage: coverage.iter().map(|c| c / n).collect(),
        specialisation: specialisation / n,
        traded: (traded_hours / worked_hours.max(1e-6)).min(1.0),
        carried: markets.carried / traded_hours.max(1e-6),
        mismatch: markets.mismatch(),
        moved: migrants.0 as f32 / n,
        town_people: town_people.clone(),
        population,
        children,
        births: lineage.births as f32,
        deaths: lineage.deaths as f32,
        old,
        old_cared_for,
        old_short,
    });
}

pub fn write_csv(
    catalog: &Catalog,
    towns: &[String],
    samples: &[Sample],
    out: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut header = vec!["year".to_string()];
    for g in &catalog.goods {
        header.push(format!("{}_rate", g.name));
    }
    for town in towns {
        for g in &catalog.goods {
            header.push(format!("{town}: {}_price", g.name));
        }
    }
    for g in &catalog.goods {
        header.push(format!("{}_share", g.sector));
    }
    let groups: Vec<_> = (0..catalog.sectors.len()).filter(|&s| !catalog.sectors[s].children.is_empty()).collect();
    for &group in &groups {
        header.push(format!("{}_share", catalog.sector_path(group)));
    }
    for town in towns {
        for g in &catalog.goods {
            header.push(format!("{town}: {}_share", g.sector));
        }
    }
    header.extend(
        ["trading_share", "leisure_share", "hungry_share", "specialisation", "traded", "carried", "mismatch", "moved"]
            .map(String::from),
    );
    for town in towns {
        header.push(format!("{town}: people"));
    }
    for g in catalog.goods.iter().filter(|g| g.is_tool()) {
        header.push(format!("{}_coverage", g.name));
    }
    header.extend(["population", "children", "births", "deaths", "old", "old_cared_for", "old_short"].map(String::from));
    writeln!(out, "{}", header.join(","))?;

    for s in samples {
        let mut row = vec![s.year.to_string()];
        row.extend(s.rates.iter().map(|v| format!("{v:.4}")));
        row.extend(s.prices.iter().flatten().map(|v| format!("{v:.4}")));
        row.extend(s.shares.iter().map(|v| format!("{v:.4}")));
        for &group in &groups {
            row.push(format!("{:.4}", catalog.sectors[group].goods.iter().map(|&g| s.shares[g]).sum::<f32>()));
        }
        row.extend(s.town_shares.iter().flatten().map(|v| format!("{v:.4}")));
        for v in [s.trading_share, s.leisure_share, s.hungry_share, s.specialisation, s.traded, s.carried, s.mismatch, s.moved] {
            row.push(format!("{v:.4}"));
        }
        row.extend(s.town_people.iter().map(|v| format!("{v:.0}")));
        for (good, def) in catalog.goods.iter().enumerate() {
            if def.is_tool() {
                row.push(format!("{:.4}", s.coverage[good]));
            }
        }
        row.extend([s.population, s.children, s.births, s.deaths, s.old, s.old_cared_for, s.old_short].map(|v| format!("{v:.0}")));
        writeln!(out, "{}", row.join(","))?;
    }
    Ok(())
}
