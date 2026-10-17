# Fields to Factories

500 self-sufficient people each split a 16-hour waking day across the goods in an economy, plus leisure. Two data files define the world:

- [`goods.ron`](goods.ron) holds facts about goods: what each one **provides** (food provides nourishment, wares provide comfort) and how it's made, from raw wood and ore up through charcoal and iron to ploughs.
- [`people.ron`](people.ron) holds what people **want and have**: needs, tastes, and how good their land, forest and ore deposits are. Each person draws their own values from the ranges given.

People work out for themselves what any good is worth to them, including tools, which provide nothing directly but save hours.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ nourishment needed ÷ (food per hour × tool boost)
```

The need doesn't grow, so every gain in farm productivity frees hours for everything else. Better ploughs raise the tool boost, but an iron plough sits at the end of a chain: wood → charcoal, plus ore → iron, then iron + wood → plough. Whether that chain pays off depends on each person's forest and ore.

![Year 50](screenshot.png)

## Run

```sh
cargo run --release                                   # interactive window
cargo run --release -- --headless 300                 # 300 years as CSV on stdout
cargo run --release -- --goods g.ron --people p.ron   # use other definitions
```

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## How people decide

1. **Needs first.** For each need, a person works on the cheapest good that provides it until the need is met, or the day runs out.
2. **Spare hours follow their own tastes,** a quarter-hour at a time: more of what they enjoy, leisure, or tools.
3. **Tools are valued by the hours they save.** A plough is worth the farming hours it saves each year, priced at what a spare hour was worth to that person last year, over the plough's discounted lifetime. Someone with no spare hours values a plough at nothing, so the poorest can't invest.
4. **The best tool is used first.** A farmer with half an iron plough and half a wooden one works with iron where it covers and wood for the rest, so an iron plough can make a wooden one worthless.
5. **Inputs carry their labour.** Making an iron plough also counts the hours spent mining, burning charcoal and smelting, each scaled by that person's own resources.

## The definitions

Edit either file and press **Reset** in the app to reload both without recompiling. Reset also redraws everyone.

**`goods.ron`**: one entry per good.

| Field | Meaning |
|---|---|
| `provides` | What one unit gives a person, e.g. `[("nourishment", 1.0)]`. Tools and inputs provide nothing |
| `recipe.rate` | Units made per hour of work |
| `recipe.uses` | A resource, e.g. `"ore"`, whose per-person quality scales the rate |
| `recipe.improves` | Rate grows every year while "improving goods get better" is on |
| `recipe.inputs` | Goods used up per unit, e.g. `[("iron", 2.0), ("wood", 2.0)]` |
| `recipe.tools` | Durable goods that multiply this recipe's output; together they cover at most one person's work |
| `depreciation` | Share of a durable good that wears out each year |

**`people.ron`**: values are fixed (`1.0`) or a range (`(0.8, 1.2)`) that each person draws from.

| Field | Meaning |
|---|---|
| `endowments` | Quality of each resource (`land`, `forest`, `ore`) a person has access to; must be above zero |
| `needs` | Amount of a characteristic needed per day; met before anything else |
| `preferences` | Enjoyment beyond the need: `Log(weight)` is always wanted at a falling rate; `Saturating(weight, scale)` is wanted until about `scale` extra |
| `leisure` | Weight on free time |
| `discount_rate` | How much less next year matters; with wear, sets how far ahead tools are valued |

The loaders reject unknown goods, duplicate names, tools without depreciation or with a multiplier below 1, recipes that are inputs to themselves, wants no good provides, and resources with no endowment.

## The panel

- **Production tree:** raw materials on the left, what people consume on the right. Solid arrows are inputs, dashed arrows are tools, and each node shows its share of all waking hours.
- **Chart:** share of waking hours by sector, stacked to 100%, with a line per tool showing the share of people who own one.
- **Production rates and tools** (collapsed): a slider for every recipe rate and tool multiplier.

## Things to try

- **Watch years 2–10.** Cheap wooden ploughs spread first, then iron ploughs overtake them. Mining, charcoal-burning and smelting appear from nothing to supply them.
- **Look at who never gets iron.** At year 20, 97% of people with rich ore own an iron plough, but only half of those with poor ore do. Geography decides who gets the better technology.
- **Watch years 50–200.** As food gets cheap, the expensive iron plough stops paying for its upkeep and people downgrade to wooden ones, then give up ploughs altogether.
- **Cause a shortage.** Open "Production rates and tools" and drag "ore per hour" down. Iron ploughs disappear, wooden ones come back, and everyone farms longer. A raw material ripples through to the whole economy's day.
- **Add a second source of nourishment,** for example fish with `provides: [("nourishment", 1.0)]` and `recipe: (rate: 0.1)`, then press Reset. Nothing in the code knows fish and food are alternatives; they both provide nourishment.
- **Export CSV** writes `history.csv` with each good's rate, each sector's share of hours, and tool ownership.

## Layout

| File | Contents |
|---|---|
| `goods.ron` | Every good: what it provides and how it's made |
| `people.ron` | What people need, enjoy and have access to, as ranges |
| `src/goods.rs` | Loads and validates the catalog; computes rates, tool boosts and embodied hours |
| `src/people.rs` | Loads and validates the population definition; tastes and their curves |
| `src/sim.rs` | People as ECS components (`Endowments`, `Needs`, `Tastes`, `Stock`, …), and the `SimTick` schedule: advance technology → plan each day → record history |
| `src/ui.rs` | People grid (each person a bar of their day), control panel, production tree and stacked chart |
| `src/main.rs` | Windowed or headless entry point |

See [CHANGELOG.md](CHANGELOG.md) for what each stage added.

## Next: stage 4

Specialisation and trade. Everyone is still self-sufficient, so someone with poor ore has to mine it badly themselves. Letting people trade means those with rich ore could become miners and sell iron, and those with good land could farm for others. That's where markets, prices and division of labour come from.
