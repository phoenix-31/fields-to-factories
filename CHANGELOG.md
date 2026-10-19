# Changelog

## Care for the old

### Added
- **Grown children help their old parents.** Each year, a person over 55 is helped by their eldest living grown child (one under 55 if possible). The child takes on the part of the parent's needs the parent couldn't manage the year before, up to half (`old_support` in `people.ron`, and a Settings slider). Help is need-based: a parent who copes alone costs their child nothing.
- Hover details say whether an old person is being helped and with how much, and whether someone helps an old parent. The Population chart has an "over 55" line; the CSV adds `old`, `old_cared_for` and `old_short`.
- **3 more tests** (45 in all): the old are helped only by their own grown children and never beyond `old_support`, children eat exactly as their household does, and the old look fewer years ahead.

### Fixed
- **Being cold counted as starving.** A household's needs met was its *worst* need, so someone fully fed but without clothes scored 0% and got the full extra chance of dying. It's now the average across needs, so they score 50%.
- **Children were shown as going short.** Their own needs-met was never set; it now matches their household's.
- **The old valued tools as if they'd live for ever.** A tool's value now counts only the years a person can expect to live (one over their yearly chance of dying), so a 70-year-old no longer pours their last hours into a 25-year plough.

### Changed
- **`invest_floor`** in `people.ron` sets how much of their needs a hungry person grows before investing in tools. The default is 0, the existing rule: tested at 0.5 and 0.75, a floor slowed recovery from the hungry start and lowered the population, because a plough repays a hungry year many times over.

### Results
- **The population settles higher,** at about 1,200–1,380 from year 200 (it was 1,050–1,100), mostly because being cold is no longer as deadly as starving. About 11% of households go short at the ceiling.
- **Care helps a little.** Only about 60 people are over 55, and most can feed themselves on half days. With care, 12% of the old go short against 13% without; the cost to their children trims the population by about 2%.

## Births and deaths

### Added
- **People are born, grow up and die.** A run starts with about 300 adults aged 15–35 (`count: 300`), and the population grows into the land.
  - **Births:** each adult aged 16–40 has a yearly chance of a child, up to 30%, scaled by how well they eat and how much spare time they have (2 hours of leisure halves it).
  - **Children** are born beside their parent, with tastes halfway between the parent's and a fresh draw. They don't work; their needs, a quarter of an adult's at birth and rising to all of them at 12, are added to the parent's, so each child lengthens the parent's working day.
  - **Old age:** from 55, people work half days.
  - **Deaths:** 2% a year for a child, 1% for an adult at 30, rising 7% for each year older, plus up to 20% more for a household meeting none of its needs.
  - **Inheritance:** a dying person's eldest child inherits their tools and savings, and takes on any children they were feeding; if the heir is a child, the nearest adult does.
- **`life`** in `people.ron` holds all of these settings, and a **Births and deaths** group in Settings changes the main chances live.
- **Absolute land capacity:** `capacity` in `map.ron` (default 3 people per cell, more on better land) sets where crowding starts. Before, crowding was measured against the current average, which couldn't limit growth.
- **Panel:** a births and deaths card, a **Population** chart (people, children, births and deaths), ages and children in the hover details, and children drawn as small dots beside their parent.
- **CSV:** `population`, `children`, `births` and `deaths` columns. Time shares, hunger and tool ownership now cover adults only.
- **8 more tests** (42 in all): death and birth chances, children's needs, inherited tastes and tools, the population ledger (each year's population is last year's plus births minus deaths), children being fed and growing up, and a well-fed population growing.

### Results
- **A Malthusian ceiling.** From 300, numbers dip in the hungry first years, reach about 630 by year 100 and level off at 1,050–1,100 from year 300. Doubling the land's capacity roughly doubles the ceiling; with crowding off, growth doesn't stop.
- **The gains go into people, not leisure.** With 300 people who never change, farming settles at 34% of the day, leisure at 23% and no one goes hungry. With births and deaths, by year 300 farming takes 50% of adults' day, leisure 14%, and 7–8% of households go short: hunger settles at the margin where deaths balance births.
- About 40% of people are children, typical of a pre-industrial population.
- Rivermouth grows to well over half the population; smaller towns mean thinner markets, with 7–9% of trade unmatched.
- 600 years take about 13 seconds.

## Tests

### Added
- **34 tests** in `src/tests/`, one file per module, run with `cargo test --release` in about a second:
  - **Loaders:** the default files load, and each kind of bad definition is rejected with the right message.
  - **Goods:** build order, the sector tree, group colours, and how tools combine.
  - **Map:** neighbourhoods, crowding past capacity, nearest towns, settling on the map.
  - **Markets:** prices respond to excess demand, merchants ship only when the gap pays for the trip, make-or-buy shares, job choice, and route planning.
  - **Whole runs:** every waking hour is accounted for, runs are repeatable, prices stay positive, no one moves when migration is off, hunger falls as ploughs spread, and tool savings never exceed one tool.

### Changed
- `sim::headless_app` builds the headless app for both `--headless` and the tests.
- The planner's long query became a named `Planner` query, and the three old clippy warnings are fixed.
- **`sim.rs` is split** into `sim/` modules: `mod.rs` (plugin, schedule, reset), `person.rs`, `plan.rs`, `place.rs`, `trade.rs` and `history.rs`.
- **One place makes a person:** `new_person`, ready for births.
- Output is unchanged, byte for byte.

## No more steady technological progress

### Removed
- **Recipes no longer improve by a fixed share every year.** `improves` is gone from `goods.ron`, along with the Technology settings. Over long runs it compounded until rates hit their cap and the model blew up. Technology will come back as **discoveries** (see TODO.md).

### Results
- **The economy now settles.** Farming falls from 81% of the day to 43% by year 20 as ploughs spread and people specialise, then holds at about 38% for 1,000 years. Leisure settles at 22%, carpentry at 16%, and iron-plough ownership creeps up from 84% to 89%.
- **Secondary work never overtakes primary.** Primary work settles at 47% of hours and secondary at 26%; the structural transformation needs ongoing productivity growth.
- **Hunger clears more slowly** (6% at year 10, 1% at year 20, against 1% and 0%), since farming no longer gets easier each year.
- **Wheat stays dear:** about 4.6 hours of work in Rivermouth at year 100, against 1.0 with technology.
- **Iron ploughs and farming:** almost exactly everyone who grows wheat owns an iron plough. At year 1,000, 1,332 of the 1,333 people farming 2 hours a day or more own one, and none of the 166 who buy all their wheat do. Only 582 of them farm for a living; the other 750 or so grow their own food (about 6.4 hours a day) alongside another job.

## The hungry invest

### Changed
- **Hungry people value saved hours most.** Before, someone who couldn't meet their needs valued a spare hour at nothing (they had none), so they never wanted a tool. Now their hour is worth at least what their first hour of rest would be, divided by the share of their needs they meet: the hungrier, the more an hour is worth.
- **They can invest while short.** Someone who can't meet their needs even working all day puts a quarter-hour into a tool instead of food whenever the tool would save more hours over its life than it costs. Everyone else still meets their needs first.

### Results
- **Hunger clears by year 8 instead of year 20.** Needs unmet falls from 48% in year 1 to 15% by year 5 and 1% by year 10, against 28% and 14% before. In year 2, 22% of all hours go into making ploughs, an investment surge led by the poorest.
- **Trade and migration no longer decide who escapes hunger.** With trade priced out (300%), or migration off, hunger clears just as fast. Before, a third of people stayed hungry for 1,000 years without migration and technology; now 1% do.
- **More ploughs.** Iron-plough ownership is about 85–87% from year 20 on, against 65–76%.
- The structural transformation is a little later: secondary work overtakes primary in year 87, against 84.

## Tools that last: saving ahead and choosing jobs with their tools

### Fixed
- **Tools could stay unfinished forever.** Saving towards a tool was tracked in fractions and only counted as done at exactly 100%. People who reached 99.99% got almost nothing from the last sliver, so they stopped and never received the tool. A tool now counts as paid for at 99%.
- **Ploughs slowly disappeared in long runs.** Two flaws, found from a 7,600-year run without technology:
  - **No saving ahead.** People only started paying for a replacement in the old tool's last year, and an iron plough costs more than most can spare in a year, so 94% went without one for a while. Owners now pay into a **sinking fund**: each tool remembers its lifetime, and by each year the owner aims to have saved a matching share of the next one.
  - **Nobody new took up the plough.** Jobs were chosen with the tools people already owned, and tools were valued by last year's hours, so a non-farmer never valued a plough and a farmer without one never looked worthwhile. Job reviews now score each job **with its best tool**, minus the tool's price spread over its lifetime. Tools are valued by the hours the person's job will need, if that's more than last year's.

### Added
- Hover details show how much of the next tool each owner has saved.

### Results
- **Iron ploughs stay.** In the default run, ownership is 76% at year 100 and 69% at year 200, against 35% and 16% before. Wooden ploughs give way faster (1% at year 50, against 9%).
- **No more drift without technology.** Over 3,000 years without improvement, iron-plough ownership rises from 59% to 87% and wheat farming holds at 38% of the day; before, ownership fell from 49% to 32% while farming rose from 45% to 53%.
- **What each fix did** (1,000 years without technology, iron-plough ownership): before, 38%; rounding fix alone, 58%; with the sinking fund and job-based tool value, 75%; with jobs chosen with their tools as well, 78%. The job change matters most when people can't move: with migration off, it cuts the share who can't meet their needs from 46% to 37%.
- Leisure is slightly higher and farming slightly lower throughout; the big picture (farming falling from 80% to 7% of the day over 200 years) is unchanged.

## Separate people chart

### Changed
- **The hours chart shows only hours.** The needs-unmet, tool-ownership and specialisation lines moved to a new **How people are doing** chart below it, since they measure shares of people rather than shares of the day. Hovering over a line reads off its value.
- The year axis labels stretch to runs of thousands of years.

## Sector hierarchy

### Changed
- **Sectors form a tree.** Each good's `sector` in `goods.ron` is now a path, broadest first: `["primary", "agriculture", "wheat farming"]`. The default tree has **primary** (agriculture, forestry, mining) and **secondary** (textiles, carpentry, metals, tools).
- **The chart starts at the top level.** It shows primary, secondary, trading & carrying, and leisure. Clicking a band or its legend entry splits it into its parts; clicking a part groups it again. A group's colour is the average of its goods.
- **The map follows the chart.** In both views, people's marks use the sector level the chart shows, and the map's colour key can split and group sectors too.
- **Food is now wheat,** so the good is separate from the need it meets (nourishment), and other foods can be added later. CSV columns are renamed to match (`wheat_rate`, `wheat farming_share`, `<town>: wheat_price`).
- The overview's first card and the Towns tab's price column follow the goods that meet the first need, instead of looking for a good called "food".

### Added
- **Hovering over the chart** shows the band's name and share of hours in that year, or a line's value.
- **CSV:** a share column for each sector group, named by its path, e.g. `primary/agriculture_share`.
- The loader rejects a sector path used both by a good and as a group, and a good without a sector.

### Fixed
- The map key's "◆ market town" showed a box instead of a diamond.

### Results
- **Structural transformation** is visible at the top level: primary work falls from 85% of hours in year 1 to 30% by year 100 and 20% by year 200, while secondary work rises from 7% to 32% and then 37%, overtaking primary in year 84.

## Migration

### Added
- **People move towards better real wages.** Each year about 5% of people look at 6 random places and move to the best if meeting their needs there takes at least 10% fewer hours. Hours are worked out the way they'd really get each good there: making or buying it, with that place's resources, its town's prices and the walk to market. Settings and `migration` in `people.ron` control it; `chance: 0` turns it off.
- **Crowding.** Where more people live round a spot (it and its eight neighbours) than the settlement pattern expects, each gets proportionally less of every resource. `crowding` in `map.ron` (default 1) and a Settings slider set how strongly. Without it, about 1,350 of 1,500 people moved to Rivermouth.
- **Panel:** a "moved home this year" card, migration and crowding sliders, and hover details showing how often someone has moved and how much crowding cuts their resources. Marks move on the map when people do.
- **CSV:** `moved` (share of people who moved that year) and each town's population.

### Changed
- **Default speed is 5 years a second,** and the slider goes up to 30.

### Results
- **The valley fills.** Rivermouth grows from about 500 to 890 people by year 100 and 950 by year 200; Ironhill falls from 300 to about 110, Crossways to about 220 and Westmoor to about 270. About 3% of people move a year at first, falling below 0.5%.
- **Hardship clears faster.** At year 20, 1% can't meet their needs, against 9% with migration off.
- **Crowding raises early hardship.** In year 1, 48% can't meet their needs, against 36% without crowding, because random settlement packs some spots tighter than the land supports.
- Farming, leisure and specialisation are almost unchanged: 21%, 29% and 57% at year 100, with or without migration.

## Free settlement and a bigger population

### Changed
- **People aren't tied to grid cells.** Each person gets a random position anywhere on the map, chosen once at the start (or on Reset). Several people can share a cell, and some cells are empty.
- **Settlement follows the land.** A new `settlement` section in `map.ron` makes people more likely to live where farmland is good and near towns. The default (`strength: 0.5`, `near_towns: 1.5`) puts about 470 of 1,500 people round valley Rivermouth and 310–370 round each hill town.
- **The population is configurable,** via `count` in `people.ron` (a number or a range), and the default rose from 500 to 1,500. `--people-count` overrides it.
- **The map draws each person as a small mark** at their position. Hovering finds the nearest person and rings them.

### Results
- **Unmatched trade fell** from 8–9% to about 5% a year: deeper town markets are less lumpy.
- A 300-year headless run takes about 12 seconds with 1,500 people (about 40 ms a year on four cores).
- Strong settlement preferences matter: at `strength: 2` the western hills almost emptied and their textile trade disappeared.

## UI clean-up

### Changed
- **The window is reorganised.** Run controls moved to a top bar. The single long side panel became four tabs: **Overview**, **Production**, **Towns** and **Settings**, each with a short description of what it shows.
- **Overview** leads with headline numbers as cards, then the chart, with the core farming equation tucked into "Why farming shrinks".
- **Settings** groups controls into Technology, Trade, Recipes (one collapsible section per good) and Definition files.
- **Tooltips** on almost every number and control explain what it measures or does.
- **The map key** moved onto the map: view switch, a collapsible colour key, and a hint.
- `src/ui.rs` became the `src/ui/` module: `mod.rs` (map drawing), `panel.rs` (layout) and `charts.rs` (tree and chart).

### Added
- **Hover details:** point at any person to see their resources, town and distance, job and wage, how they spent the day, and which tools they own.
- **How it works** window explaining the model in plain language.
- **+1 year** button to step while paused.

## Stage 5: distance

### Added
- **A map.** People live at fixed spots on a 25 × 20 grid. A new `map.ron` generates smooth terrain: land is best in valleys, pasture and ore on hills, forest on the slopes between. Each person's resources depend on where they live. `--show-map` prints it as text.
- **Market towns.** Four towns, each with its own prices. Everyone trades at their nearest town.
- **Carrying costs.** Each good has a `bulk`; buyers carry goods home and sellers carry them to market, at `travel_cost` × bulk × distance hours.
- **Merchants** ship goods between towns when the price gap exceeds the cost of carrying them.
- **Panel.** A carrying-cost slider, a town table (people, wage, main work, food price), a choice of which town's prices the tree shows, and a "main work" map view. The map shows terrain round each bar and the towns as labelled diamonds.
- **CSV.** Each good's price in every town, each sector's share of hours in every town, and the share of trade carried between towns.

### Changed
- **Flax became wool,** grown on upland `pasture`, a new resource, so the hills have something to sell.
- **Resource ranges are wider:** land 0.3–1.7, pasture 0.3–1.7, forest 0.4–1.6, ore 0.1–1.6, so geography matters.
- People are no longer sorted by endowment on the grid; their position is their place on the map.

### Results
- **Regions specialise.** By year 100 the valley farms, the western hills round Westmoor turn to wool, cloth and tailoring, and carpenters cluster on forested slopes.
- **Prices differ by place.** At year 100, food costs about 1.3 hours of work in the valley town against about 1.9 in hill towns.
- **Trade feeds the hills.** 42% of people start unable to meet their needs, mostly on poor hill farmland. By year 20, 4% still can't with trade, against 13% without.
- **Unmatched trade rose** to about 8–9% a year, from 4–5% with one market: four smaller markets are lumpier. Little trade (1–3%) moves between towns at the default carrying cost.

## Stage 4.1: real goods for freed time

### Changed
- **Wares are gone,** replaced by real goods for different wants:
  - **Clothing** comes from a textile chain (flax on land → cloth → clothing) and provides **warmth**. Warmth is a new need, met right after nourishment, and also a taste beyond it.
  - **Furniture** is made from wood and provides **comfort**, the open-ended want.
- **Each recipe has its own improvement rate.** `improves: true` became `improves: 0.015` (1.5% a year). Weaving improves fastest (2%), farming 1.5%, flax and iron 1%, and tailoring, carpentry, forestry and mining 0.5%. The panel's growth control became a "technology improves, at N×" multiplier.
- **Buying is gradual.** The cheaper buying is compared with making, the larger the share of a good (or input) a person buys. Before, everyone flipped at once, which made flax swing between all-bought and all-sold from year to year.
- "Hungry" became "needs unmet", since there are now two needs.

### Fixed
- **Prices were scaled by an average wage that counted the self-sufficient as earning 0,** which inflated every price about fivefold while the market was forming.
- **The chart's year axis** showed no labels between years 50 and 100.

### Results
- Unmatched trade fell from about 8% to 4–5% a year once the market has formed.
- Baumol's cost disease appears: weaving shrinks from 3–4% to under 1% of hours as it mechanises, while slow-improving carpentry grows to about a third of the day by year 200, and forestry grows with it.
- Iron-plough ownership shows an investment echo: it peaks around year 20, then dips as that cohort wears out.

## Stage 4: trade

### Added
- **A marketplace with prices,** measured in hours of an average person's work. Each year a good's price rises if more was bought than sold, and falls if the opposite.
- **Make or buy.** For every good and every input, people compare making it with earning the money at their job and buying it. A smith can buy iron rather than mine ore.
- **Jobs and wages.** A job is the good someone makes to sell; the wage is what an hour of it earns after bought inputs. People start self-sufficient and join the market gradually: each year about one in five reviews their job, choosing better-paying jobs more often but not always.
- **Trade cost,** paid as time at market in proportion to the price, so money balances. Slider in the panel, `--trade-cost` on the command line.
- **Tool lifetimes.** Tools are whole items that last a random number of years (`lifetime` in `goods.ron`) and then break. Hours or money put towards a tool build up as progress until there's a whole one.
- **Panel and CSV:** prices in the production tree, a trading band in the chart and in every bar, a specialisation line, and unmatched trade. The CSV adds prices, trading time, specialisation, share of work sold and mismatch.

### Changed
- `depreciation` in `goods.ron` became `lifetime: (min, max)` in years.

### Results
- With the default 30% trade cost, specialisation rises from about 55% to about 90%, and farming falls to 20% of the day by year 100, against 36% without trade.
- Leisure is about 2–5 points higher with trade than without, depending on the trade cost.
- Once the market has formed, about 8% of trade by value is unmatched in a given year. Faster price changes made this far worse, because prices overshot.

## Stage 3: iron and charcoal

### Added
- **A deeper production tree.** Wood (forests) and ore (mines) are raw materials; charcoal is burned from wood; iron is smelted from ore and charcoal; an iron plough needs iron and wood.
- **Two ploughs.** A cheap wooden plough (×1.5 farm output, wears out at 15% a year) and an iron plough (×2.5, 4% a year). A recipe can list several tools; the best one owned covers the work first, and together they cover at most one person's work.
- **Uneven geography.** People draw the quality of their land, forest and ore from ranges in `people.ron`. A recipe's `uses` names the resource that scales it.
- **Production tree diagram** in the panel, showing each good's share of hours, inputs as solid arrows and tools as dashed ones.
- **Validation** for missing or non-positive endowments and tool multipliers below 1.

### Changed
- `recipe.uses_land` became `recipe.uses: "<resource>"`, and `recipe.tool` became `recipe.tools: [...]`.
- `land_quality` in `people.ron` became the `endowments` list.
- **Panel.** Rate and tool sliders moved into a collapsible section, each tool has its own coloured ownership line, and the grid ranks people by total endowment.

### Results
- Wooden ploughs spread in year 2; iron ploughs overtake them by year 8.
- At year 20, 97% of people with rich ore own an iron plough, against 50% with poor ore.
- When farming gets cheap, people downgrade from iron to wooden ploughs before giving them up.
- Making ore four times harder to mine cuts iron-plough ownership at year 20 from 83% to 23%.

## Stage 2.1: people own their wants

### Changed
- **Goods describe facts; people hold wants.** A good now lists what it `provides` (food → nourishment, wares → comfort) instead of saying why it's wanted. Needs and tastes live in a new `people.ron`, as `Needs` and `Tastes` components on each person.
- **Tools are valued only by the person's own calculation** of hours saved, so the `Tool` want is gone. A good that provides nothing is still worth making if it's a tool or an input.
- **Tastes differ per person,** drawn from ranges in `people.ron`. Reset reloads both files and redraws the population.
- **Need urgency is gone.** Needs are met first by construction, so it never affected anything.

### Added
- **Substitution.** Several goods can provide the same characteristic, and people meet a need with whichever is cheapest for them.
- **`--people <file>`** chooses a different population definition.

## Stage 2: the plough

### Added
- **Goods catalog in `goods.ron`.** Food, wares and the plough are all plain entries, and the simulation, UI and CSV loop over the catalog instead of naming farming. Reset reloads the file, so new goods appear without recompiling.
- **Plough.** A durable tool made from labour and 2 wares that doubles farm output. It wears out at 5% a year.
- **Recipes with inputs.** Making a good also counts the hours embodied in its inputs, so inputs can nest into a production tree.
- **Investment decisions.** People value a tool by the hours it saves each year, priced at their own value of a spare hour, over its discounted lifetime.
- **Catalog validation.** The loader reports unknown goods, duplicate names, tools without depreciation, and cycles in recipes.
- **UI.** One slider per good and per tool boost, a "tool owners" line on the chart, and a bar segment per sector.
- **`--goods <file>`** chooses a different catalog.

### Changed
- **Survival comes first.** People meet essential needs before anything else, so those with no spare hours can't invest.
- **CSV columns** are now generated from the catalog: `<good>_rate`, `<sector>_share`, `leisure_share`, `hungry_share` and `<tool>_coverage`.

### Fixed
- **Stage 1 over-reported early hunger** (about 15%, really about 2%). People stopped farming a sliver short of their need because the last bit of food was worth less than a quarter-hour of leisure. Survival-first planning removes the artifact.

## Stage 1: time allocation

- 500 self-sufficient people splitting a 16-hour day between farming, crafting and leisure.
- Satiable food demand, so hours farming ≈ appetite ÷ food per hour.
- Interactive Bevy window with sliders, a per-person day grid and a stacked history chart; headless CSV mode.
