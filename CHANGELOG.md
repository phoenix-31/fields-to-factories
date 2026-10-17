# Changelog

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
