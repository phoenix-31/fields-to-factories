# Fields to Factories

500 self-sufficient people each split a 16-hour waking day across the goods in an economy, plus leisure. Two data files define the world:

- [`goods.ron`](goods.ron) holds facts about goods: what each one **provides** (food provides nourishment, wares provide comfort) and how it's made.
- [`people.ron`](people.ron) holds what people **want**: needs (so much nourishment a day) and tastes (how much they enjoy more of something, and free time). Each person draws their own values from the ranges given.

People work out for themselves what any good is worth to them, including tools, which provide nothing directly but save hours.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ nourishment needed ÷ (food per hour × plough boost)
```

The need doesn't grow, so every gain in farm productivity frees hours for everything else. A plough is the first good that improves another good's recipe: hours spent making ploughs today mean fewer hours farming tomorrow.

![Year 34](screenshot.png)

## Run

```sh
cargo run --release                              # interactive window
cargo run --release -- --headless 300            # 300 years as CSV on stdout
cargo run --release -- --goods g.ron --people p.ron   # use other definitions
```

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## How people decide

1. **Needs first.** For each need, a person works on the cheapest good that provides it until the need is met, or the day runs out.
2. **Spare hours follow their own tastes,** a quarter-hour at a time: more of what they enjoy, leisure, or tools.
3. **Tools are valued by the hours they save.** A plough is worth the farming hours it saves each year, priced at what a spare hour was worth to that person last year, over the plough's discounted lifetime. Someone with no spare hours values a plough at nothing, so the poorest can't invest.
4. **Inputs carry their labour.** A plough needs 2 wares, so making one also counts the crafting hours behind those wares.

## The definitions

Edit either file and press **Reset** in the app to reload both without recompiling. Reset also redraws everyone's tastes.

**`goods.ron`**: one entry per good.

| Field | Meaning |
|---|---|
| `provides` | What one unit gives a person, e.g. `[("nourishment", 1.0)]`. Tools and pure inputs provide nothing |
| `recipe.rate` | Units made per hour of work |
| `recipe.uses_land` | Output scales with the worker's land quality |
| `recipe.improves` | Rate grows every year while "improving goods get better" is on |
| `recipe.inputs` | Goods used up per unit, e.g. `[("wares", 2.0)]` |
| `recipe.tool` | A durable good that multiplies this recipe's output, up to one per person |
| `depreciation` | Share of a durable good that wears out each year |

**`people.ron`**: values are fixed (`1.0`) or a range (`(0.8, 1.2)`) that each person draws from.

| Field | Meaning |
|---|---|
| `needs` | Amount of a characteristic needed per day; met before anything else |
| `preferences` | Enjoyment beyond the need: `Log(weight)` is always wanted at a falling rate; `Saturating(weight, scale)` is wanted until about `scale` extra |
| `leisure` | Weight on free time |
| `land_quality` | Scales output of recipes that use land |
| `discount_rate` | How much less next year matters; with wear, sets how far ahead tools are valued |

The loaders reject unknown goods, duplicate names, tools without depreciation, recipes that are inputs to themselves, and wants that no good provides.

## Things to try

- **Watch years 2–5.** Leisure collapses while people build ploughs: an "industrious revolution". Then the white line (share of people with a plough) completes its S-curve.
- **Watch years 150–200.** Food gets so cheap that a plough no longer saves enough hours to cover its upkeep, and people stop replacing them.
- **Drag "× food with a plough" to 1.** Ploughs become worthless and the economy runs like stage 1.
- **Add a second source of nourishment,** for example fish with `provides: [("nourishment", 1.0)]` and `recipe: (rate: 0.1)`, then press Reset. People fish at first because it's cheaper, then switch back to farming once ploughs and better farming overtake it. Nothing in the code knows fish and food are alternatives; they both provide nourishment.
- **Add a new want,** for example a `warmth` preference in `people.ron` and a good that provides it in `goods.ron`. It gets its own slider, chart band and bar segment.
- **Export CSV** writes `history.csv` with each good's rate, each sector's share of hours, and tool ownership.

## Layout

| File | Contents |
|---|---|
| `goods.ron` | Every good: what it provides and how it's made |
| `people.ron` | What people need and enjoy, as ranges |
| `src/goods.rs` | Loads and validates the catalog; computes rates, tool boosts and embodied hours |
| `src/people.rs` | Loads and validates the population definition; tastes and their curves |
| `src/sim.rs` | People as ECS components (`Needs`, `Tastes`, `Stock`, …), and the `SimTick` schedule: advance technology → plan each day → record history |
| `src/ui.rs` | People grid (each person a bar of their day), control panel and stacked chart |
| `src/main.rs` | Windowed or headless entry point |

See [CHANGELOG.md](CHANGELOG.md) for what each stage added.

## Next: stage 3

Make the tree deeper. Add iron and charcoal so ploughs need mined inputs, letting the production web reach back to the mines, and so a shortage of one raw material can ripple outward.
