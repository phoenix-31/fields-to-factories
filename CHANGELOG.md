# Changelog

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
