# Changelog

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
