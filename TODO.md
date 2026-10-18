# To do

A working list, reviewed together. Items move from **Ideas** to **Next** once we've agreed how they should work, and to the [CHANGELOG](CHANGELOG.md) when they're done. Open questions are marked **?**.

## Next: the big four

Suggested order: 1 → 2 → 4 → 3. 1 is done. The hierarchy makes adding goods bearable. Multiple recipes are the slot that discoveries fill. Deposits give discoveries something to find.

### 1. Sector hierarchy: done

See the CHANGELOG. Follow-ups:

- [ ] Trading & carrying sits at the top level for now. Once there are real services (merchants, transport), make it part of **tertiary**.
- [ ] The Towns tab and hover details still list leaf activities. Should they follow the chart's level too?
- [ ] The first overview card's label wraps ("of the day spent wheat farming"); a shorter name or a wider card.

### 2. Multiple recipes per good

Today each good has exactly one way to make it.

- A good lists `recipes: [...]`, each with its own name, sector, rate, `uses`, inputs and tools. People use whichever is cheapest for them, the same way they already choose between making and buying.
- Examples: iron from a charcoal bloomery or a coal furnace; cloth by hand or on a loom; food from fields or from pasture as meat.
- Code impact: hours and sectors are tracked per recipe rather than per good; affects the catalog, `plan_routes` and the production tree.

### 3. Technology as discovery

Steady yearly improvement has been **removed** (your call: it blew up long runs). Without it the economy settles by year 50 and stays put, so discoveries are now the only source of progress. Real history is lumpy: new crops, coal, the blast furnace. Discoveries create winners and losers, and change where it pays to live, which migration then acts on.

- **Discoveries** unlock a new resource, recipe or good.
- What drives a discovery (agreed): **learning by doing, multiplied by spare time.** Each person's chance grows with their experience in a related sector *and* with their leisure. Someone farming all day on the edge of hunger rarely invents a plough; someone with a couple of spare hours might. This makes a feedback loop: discoveries free time, and free time breeds discoveries. It also means the poorest regions fall behind until trade or migration lifts them.
- **?** How experience counts: a person's own years in the sector, the town's cumulative output, or both?
- **?** Does a discovery appear everywhere at once, or start in one town and spread along trade routes?

### 4. Uneven resources (power law)

Ore ranges smoothly from 0.1 to 1.6 over much of the map, so plenty of people can mine reasonably well. Real resources are rare and rich: a few places hold most of the ore, and the best farmland too.

- **A power law for every resource** (agreed). Keep the terrain shapes (valleys still have the best land), but map each spot's rank to a heavy-tailed (Pareto) quality instead of an even range, with a `tail` setting per resource. A few great spots, many poor ones, so people must move or trade.
- Ore could go further: a few point deposits, near zero elsewhere, so mining towns appear.
- Crowding needs to become **per resource**: a rich mine supports a set number of miners, so it shouldn't thin out the land of the farmers living beside it. Otherwise everyone piles onto the mine.
- Endowments are currently required to be above zero; deposits need near-zero to be allowed.
- **?** Should deposits deplete, so mines run out and mining towns boom and bust?

## Refactors to do before things get more complicated

Each is tied to the feature that will need it. The tests guard behaviour while these are done.

- [ ] **Split `sim.rs`** (840 lines). It holds the components, spawning, the yearly plan, tools, migration, history and the CSV. Split it into `sim/` modules. *Before births and deaths*, which add a person lifecycle.
- [ ] **One place to make a person.** Spawning builds about 15 components inline. Births will need the same, so make one constructor. Per-person random seeds come from the spawn order; births need a seed source that keeps runs repeatable (the tests check this).
- [ ] **Typed per-good vectors.** Hours, purchases, sales, prices, tool progress and coverage are all plain `Vec<f32>` indexed by good, with length checks scattered through the UI. Multiple recipes will add a second index (per recipe) beside the per-good one, which is easy to mix up. Use typed indices or a `PerGood<T>` wrapper. *Before multiple recipes.*
- [ ] **One recipe per good is baked in:** `Good.recipe`, `direct_rate(good)`, hours tracked by good, and each good is its own sector leaf. Separate "what is made" from "how". *This is most of the work of multiple recipes.*
- [ ] **Break up `plan_routes`** (126 lines). It runs a fixed four-pass loop that works out make-or-buy shares, the wage and the job choice together, then builds the flows. Split it into costs given a wage, job choice, and flows, so each can be tested alone.
- [ ] **Name the time units.** One "day" stands for every day of a year, so an iron plough "costs 22 hours" of a 16-hour day. It's consistent but confusing; make the units explicit.
- [ ] **Move tuning constants into the data files.** Price speed, job and buy spreads, route passes, job review chance, the paid-for threshold and the starting hour value are scattered constants in `market.rs` and `sim.rs`.
- [ ] **Towns are fixed indices everywhere:** `Home.town`, `Markets.towns`, per-town CSV columns, the Towns tab and the merchants' loop. Markets that come and go need stable ids and a CSV that can grow. *Before emergent markets* (see below).
- [ ] **Split `ui/panel.rs`** (680 lines) into one file per tab.
- [ ] **Performance.** About 40 ms a year for 1,500 people. Each person plans routes every year, and again for every place they consider moving to. Profile before the population can grow through births.

## Model flaws found

### Ploughs slowly disappear: fixed

Found in your 7,600-year no-technology run. Fixed with a sinking fund and choosing jobs with their tools (fixes 1 and 2), plus a rounding bug found on the way that left people stuck at 99.99% of a tool. See the CHANGELOG for the numbers.

- [ ] **Credit** (saved for later): borrow for a tool and repay from the hours it saves.
- [x] Hungry people value saved hours *more* (agreed), and can invest while short. Hunger now clears by year 8.
- [ ] **?** Hunger has no lasting cost: someone can go very short for a year to buy a plough and be fine afterwards. Should going short have consequences (weaker work, illness, death and births) so investing while hungry is a real gamble?

## Quality of life

- [x] Chart: hover to read the values at a year.
- [ ] Chart: zoom to the last N years.
- [ ] Keyboard shortcuts: space to pause, → to step a year, R to reset.
- [ ] A seed setting and a "new seed" button. The random seed is fixed, so every run is identical.
- [ ] Compare runs: keep the last run's lines faintly on the chart after a Reset.
- [ ] Map pan and zoom.
- [ ] Mark the settings that only apply after a Reset.
- [x] Tests: 34 to start with (`cargo test --release`). Add tests alongside each new feature.
- [x] Fix the three existing clippy warnings.
- [ ] Faster headless runs (about 40 ms a year with 1,500 people).
- [x] Needs-unmet, tool and specialisation lines moved off the hours chart onto their own chart.

## Markets that emerge instead of fixed towns

The four towns are placed by hand in `map.ron`, and people's settlement is pulled towards them. So the most important geography is assumed, not grown. Options:

1. **Emergent market towns.** Markets are founded where many people live far from any market and trade would pay; they grow with trade and close when too few use them. People trade at the nearest open market. This is close to central place theory (Christaller, Lösch): market towns spaced by how far people will walk.
2. **No towns at all.** Every map cell has its own prices, and carriers move goods between neighbouring cells when the price gap pays. Towns appear only as places where trade happens to concentrate. Most elegant, but the biggest change and the slowest to run.
3. **Keep fixed towns** until births, deaths and discoveries are in.

Things any option needs: stable market ids, a CSV and Towns tab that cope with markets appearing and closing, settlement that doesn't assume towns, and a rule for which market a person uses (nearest open one, or the best net of carrying).

- [ ] **?** Which option, and when?

## Ideas, not yet planned

- **Births and deaths** (on the cards), starting from a smaller population. Going short of food should then carry real risk.
- **A plough for every household.** About 750 people who farm only for themselves each own an iron plough. A tool's value scales with hours used, and there's no way to share or hire one. Historically, ploughs and teams were shared, or ploughing was hired as a service; a cottager with a small plot dug with a spade. **?** Should tools be hireable (a ploughing service, which would be tertiary work), or need a minimum farm size to pay?

- Who gains: track each person's consumption and leisure to see whether gains spread evenly (the old "stage 6").
- Merchants' own time counted in someone's day.
- Moving costs time or tools, so migration isn't free.

## Review log

- **2026-10-18:** list started. Big four added from your notes; QoL seeded with things noticed while building.
- **2026-10-18:** agreed click-to-expand for the chart, discoveries driven by learning by doing × spare time, a power law for every resource, and old constant improvement kept as an option. Still open: how experience counts, discoveries spreading between towns, and deposit depletion.
- **2026-10-18:** sector hierarchy built and food renamed to wheat. Secondary work overtakes primary in year 84. New follow-ups noted under 1.
- **2026-10-18:** lines moved to a separate people chart. Your long no-technology run exposed two flaws in how tools are replaced and taken up; written up under Model flaws, with three possible fixes to choose from.
- **2026-10-18:** saving ahead and choosing jobs with their tools built. Iron ploughs now hold at about 70–87% instead of draining away. New open question on how the hungriest value tools.
- **2026-10-18:** hungry people now value saved hours most and can invest while short. They invest their way out of hunger by year 8, so trade and migration no longer decide who escapes it. New open question on whether going short should cost anything.
- **2026-10-18:** steady technological progress removed; the economy now settles by year 50. Found that iron-plough ownership tracks who grows their own wheat; noted ploughing-as-a-service and births and deaths under ideas.
- **2026-10-18:** 34 tests added and the old lint warnings fixed. Refactors flagged, each tied to the feature that needs it. Emergent markets written up with three options.
