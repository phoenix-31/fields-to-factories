# Fields to Factories

500 people each split a 16-hour waking day across the goods in an economy, trading at a shared market, plus leisure. Two data files define the world:

- [`goods.ron`](goods.ron) holds facts about goods: what each one **provides** (food provides nourishment, clothing provides warmth, furniture provides comfort), how it's made, and how fast its recipe improves. Raw flax, wood and ore feed chains up to clothing, furniture and ploughs.
- [`people.ron`](people.ron) holds what people **want and have**: needs, tastes, and how good their land, forest and ore deposits are. Each person draws their own values from the ranges given.

People work out for themselves what any good is worth to them, including tools, which provide nothing directly but save hours. Every good can be made or bought, whichever costs that person fewer hours, so people with good land end up farming for others and people near rich ore end up mining.

Nothing is scripted. The shift out of farming comes from one relationship:

```
hours farming ≈ nourishment needed ÷ (food per hour × tool boost)
```

The need doesn't grow, so every gain in farm productivity frees hours for everything else. Better ploughs raise the tool boost, but an iron plough sits at the end of a chain: wood → charcoal, plus ore → iron, then iron + wood → plough. Trade lets each person take part in that chain at whichever step suits their land, forest and ore.

![Year 90](screenshot.png)

## Run

```sh
cargo run --release                                   # interactive window
cargo run --release -- --headless 300                 # 300 years as CSV on stdout
cargo run --release -- --goods g.ron --people p.ron   # use other definitions
cargo run --release -- --trade-cost 3 --headless 300  # start with a different trade cost
```

The first build compiles Bevy and takes several minutes. On Linux you also need the usual Bevy system packages (`libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`). See Bevy's [Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

## How people decide

1. **Needs first.** For each need, a person works on the cheapest good that provides it until the need is met, or the day runs out.
2. **Spare hours follow their own tastes,** a quarter-hour at a time: more of what they enjoy, leisure, or tools.
3. **Tools are valued by the hours they save.** A plough is worth the farming hours it saves each year, priced at what a spare hour was worth to that person last year, over the plough's discounted lifetime. Someone with no spare hours values a plough at nothing, so the poorest can't invest.
4. **Tools are whole and wear out at random.** A new tool lasts a number of years drawn from its `lifetime` range, then breaks and must be replaced. Hours put towards a tool build up as progress (or savings, if buying) until there's a whole one. A farmer works with the best tool they own, so an iron plough makes a wooden one pointless.
5. **Make or buy.** For each good, and each input, a person compares making it with earning the money at their job and buying it. Buying also costs time at market: the trade cost, a share of the price. Hours spent making inputs count in those sectors, and hours spent earning count in the job's sectors.
6. **Gradual choices.** The cheaper buying is compared with making, the larger the share of a good a person buys, rather than everyone flipping at once.
7. **Jobs.** A person's job is the good they make to sell; their wage is what an hour of it earns after paying for bought inputs. People start out self-sufficient. Each year about one in five reviews their job and picks one, where better-paying jobs are more likely but not certain.

## The market

All trade goes through one marketplace. Prices are measured in **hours of an average person's work**, so an iron plough "costs 22 hours". Each year, a good's price rises if more was bought than sold, and falls if the opposite. Buyers pay the price plus time at market; sellers get the price.

The market doesn't clear exactly. Once it has formed, about 4–5% of trade by value finds no counterpart in a given year: merchants absorb the gap, and prices move to close it. The panel shows this as "unmatched". In the first 10 years, while people are still joining, most trade is unmatched.

## The definitions

Edit either file and press **Reset** in the app to reload both without recompiling. Reset also redraws everyone.

**`goods.ron`**: one entry per good.

| Field | Meaning |
|---|---|
| `provides` | What one unit gives a person, e.g. `[("nourishment", 1.0)]`. Tools and inputs provide nothing |
| `recipe.rate` | Units made per hour of work |
| `recipe.uses` | A resource, e.g. `"ore"`, whose per-person quality scales the rate |
| `recipe.improves` | Share the rate grows by each year, e.g. `0.015` for 1.5%, scaled by the "technology improves" setting |
| `recipe.inputs` | Goods used up per unit, e.g. `[("iron", 2.0), ("wood", 2.0)]` |
| `recipe.tools` | Tools that multiply this recipe's output; a person works with the best one they own |
| `lifetime` | Makes the good a tool: the years it lasts before breaking, drawn from the range, e.g. `(15.0, 35.0)` |

**`people.ron`**: values are fixed (`1.0`) or a range (`(0.8, 1.2)`) that each person draws from.

| Field | Meaning |
|---|---|
| `endowments` | Quality of each resource (`land`, `forest`, `ore`) a person has access to; must be above zero |
| `needs` | Amount of a characteristic needed per day; met before anything else, in the order listed (nourishment, then warmth) |
| `preferences` | Enjoyment beyond the need: `Log(weight)` is always wanted at a falling rate; `Saturating(weight, scale)` is wanted until about `scale` extra |
| `leisure` | Weight on free time |
| `discount_rate` | How much less next year matters; with wear, sets how far ahead tools are valued |

The loaders reject unknown goods, duplicate names, tools without a lifetime (or one under a year) or with a multiplier below 1, recipes that are inputs to themselves, wants no good provides, and resources with no endowment.

## The panel

- **Technology improves, at N×:** multiplies every recipe's `improves` rate. 0× freezes technology; 2× doubles every rate of progress.
- **Trade cost:** time spent trading, as a share of the price. Drag it to 0% for free trade, or 200% to make trade pointless.
- **Production tree:** raw materials on the left, what people consume on the right. Solid arrows are inputs, dashed arrows are tools, and each node shows its share of all waking hours and its price.
- **Chart:** share of waking hours by sector, stacked to 100% with trading and leisure on top, plus a line per tool showing the share of people who own one, and a dashed line for specialisation.
- **Production rates and tools** (collapsed): a slider for every recipe rate and tool multiplier.

## Things to try

- **Watch the grid specialise.** Bars start as a mix of everything and sort into farmers (green), carpenters (orange), weavers and tailors (coral and pink), and a few foresters, miners and smiths.
- **Watch where freed time goes.** Farming shrinks, and so does weaving, which improves fastest (2% a year). Tailoring and carpentry improve slowly (0.5%), so they hold on to workers or grow: carpentry rises to about a third of the day by year 200. This is **Baumol's cost disease**: labour moves into the sectors whose productivity *doesn't* rise. Try giving carpentry `improves: 0.02` and see the hours go to leisure instead.
- **Change the trade cost and compare.** At year 100, compared with no trade:

  | At year 100 | Cheap trade (5%) | Default (30%) | No trade (300%) |
  |---|---|---|---|
  | Farming | 20% of the day | 20% | 32% |
  | Leisure | 31% | 29% | 28% |
  | Specialisation | 78% | 64% | 46% |

  These are the gains from trade: people with good land farm for everyone, and everyone gets more free time. Specialisation counts the share of a person's work in their main sector; many people buy part of a good and make the rest, so it stays well below 100%.
- **Watch years 2–10.** Cheap wooden ploughs spread first, then iron ploughs take over as a mining, charcoal and smelting chain forms.
- **Watch for an echo.** Iron-plough ownership peaks around year 20, then dips as that whole cohort of ploughs wears out at once, then recovers more smoothly.
- **Cause a shortage.** Open "Production rates and tools" and drag "ore per hour" down. Iron's price rises, iron ploughs disappear, and farming takes longer.
- **Add a second source of nourishment,** for example fish with `provides: [("nourishment", 1.0)]` and `recipe: (rate: 0.1)`, then press Reset. Nothing in the code knows fish and food are alternatives; they both provide nourishment.
- **Export CSV** writes `history.csv` with each good's rate and price, each sector's share of hours, trading time, specialisation, unmatched trade and tool ownership.

## Layout

| File | Contents |
|---|---|
| `goods.ron` | Every good: what it provides and how it's made |
| `people.ron` | What people need, enjoy and have access to, as ranges |
| `src/goods.rs` | Loads and validates the catalog; computes rates, tool boosts and embodied hours |
| `src/people.rs` | Loads and validates the population definition; tastes and their curves |
| `src/market.rs` | Prices, the trade cost, and each person's make-or-buy routes and job choice |
| `src/sim.rs` | People as ECS components (`Endowments`, `Needs`, `Tastes`, `Tools`, `Job`, `Trades`, …), and the `SimTick` schedule: advance technology → plan each day → replace tools → clear the market → record history |
| `src/ui.rs` | People grid (each person a bar of their day), control panel, production tree and stacked chart |
| `src/main.rs` | Windowed or headless entry point |

See [CHANGELOG.md](CHANGELOG.md) for what each stage added.

## Next: stage 5

Distance. Right now everyone trades at one marketplace for the same cost. If the trade cost grew with distance on the grid, markets would become local: towns could form where good land and ore sit close together, and a far-off village could stay self-sufficient. That would also let the grid stand for a map.
