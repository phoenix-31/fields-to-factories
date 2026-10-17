# Fields to Factories, stage 1

500 self-sufficient people each split a 16-hour waking day between **farming**, **crafting** and **leisure**. Food is a satiable need: past a full stomach, more of it is worth very little. Goods and leisure have diminishing returns but never stop being worth something.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ appetite ÷ food per hour
```

Appetite doesn't grow, so every gain in farm productivity frees hours for everything else.

## Run

```sh
cargo run --release                        # interactive window
cargo run --release -- --headless 300      # 300 years as CSV on stdout
```

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## Things to try

- **Watch the first 30 years.** About 15% of people, on the worst land (bottom-right of the grid), can't feed themselves even farming all 16 hours, so their bars turn red. Pause and drag "food per hour" down to watch famine spread.
- **Notice the plateau around years 40–80.** Once basic food is cheap, people put some of the freed time into *better* food. The readout splits farming into "to eat enough" and "growing better food". It's a small Engel's-law story inside the big one.
- **Leave growth at 1.5% a year.** Farming falls from about 79% of waking hours to about 5% by year 300.
- **Raise "goods per hour".** The split between crafting and leisure moves, but the hours needed to eat don't. Only farm productivity frees people from the fields.
- **Export CSV** writes `history.csv` for analysis anywhere.

## Layout

| File | Contents |
|---|---|
| `src/sim.rs` | Components (`LandQuality`, `Appetite`, `TimeUse`, `Nourishment`), the `Economy` resource, and the `SimTick` schedule: advance technology → allocate time → record history |
| `src/ui.rs` | People grid (each person a bar of their day, as Bevy sprites), control panel and stacked chart (egui) |
| `src/main.rs` | Windowed or headless entry point |

The simulation runs in its own `SimTick` schedule, so the window and headless mode share exactly the same code.

## Next: stage 2

Add a **plough**: a crafted good that multiplies farm productivity. Hours spent crafting today then raise food per hour tomorrow, which turns the one-way slider into a feedback loop.
