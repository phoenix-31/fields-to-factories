# Fields to Factories

500 self-sufficient people each split a 16-hour waking day across the goods in an economy, plus leisure. Every good, including food, is defined as data in [`goods.ron`](goods.ron): what it's wanted for and how it's made.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ appetite ÷ (food per hour × plough boost)
```

Appetite doesn't grow, so every gain in farm productivity frees hours for everything else. A plough is the first good that improves another good's recipe: hours spent making ploughs today mean fewer hours farming tomorrow.

![Stage 2 at year 49](screenshot.png)

## Run

```sh
cargo run --release                              # interactive window
cargo run --release -- --headless 300            # 300 years as CSV on stdout
cargo run --release -- --goods my-goods.ron      # use a different catalog
```

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## How people decide

1. **Survival first.** Each person works on essential goods (food) until their need is met, or the day runs out.
2. **Spare hours go to whatever is worth most,** a quarter-hour at a time: better food, wares, leisure, or tools.
3. **Tools are valued by the hours they save.** A plough is worth the farming hours it saves each year, priced at what a spare hour was worth to that person last year, over the plough's discounted lifetime. Someone with no spare hours values a plough at nothing, so the poorest can't invest.
4. **Inputs carry their labour.** A plough needs 2 wares, so making one also counts the crafting hours behind those wares.

## The catalog (`goods.ron`)

Each entry is one good. Edit the file and press **Reset** in the app to reload it without recompiling.

| Field | Meaning |
|---|---|
| `want` | `Essential(..)` is met before anything else; `Diminishing(..)` is always wanted, at a falling rate; `Tool` is valued only for the hours it saves |
| `recipe.rate` | Units made per hour of work |
| `recipe.uses_land` | Output scales with the worker's land quality |
| `recipe.improves` | Rate grows every year while "improving goods get better" is on |
| `recipe.inputs` | Goods used up per unit, e.g. `[("wares", 2.0)]` |
| `recipe.tool` | A durable good that multiplies this recipe's output, up to one per person |
| `depreciation` | Share of a durable good that wears out each year |

The loader rejects unknown goods, duplicate names, tools without depreciation, and recipes that are inputs to themselves.

## Things to try

- **Watch years 2–5.** Leisure collapses while people build ploughs: an "industrious revolution". Then the white line (share of people with a plough) completes its S-curve.
- **Watch years 150–200.** Food gets so cheap that a plough no longer saves enough hours to cover its upkeep, and people stop replacing them.
- **Drag "× food with a plough" to 1.** Ploughs become worthless and the economy runs like stage 1.
- **Add a good to `goods.ron`,** for example cloth with `want: Diminishing(weight: 0.8)`, then press Reset. It gets its own slider, chart band and bar segment.
- **Export CSV** writes `history.csv` with each good's rate, each sector's share of hours, and tool ownership.

## Layout

| File | Contents |
|---|---|
| `goods.ron` | The catalog: every good, want and recipe |
| `src/goods.rs` | Loads and validates the catalog; computes rates, tool boosts and embodied hours |
| `src/sim.rs` | People as ECS components, and the `SimTick` schedule: advance technology → plan each day → record history |
| `src/ui.rs` | People grid (each person a bar of their day), control panel and stacked chart |
| `src/main.rs` | Windowed or headless entry point |

See [CHANGELOG.md](CHANGELOG.md) for what each stage added.

## Next: stage 3

Make the tree deeper. Add iron and charcoal so ploughs need mined inputs, letting the production web reach back to the mines, and so a shortage of one raw material can ripple outward.
