# Fields to Factories

About 300 people (you choose how many) start out scattered over a 25 × 20 map, settling more densely on good farmland and near towns. They have children, grow old and die, and the population grows until the land is crowded. Each adult splits a 16-hour waking day across the goods in an economy, trading at the nearest market town, plus leisure, and feeds their children. Three data files define the world:

- [`goods.ron`](goods.ron) holds facts about goods: what each one **provides** (wheat provides nourishment, clothing provides warmth, furniture provides comfort), how it's made, and how bulky it is to carry. Raw wool, wood and ore feed chains up to clothing, furniture and ploughs.
- [`people.ron`](people.ron) holds **who** lives here and what they **want**: how many people, their needs and tastes (drawn per person from ranges), and the range of quality for each resource.
- [`map.ron`](map.ron) holds the **geography**: terrain that decides where land, pasture, forest and ore are good, where people first settle, how crowding thins out resources, the market towns, and the cost of carrying goods.

People work out for themselves what any good is worth to them, including tools, which provide nothing directly but save hours. Every good can be made or bought, whichever costs that person fewer hours, so people with good land end up farming for others and people near rich ore end up mining.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ nourishment needed ÷ (wheat per hour × tool boost)
```

The need doesn't grow, so every gain in farm productivity frees hours for everything else. Recipes don't improve on their own: the gains come from tools, from specialising, and from trade. (Steady yearly improvement was removed because it made long runs blow up; discoveries are planned instead, see [TODO.md](TODO.md).) Better ploughs raise the tool boost, but an iron plough sits at the end of a chain: wood → charcoal, plus ore → iron, then iron + wood → plough. Trade lets each person take part in that chain at whichever step suits their land, forest and ore, as long as the trip to market is worth it.

![The window at year 181 with 932 people, 364 of them children, hovering over a young farmer near Rivermouth](screenshot.png)

## Run

```sh
cargo run --release                                   # interactive window
cargo run --release -- --headless 300                 # 300 years as CSV on stdout
cargo run --release -- --goods g.ron --people p.ron   # use other definitions (also --map)
cargo run --release -- --show-map                     # print the terrain and towns as text
cargo run --release -- --trade-cost 3 --headless 300  # start with a different trade cost
cargo run --release -- --people-count 3000            # override the number of people
```

Run the tests with `cargo test --release` (using the release profile reuses the already-compiled Bevy).

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## How people decide

1. **Needs first.** For each need, a person works on the cheapest good that provides it until the need is met, or the day runs out. The exception is someone who can't meet their needs even working all day: they put hours into a tool instead whenever it would save more hours over its life than it costs.
2. **Spare hours follow their own tastes,** a quarter-hour at a time: more of what they enjoy, leisure, or tools.
3. **Tools are valued by the hours they save.** A plough is worth the farming hours it saves each year, priced at what a spare hour was worth to that person last year, over the plough's discounted lifetime. The farming hours are last year's, or what their job will need if that's more, so someone who has just become a farmer values a plough properly. Someone who can't meet their needs values an hour more than anyone: at least what their first hour of rest would be worth, and more the further short they fall.
4. **Tools are whole and wear out at random.** A new tool lasts a number of years drawn from its `lifetime` range, then breaks and must be replaced. Hours put towards a tool build up as progress (or savings, if buying) until there's a whole one. Owners save for the next one over the old one's life, a share each year (a sinking fund), so it's paid for when the old one breaks. A farmer works with the best tool they own, so an iron plough makes a wooden one pointless.
5. **Make or buy.** For each good, and each input, a person compares making it with earning the money at their job and buying it. Buying also costs time at market: the trade cost, a share of the price. Hours spent making inputs count in those sectors, and hours spent earning count in the job's sectors.
6. **Gradual choices.** The cheaper buying is compared with making, the larger the share of a good a person buys, rather than everyone flipping at once.
7. **Distance.** Everyone trades at their nearest town. Buyers carry goods home and sellers carry goods to market, each costing `travel_cost` × the good's `bulk` × the distance in hours. Far from town, buying and selling cost more, so people make more for themselves.
8. **Jobs.** A person's job is the good they make to sell; their wage is what an hour of it earns after paying for bought inputs. People start out self-sufficient. Each year about one in five reviews their job and picks one, where better-paying jobs are more likely but not certain. A job is judged together with its best tool: farming is scored as farming with a plough, minus the plough's yearly cost, so becoming a plough farmer is one decision rather than two.
9. **Migration.** Each year about one in twenty people look at a handful of other places and move to the best if it clearly beats home. "Best" means a **real wage**: the hours it would take to meet their needs there, making or buying each good as they would, with that place's resources, town prices and carrying distance. Moving takes their position and nothing else; their tools and tastes come with them.
10. **Crowding.** Land is shared. Each cell supports a number of people (`capacity`, more on better land); when more people live round a spot (it and its eight neighbours) than that, everyone there gets less of every resource: at `crowding: 1`, twice as many people leave each with two thirds as much. Crowding is what stops the population growing for ever.
11. **Births, growing up and death.** Adults aged 16–40 may have a child each year, more likely the more spare time they have and the better they eat. A child is born beside its parent, with tastes between the parent's and a fresh draw, and is fed by them until 12: its needs, a quarter of an adult's at birth and rising, are added to the parent's. From 55 people work half days. The chance of dying is 2% a year for a child and 1% for an adult at 30, rising 7% for each year older, plus more for a household going short of food. When someone dies, their eldest child inherits their tools, and any children they were feeding go to that heir, or to the nearest adult.

## Markets and towns

Each town has its own prices, measured in **hours of an average person's work**, so an iron plough "costs 19 hours". Each year, a good's price in a town rises if more was bought there than sold, and falls if the opposite. Buyers pay the price plus time at market and carrying; sellers get the price, minus their own carrying time.

**Merchants** link the towns. When one town has surplus and another a shortage, and the price gap is bigger than the cost of carrying the good between them, merchants ship it. Merchants' own time isn't yet counted in anyone's day.

The markets don't clear exactly. Once they've formed, about 5% of trade by value finds no counterpart in a given year. Smaller populations make it worse: with 500 people it was 8–9%, because each town's market was lumpier. Merchants absorb the gap, and prices move to close it. The panel shows this as "unmatched". In the first 10 years, while people are still joining, most trade is unmatched.

## The definitions

Edit any of them and press **Reset** in the app to reload all three without recompiling. Reset also redraws everyone.

**`goods.ron`**: one entry per good.

| Field | Meaning |
|---|---|
| `sector` | Where the work sits in the industry tree, broadest first, e.g. `["primary", "agriculture", "wheat farming"]`. The last name is the good's own activity. The chart, map and CSV group by it |
| `color` | The activity's colour; a group's colour is the average of its goods |
| `provides` | What one unit gives a person, e.g. `[("nourishment", 1.0)]`. Tools and inputs provide nothing |
| `recipe.rate` | Units made per hour of work |
| `recipe.uses` | A resource, e.g. `"ore"`, whose per-person quality scales the rate |
| `recipe.inputs` | Goods used up per unit, e.g. `[("iron", 2.0), ("wood", 2.0)]` |
| `recipe.tools` | Tools that multiply this recipe's output; a person works with the best one they own |
| `lifetime` | Makes the good a tool: the years it lasts before breaking, drawn from the range, e.g. `(15.0, 35.0)` |
| `bulk` | How hard one unit is to carry, relative to wheat (default 1). Ore is 2, cloth 0.3 |

**`people.ron`**: values are fixed (`1.0`) or a range (`(0.8, 1.2)`) that each person draws from.

| Field | Meaning |
|---|---|
| `count` | How many people a run starts with; a fixed number, or a range drawn once per Reset. Default 300 |
| `life` | Births, deaths and ages: `start_age` of the first people, `adult_at` (stop being fed, start working), `old_at` and `old_work` (share of a day the old work), `child_needs` (a newborn's share of an adult's needs), `fertile` ages, `birth_chance` and `spare_half` (hours of leisure that halve it), `child_death`, `adult_death` (at 30), `ageing` (yearly rise in an adult's chance of dying) and `hunger_death` |
| `migration` | Each year, `chance` of people look at `look_at` random places (drawn from the settlement pattern) and move to the best if it cuts the hours they need to meet their needs by at least `min_gain`. `chance: 0` turns it off |
| `endowments` | Range of quality for each resource (`land`, `pasture`, `forest`, `ore`); where in the range a person falls depends on the terrain where they live. Must be above zero |
| `needs` | Amount of a characteristic needed per day; met before anything else, in the order listed (nourishment, then warmth) |
| `preferences` | Enjoyment beyond the need: `Log(weight)` is always wanted at a falling rate; `Saturating(weight, scale)` is wanted until about `scale` extra |
| `leisure` | Weight on free time |
| `discount_rate` | How much less next year matters; with wear, sets how far ahead tools are valued |

**`map.ron`**: run with `--show-map` to see the terrain and where towns sit.

| Field | Meaning |
|---|---|
| `seed`, `feature_size` | Which terrain to generate, and roughly how many cells wide its hills and valleys are |
| `terrain` | Where each resource is best: `Low` (valleys), `Middle` (slopes), `High` (hills) or `Anywhere` |
| `towns` | Market towns, by name and `(column, row)` from the top-left |
| `travel_cost` | Hours to carry one unit of bulk one cell |
| `settlement` | Where people choose to live: denser where the `follows` resource is good (`strength` 0 ignores it; higher crowds the best spots) and up to `1 + near_towns` times denser right beside a town |
| `crowding` | How fast resources thin out where more people live than the land supports; 0 never, 1 (default) leaves two thirds each at twice the supported density |
| `capacity` | People per cell the land supports, on average, before crowding starts; better land supports more. Default 3 |

The loaders reject unknown goods, duplicate names, tools without a lifetime (or one under a year) or with a multiplier below 1, recipes that are inputs to themselves, wants no good provides, resources with no endowment, towns off the map, terrain or settlement for resources no recipe uses, and a count below 1.

## The window

- **Top bar:** run or pause, step one year, simulation speed (5 years a second by default, up to 30), reset (reloading the definition files), export CSV, and **How it works**, a short guide to the model.
- **Map:** each small mark is one person, placed where they live; several can share a cell. The key in its corner switches between **Each day** (a bar of their 16 waking hours, work at the bottom, leisure on top) and **Main work** (coloured by their biggest sector, like a land-use map), and opens a colour key. The marks use whatever sector level the chart shows. The ground shows terrain: yellow-green valleys, dark green forest, grey hills; diamonds are market towns. **Hover over anyone** to ring them and see their age, resources, town, how many children they feed, job, wage, how they spent the day, which tools they own, how often they've moved, and how much crowding cuts their resources. Children are small pale dots beside their parent.
- **Side panel tabs.** Hover over almost anything for an explanation.
  - **Overview:** headline numbers (wheat farming, leisure, needs unmet, specialisation, work sold, time trading and carrying, people who moved home this year, births and deaths), the "Where the day goes" chart of adults' hours by sector, the "How people are doing" chart (needs unmet, tool ownership, specialisation), the "Population" chart (people, children, births and deaths), and "Why farming shrinks", the core equation with today's numbers.
  - **Production:** the production tree with prices in a chosen town, and how many people own each tool.
  - **Towns:** people, average wage, main work and wheat price for each town.
  - **Settings:** time at market, carrying cost, a slider for every recipe and tool, migration (how many look, how many places, the gain needed, crowding), births and deaths (birth and death chances, and how many people the land supports), and the definition files with a reset button.

## Things to try

- **Switch the map to "Main work".** By year 100 the fertile valley round Rivermouth farms, the sheep hills round Westmoor have become a textile district (wool, cloth, tailoring), and carpenters cluster on the forested slopes near their wood. Nothing places them; it's where their resources are.
- **Split the chart open.** It starts with four bands: primary (farming, herding, forestry, mining), secondary (making things from raw materials), trading & carrying, and leisure. Click a band to split it, and click a part to group it again; hovering reads off the share in any year. At the top level, primary work falls from 86% of hours in year 1 to 47% by year 50 as ploughs spread and people specialise, then holds. Secondary work rises from 7% to 26%. Without new technology, secondary never overtakes primary.
- **Watch the population fill the land.** From 300, numbers dip in the first hungry years, then grow to about 630 by year 100 and level off at about 1,050–1,100 from year 300, with births and deaths near 45 a year each and about 40% of people children. Doubling `capacity` roughly doubles the ceiling.
- **See the Malthusian trap.** Compare a run with births and deaths against one with `birth_chance`, the death chances and `hunger_death` set to 0 (and `old_at` very high). With 300 people that never change, wheat farming settles at 34% of the day, leisure at 23% and nobody goes hungry. With births and deaths, the gains from ploughs and trade go into more people instead: by year 300 there are about 1,050, farming takes 50% of the day, leisure 14%, and 7–8% of households go short.
- **Compare wheat prices and populations on the Towns tab.** The valley town Rivermouth grows to about 615 of the 1,080 or so people, and wheat costs about 5 hours of work there against 6.7–7.8 in the hill towns: the price of carrying it uphill.
- **Watch the hills go hungry, invest their way out, then fill up again.** Hill farmland is poor, so about 30% of households start out unable to meet their needs. Because a hungry person values saved hours most, they put hours into a plough even while short of food, and hunger falls to about 2% by year 25. Then the population grows, the land crowds, and hunger creeps back to 6–8%, where it stays: the margin at which deaths balance births.
- **Change the starting population.** Set `count` in `people.ron` (or run with `--people-count`) and press Reset. Small populations make for thin markets: unmatched trade is 7–9% a year at about 1,000 people.
- **Change where people settle.** In `map.ron`, raise `settlement.strength` to crowd everyone into the valleys and empty the hills, or set it to 0 to spread people evenly.
- **Watch the drift to the valley.** People move towards cheaper wheat, and the valley's people have the most children. Rivermouth's share of the population grows from about a third to well over half, while Ironhill stays small.
- **Turn crowding down.** At `crowding: 0` (in Settings or `map.ron`) nothing stops the population growing, and the run slows down as numbers climb into the thousands.

- **Raise the carrying cost.** Trade shrinks, prices drift further apart between towns, and the regions become more self-sufficient.
- **Move or add a town in `map.ron`,** then press Reset. Put one deep in the western hills and see whether the textile district moves with it.
- **Watch where freed time goes.** As ploughs spread, wheat farming falls from 77% of adults' day to about 42% by year 25, and the freed time goes to leisure and carpentry. Then the growing population claws it back: farming returns to about 50% as the land crowds and every child needs feeding.
- **Watch years 2–10.** Cheap wooden ploughs spread first, then iron ploughs take over as a mining, charcoal and smelting chain forms. By year 50 about three-quarters of people own an iron plough, and wooden ones have almost gone.
- **Run for hundreds of years.** The population settles by about year 300 and stays between about 1,010 and 1,150, with farming at about 50% of adults' day, leisure at 14% and iron-plough ownership at about 60%.
- **Export CSV** writes `history.csv` with each good's rate and its price in every town, each sector's share of hours overall and in every town, each sector group's share (e.g. `primary/agriculture_share`), trading time, specialisation, trade carried between towns, unmatched trade and tool ownership.

## Layout

| File | Contents |
|---|---|
| `goods.ron` | Every good: what it provides and how it's made |
| `people.ron` | What people need and enjoy, and the range of each resource's quality |
| `map.ron` | Terrain, market towns and carrying cost |
| `src/goods.rs` | Loads and validates the catalog; computes rates, tool boosts and embodied hours |
| `src/people.rs` | Loads and validates the population definition; tastes and their curves |
| `src/map.rs` | Loads the map, generates terrain and finds each spot's nearest town |
| `src/market.rs` | Each town's prices, merchants between towns, and each person's make-or-buy routes and job choice |
| `src/sim/mod.rs` | The plugin and the `SimTick` schedule: tick the clock → grow up → count crowding → add children's needs to their parents' → plan each year → replace tools → clear the market → migrate → births and deaths → record history; resetting from the files |
| `src/sim/person.rs` | People as ECS components (`Home`, `Endowments`, `Needs`, `Tastes`, `Tools`, `Job`, `Trades`, …), `new_person`, and tools wearing out |
| `src/sim/plan.rs` | Each person's year: job, routes, needs first, then spare quarter-hours, tools and saving ahead |
| `src/sim/place.rs` | Crowding and migration |
| `src/sim/trade.rs` | Opening and clearing each town's market |
| `src/sim/life.rs` | Growing up, feeding children, births, deaths, inheritance and orphans |
| `src/sim/history.rs` | Yearly samples and the CSV |
| `src/ui/mod.rs` | The map: terrain, towns, and each person's day or main work |
| `src/ui/panel.rs` | Top bar, tabbed side panel, map key, hover details and the How it works window |
| `src/ui/charts.rs` | The production tree, and the history chart with its expandable sector bands |
| `src/main.rs` | Windowed or headless entry point |
| `src/tests/` | Tests for each module, from loader errors to whole headless runs; `cargo test --release` runs them in about a second |

See [CHANGELOG.md](CHANGELOG.md) for what each stage added.

## Next: stage 6

Who gains? Wages already differ a lot between people and places. Tracking each person's consumption and leisure would show whether trade and tools spread gains evenly, or whether the best-endowed places pull ahead, and what happens to the remote and the poor.
