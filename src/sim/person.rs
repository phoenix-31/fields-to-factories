//! A person: the components each one carries, how the population is first spawned, and tools wearing out.

use super::*;

#[derive(Component)]
pub struct Person {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Home {
    pub town: usize,
    pub distance: f32,
}

#[derive(Component)]
pub struct Endowments(pub Vec<f32>);

#[derive(Component)]
pub struct Needs(pub Vec<(CharId, f32)>);

#[derive(Component)]
pub struct Tastes {
    pub preferences: Vec<(CharId, Taste)>,
    pub leisure: f32,
}

#[derive(Component, Default, Clone)]
pub struct TimeUse {
    pub hours: Vec<f32>,
    pub trading: f32,
    pub leisure: f32,
}

impl TimeUse {
    pub(super) fn new(goods: usize) -> Self {
        Self { hours: vec![0.0; goods], trading: 0.0, leisure: 0.0 }
    }
}

#[derive(Component)]
pub struct Tools {
    pub years_left: Vec<f32>,
    pub progress: Vec<f32>,
    pub life: Vec<f32>,
}

impl Tools {
    fn new(goods: usize) -> Self {
        Self { years_left: vec![0.0; goods], progress: vec![0.0; goods], life: vec![0.0; goods] }
    }

    pub fn owned(&self) -> Vec<f32> {
        self.years_left.iter().map(|&y| if y > 0.0 { 1.0 } else { 0.0 }).collect()
    }
}

#[derive(Component)]
pub struct Trades {
    pub bought: Vec<f32>,
    pub sold: Vec<f32>,
}

#[derive(Component, Default)]
pub struct Wage(pub f32);

#[derive(Component, Default)]
pub struct Job(pub Option<GoodId>);

#[derive(Component, Default)]
pub struct Moves(pub u32);

#[derive(Component)]
pub struct Crowding(pub f32);

#[derive(Component, Default)]
pub struct NeedsMet(pub f32);

#[derive(Component)]
pub struct HourValue(pub f32);

pub(super) fn spawn_people(
    mut commands: Commands,
    mut rng: ResMut<WorldRng>,
    catalog: Res<Catalog>,
    population: Res<Population>,
    world_map: Res<WorldMap>,
    count: Res<CountOverride>,
) {
    let people = count.0.unwrap_or_else(|| population.count.draw(rng.0.next_f32()).round() as usize).max(1);
    for index in 0..people {
        let (pick, jx, jy) = (rng.0.next_f32(), rng.0.next_f32(), rng.0.next_f32());
        let at = world_map.settle(pick, jx, jy);
        let seed = SEED ^ (index as u64).wrapping_mul(0xA24B_AED4_963E_E407);
        commands.spawn(new_person(at, seed, &mut rng.0, &catalog, &population, &world_map));
    }
}

pub(super) fn new_person(
    (x, y): (f32, f32),
    seed: u64,
    rng: &mut Rng,
    catalog: &Catalog,
    population: &Population,
    world_map: &WorldMap,
) -> impl Bundle {
    let n = catalog.len();
    let spot = WorldMap::spot(x, y);
    let endowments: Vec<f32> =
        population.endowments.iter().zip(&world_map.fields).map(|(spread, field)| spread.draw(field[spot])).collect();
    let needs: Vec<(CharId, f32)> =
        population.needs.iter().map(|&(c, amount)| (c, amount.draw(rng.next_f32()))).collect();
    let preferences = population
        .preferences
        .iter()
        .map(|&(c, curve)| {
            let taste = match curve {
                Curve::Log { weight } => Taste::Log { weight: weight.draw(rng.next_f32()) },
                Curve::Saturating { weight, scale } => Taste::Saturating { weight: weight.draw(rng.next_f32()), scale },
            };
            (c, taste)
        })
        .collect();
    let leisure = population.leisure.draw(rng.next_f32());
    let (town, distance) = world_map.nearest_town((x, y));
    (
        (Person { x, y }, Home { town, distance }, Endowments(endowments), Needs(needs)),
        Tastes { preferences, leisure },
        Tools::new(n),
        Trades { bought: vec![0.0; n], sold: vec![0.0; n] },
        Wage::default(),
        Job::default(),
        Moves::default(),
        Crowding(1.0),
        TimeUse::new(n),
        NeedsMet::default(),
        HourValue(INITIAL_HOUR_VALUE),
        PersonRng(Rng(seed)),
    )
}

pub(super) fn replace_tools(catalog: Res<Catalog>, mut people: Query<(&mut Tools, &mut PersonRng)>) {
    for (mut tools, mut rng) in &mut people {
        for (good, def) in catalog.goods.iter().enumerate() {
            let Some(lifetime) = def.lifetime else { continue };
            tools.years_left[good] = (tools.years_left[good] - 1.0).max(0.0);
            if tools.years_left[good] <= 0.0 && tools.progress[good] >= PAID_FOR {
                tools.progress[good] = (tools.progress[good] - 1.0).max(0.0);
                tools.years_left[good] = lifetime.draw(rng.0.next_f32()).round().max(1.0);
                tools.life[good] = tools.years_left[good];
            }
            tools.progress[good] = tools.progress[good].min(1.0);
        }
    }
}
