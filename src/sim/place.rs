//! Where people live: crowding, which thins out resources past what the land supports, and migration
//! towards places where meeting needs takes fewer hours.

use super::*;

#[derive(Resource, Default)]
pub struct Migrants(pub usize);

#[derive(Resource, Default)]
pub struct Occupancy {
    pub counts: Vec<f32>,
}

impl Occupancy {
    fn factor(&self, world_map: &WorldMap, spot: usize, extra: f32) -> f32 {
        world_map.crowding_factor(spot, &self.counts, extra)
    }
}

pub(super) fn count_occupancy(mut occupancy: ResMut<Occupancy>, mut people: Query<(&Person, &mut Crowding)>, world_map: Res<WorldMap>) {
    let mut counts = vec![0.0; POPULATION_COLS * POPULATION_ROWS];
    for (person, _) in &people {
        counts[WorldMap::spot(person.x, person.y)] += 1.0;
    }
    occupancy.counts = counts;
    for (person, mut crowding) in &mut people {
        crowding.0 = occupancy.factor(&world_map, WorldMap::spot(person.x, person.y), 0.0);
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn migrate(
    catalog: Res<Catalog>,
    markets: Res<Markets>,
    population: Res<Population>,
    world_map: Res<WorldMap>,
    occupancy: Res<Occupancy>,
    mut migrants: ResMut<Migrants>,
    mut people: Query<
        (&mut Person, &mut Home, &mut Endowments, &mut Moves, &mut PersonRng, &Needs, &Tools, &TimeUse),
        Without<Child>,
    >,
) {
    let rules = population.migration;
    let moved = std::sync::atomic::AtomicUsize::new(0);
    people.par_iter_mut().for_each(|(mut person, mut home, mut endowments, mut moves, mut rng, needs, tools, time)| {
        if rules.chance <= 0.0 || rng.0.next_f32() >= rules.chance {
            return;
        }
        let owned = tools.owned();
        let work_hours = time.hours.iter().sum::<f32>() + time.trading;
        let score = |endowments: &[f32], town: usize, distance: f32| {
            let access = markets.access(&catalog, town, distance);
            let routes = plan_routes(&catalog, &access, endowments, &owned, None, BEST_JOB, work_hours);
            hours_for_needs(&catalog, &routes, &needs.0)
        };
        let spot_here = WorldMap::spot(person.x, person.y);
        let here = score(&effective(&endowments.0, occupancy.factor(&world_map, spot_here, 0.0)), home.town, home.distance);
        let mut best: Option<(f32, (f32, f32), Vec<f32>, usize, f32)> = None;
        for _ in 0..rules.look_at {
            let at = world_map.settle(rng.0.next_f32(), rng.0.next_f32(), rng.0.next_f32());
            let spot = WorldMap::spot(at.0, at.1);
            let there: Vec<f32> = population
                .endowments
                .iter()
                .zip(&world_map.fields)
                .map(|(spread, field)| spread.draw(field[spot]))
                .collect();
            let (town, distance) = world_map.nearest_town(at);
            let extra = if spot == spot_here { 0.0 } else { 1.0 };
            let hours = score(&effective(&there, occupancy.factor(&world_map, spot, extra)), town, distance);
            if best.as_ref().is_none_or(|b| hours < b.0) {
                best = Some((hours, at, there, town, distance));
            }
        }
        if let Some((hours, at, there, town, distance)) = best
            && hours < here * (1.0 - rules.min_gain)
        {
            (person.x, person.y) = at;
            endowments.0 = there;
            *home = Home { town, distance };
            moves.0 += 1;
            moved.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });
    migrants.0 = moved.into_inner();
}
