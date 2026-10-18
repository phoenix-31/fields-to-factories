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

## Model flaws found

### Ploughs slowly disappear (found in your 7,600-year no-technology run)

Iron-plough ownership drifts from about 49% at year 100 to 17% at year 7,600, and wheat farming rises from 43% to 64% of the day. With migration off it's faster: 30% to 5% in 3,000 years. Two causes, both found by logging every plough that reaches the end of its life:

1. **No saving ahead.** People only start paying for a replacement in the old tool's last year. An iron plough costs about 22 hours of average work, more than most can spare in one year, so 94% of owners are left without one for a year or more (they'd saved 60–70% of it). A test where people start saving five years ahead cut these gaps from 94% to about 3%.
2. **Nobody new takes up the plough.** A plough is valued by the hours *you* farmed last year, and a job is chosen with the tools *you* own now. So a non-farmer never values a plough, and a farmer without one never looks like a good farmer. Each owner who drops out isn't replaced; the remaining owners farm longer instead. Even with saving ahead, new buyers fell to almost none and ownership still drifted from 40% to 18% over 2,000 years.

The early high ownership is a legacy of the start, when everyone farmed for themselves and wanted a plough. The model then slowly relaxes towards fewer, overworked plough farmers. It's slow because ploughs last 15–35 years and only a few percent of each generation are lost.

- [ ] **?** Fix 1: save towards a replacement over the tool's life (a sinking fund), or start saving a few years before it's due?
- [ ] **?** Fix 2: when people review their job, compare "job plus the tools it uses" bundles, so becoming a ploughing farmer is one decision, not two. Value tools by the hours the planned job will need, not last year's hours.
- [ ] **?** Credit: let people borrow for a tool and repay from the hours it saves. That's another way out of the trap, and a step towards banks.

## Quality of life

- [x] Chart: hover to read the values at a year.
- [ ] Chart: zoom to the last N years.
- [ ] Keyboard shortcuts: space to pause, → to step a year, R to reset.
- [ ] A seed setting and a "new seed" button. The random seed is fixed, so every run is identical.
- [ ] Compare runs: keep the last run's lines faintly on the chart after a Reset.
- [ ] Map pan and zoom.
- [ ] Mark the settings that only apply after a Reset.
- [ ] Tests: there are none. Start with the loaders and a short headless run, before the big refactors.
- [ ] Fix the three existing clippy warnings.
- [ ] Faster headless runs (about 40 ms a year with 1,500 people).
- [ ] A `--no-tech` flag, so long no-technology runs don't need an edited goods.ron.
- [x] Needs-unmet, tool and specialisation lines moved off the hours chart onto their own chart.

## Ideas, not yet planned

- Who gains: track each person's consumption and leisure to see whether gains spread evenly (the old "stage 6").
- Merchants' own time counted in someone's day.
- Moving costs time or tools, so migration isn't free.

## Review log

- **2026-10-18:** list started. Big four added from your notes; QoL seeded with things noticed while building.
- **2026-10-18:** agreed click-to-expand for the chart, discoveries driven by learning by doing × spare time, a power law for every resource, and old constant improvement kept as an option. Still open: how experience counts, discoveries spreading between towns, and deposit depletion.
- **2026-10-18:** sector hierarchy built and food renamed to wheat. Secondary work overtakes primary in year 84. New follow-ups noted under 1.
- **2026-10-18:** lines moved to a separate people chart. Your long no-technology run exposed two flaws in how tools are replaced and taken up; written up under Model flaws, with three possible fixes to choose from.
