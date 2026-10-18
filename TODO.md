# To do

A working list, reviewed together. Items move from **Ideas** to **Next** once we've agreed how they should work, and to the [CHANGELOG](CHANGELOG.md) when they're done. Open questions are marked **?**.

## Next: the big four

Suggested order: 1 → 2 → 4 → 3. The hierarchy makes adding goods bearable. Multiple recipes are the slot that discoveries fill. Deposits give discoveries something to find.

### 1. Sector hierarchy

The chart has 13 bands, and it gets worse with every good. Group sectors into a tree and show the top level by default.

- Each good (later, each recipe) names its place in the tree, e.g. `sector: ["primary", "agriculture"]`, and the good itself is the leaf.
- A first tree:
  - **Primary:** agriculture (food, wool), forestry (wood), mining (ore)
  - **Secondary:** textiles (cloth, clothing), woodwork (furniture, wooden plough), metals (charcoal, iron, iron plough)
  - **Tertiary:** trading & carrying (later merchants and other services)
  - **Leisure**
- Chart (agreed): one band per top-level group by default; clicking a band or its legend entry expands it into its children while the others stay collapsed. Colours: one hue per family, shades for the children.
- The same grouping applies to the map's "Main work" view, the Towns tab, hover details and the CSV (shares at each level).
- **?** Rename `food` to `wheat`, so the good is separate from the need (nourishment) and other foods (barley, meat, fish) can be added later?

### 2. Multiple recipes per good

Today each good has exactly one way to make it.

- A good lists `recipes: [...]`, each with its own name, sector, rate, `uses`, inputs and tools. People use whichever is cheapest for them, the same way they already choose between making and buying.
- Examples: iron from a charcoal bloomery or a coal furnace; cloth by hand or on a loom; food from fields or from pasture as meat.
- Code impact: hours and sectors are tracked per recipe rather than per good; affects the catalog, `plan_routes` and the production tree.

### 3. Technology as discovery

At the moment every recipe improves by a fixed percentage every year, so every run is the same smooth glide. Real history is lumpy: new crops, coal, the blast furnace. Discoveries create winners and losers, and change where it pays to live, which migration then acts on.

- **Discoveries** unlock a new resource, recipe or good. The old constant improvement becomes **optional** (agreed): off by default, with a setting to turn it back on for comparison.
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

## Quality of life

- [ ] Chart: hover to read the values at a year; zoom to the last N years.
- [ ] Keyboard shortcuts: space to pause, → to step a year, R to reset.
- [ ] A seed setting and a "new seed" button. The random seed is fixed, so every run is identical.
- [ ] Compare runs: keep the last run's lines faintly on the chart after a Reset.
- [ ] Map pan and zoom.
- [ ] Mark the settings that only apply after a Reset.
- [ ] Tests: there are none. Start with the loaders and a short headless run, before the big refactors.
- [ ] Fix the three existing clippy warnings.
- [ ] Faster headless runs (about 40 ms a year with 1,500 people).

## Ideas, not yet planned

- Who gains: track each person's consumption and leisure to see whether gains spread evenly (the old "stage 6").
- Merchants' own time counted in someone's day.
- Moving costs time or tools, so migration isn't free.

## Review log

- **2026-10-18:** list started. Big four added from your notes; QoL seeded with things noticed while building.
- **2026-10-18:** agreed click-to-expand for the chart, discoveries driven by learning by doing × spare time, a power law for every resource, and old constant improvement kept as an option. Still open: rename food to wheat, how experience counts, discoveries spreading between towns, and deposit depletion.
