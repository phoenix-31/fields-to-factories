//! Births, deaths and growing up. Children are fed by a parent, whose needs grow with theirs; living
//! standards set the chances of a birth and of dying; a dying parent's tools pass to their eldest child.

use std::collections::HashMap;

use super::*;

#[derive(Resource, Default)]
pub struct Lineage {
    pub next_id: u64,
    pub births: usize,
    pub deaths: usize,
}

pub(super) fn grow_up(
    mut commands: Commands,
    population: Res<Population>,
    world_map: Res<WorldMap>,
    mut people: Query<(Entity, &mut Age, &Person, &mut Endowments, Has<Child>)>,
) {
    for (entity, mut age, person, mut endowments, child) in &mut people {
        age.0 += 1;
        if child && age.0 >= population.life.adult_at {
            commands.entity(entity).remove::<Child>();
            let spot = WorldMap::spot(person.x, person.y);
            endowments.0 =
                population.endowments.iter().zip(&world_map.fields).map(|(spread, field)| spread.draw(field[spot])).collect();
        }
    }
}

pub(super) fn gather_dependents(
    population: Res<Population>,
    mut adults: Query<(&Person, &Home, &mut Dependents), Without<Child>>,
    mut children: Query<(&Child, &Age, &Needs, &mut Person, &mut Home)>,
) {
    for (_, _, mut dependents) in &mut adults {
        dependents.count = 0;
        dependents.needs.iter_mut().for_each(|n| *n = 0.0);
    }
    for (child, age, needs, mut person, mut home) in &mut children {
        let Ok((parent, parent_home, mut dependents)) = adults.get_mut(child.fed_by) else { continue };
        dependents.count += 1;
        let share = population.life.child_share(age.0);
        for (total, &(_, amount)) in dependents.needs.iter_mut().zip(&needs.0) {
            *total += amount * share;
        }
        let (x, y) = (parent.x, parent.y);
        if (person.x - x).abs() > 0.5 || (person.y - y).abs() > 0.5 {
            person.x = (x + 0.2).min((POPULATION_COLS - 1) as f32);
            person.y = (y + 0.2).min((POPULATION_ROWS - 1) as f32);
        }
        *home = Home { town: parent_home.town, distance: parent_home.distance };
    }
}

struct Death {
    who: Entity,
    at: (f32, f32),
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn births_and_deaths(
    mut commands: Commands,
    catalog: Res<Catalog>,
    population: Res<Population>,
    world_map: Res<WorldMap>,
    mut lineage: ResMut<Lineage>,
    mut people: Query<(
        Entity,
        &Person,
        &Age,
        &NeedsMet,
        &TimeUse,
        &Tastes,
        &Family,
        Option<&Child>,
        &mut PersonRng,
    )>,
    mut tools: Query<&mut Tools>,
) {
    let life = population.life;
    lineage.births = 0;
    lineage.deaths = 0;
    let needs_met: HashMap<Entity, f32> = people.iter().map(|p| (p.0, p.3.0)).collect();
    let mut deaths = Vec::new();
    let mut births = Vec::new();
    for (entity, person, age, met, time, tastes, _, child, mut rng) in &mut people {
        let household = child.and_then(|c| needs_met.get(&c.fed_by)).copied().unwrap_or(met.0);
        if rng.0.next_f32() < life.death_chance(age.0, household) {
            deaths.push(Death { who: entity, at: (person.x, person.y) });
        } else if child.is_none() && rng.0.next_f32() < life.birth_chance(age.0, met.0, time.leisure) {
            births.push((entity, (person.x, person.y), tastes.clone()));
        }
    }

    let dying: Vec<Entity> = deaths.iter().map(|d| d.who).collect();
    let alive = |e: &Entity| !dying.contains(e);
    let mut offspring: HashMap<Entity, Vec<(Entity, u32, bool)>> = HashMap::new();
    let mut adults = Vec::new();
    for (entity, person, age, _, _, _, family, child, _) in &people {
        if let Some(parent) = family.parent {
            offspring.entry(parent).or_default().push((entity, age.0, child.is_some()));
        }
        if child.is_none() && alive(&entity) {
            adults.push((entity, (person.x, person.y)));
        }
    }
    let fed_by: HashMap<Entity, Vec<Entity>> = people.iter().fold(HashMap::new(), |mut map, p| {
        if let Some(c) = p.7 {
            map.entry(c.fed_by).or_default().push(p.0);
        }
        map
    });

    for death in &deaths {
        lineage.deaths += 1;
        let heir = offspring
            .get(&death.who)
            .into_iter()
            .flatten()
            .filter(|(e, _, _)| alive(e))
            .max_by_key(|&&(e, age, _)| (age, e))
            .copied();
        if let Some((heir, _, _)) = heir
            && let Ok([dead, mut heir]) = tools.get_many_mut([death.who, heir])
        {
            inherit(&mut heir, &dead);
        }
        let guardian = heir.filter(|&(_, _, child)| !child).map(|(e, _, _)| e).or_else(|| {
            adults
                .iter()
                .filter(|(e, _)| *e != death.who)
                .min_by(|a, b| WorldMap::distance(a.1, death.at).total_cmp(&WorldMap::distance(b.1, death.at)))
                .map(|(e, _)| *e)
        });
        for &orphan in fed_by.get(&death.who).into_iter().flatten().filter(|e| alive(e)) {
            match guardian {
                Some(g) => {
                    commands.entity(orphan).insert(Child { fed_by: g });
                }
                None => {
                    commands.entity(orphan).despawn();
                    lineage.deaths += 1;
                }
            }
        }
        commands.entity(death.who).despawn();
    }

    for (parent, (x, y), tastes) in births {
        let id = lineage.next_id;
        lineage.next_id += 1;
        let mut rng = Rng(person_seed(id) ^ 0x5DEE_CE66);
        let at = ((x + 0.2).min((POPULATION_COLS - 1) as f32), (y + 0.2).min((POPULATION_ROWS - 1) as f32));
        let child = new_person(at, person_seed(id), &mut rng, Some(&tastes), &catalog, &population, &world_map);
        commands.spawn(child).insert((Age(0), Child { fed_by: parent }, Family { parent: Some(parent) }));
        lineage.births += 1;
    }
}

pub(super) fn inherit(heir: &mut Tools, dead: &Tools) {
    for good in 0..dead.years_left.len() {
        if dead.years_left[good] > heir.years_left[good] {
            heir.years_left[good] = dead.years_left[good];
            heir.life[good] = dead.life[good];
        }
        heir.progress[good] = (heir.progress[good] + dead.progress[good]).min(1.0);
    }
}
