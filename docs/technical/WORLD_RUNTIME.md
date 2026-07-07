# Matter World Runtime

Matter now has a `world` backend for the multiplayer problem that started this thread: one logical world cannot be one naive shared server when thousands of players need realtime state.

The first implementation is deliberately concrete:

- `world.configure(cell_size, interest_radius, cell_capacity, max_visible)`
- `world.place(id, x, y)` / `world.spawn(id, x, y)` / `world.move(id, x, y)`
- `world.nearby(id)`
- `world.plan()`
- `world.status()`

## Model

The runtime treats the map as a logical world split into spatial cells. Each cell has a capacity. When a cell is overloaded, entities are assigned deterministic layers. `world.nearby(id)` returns only same-layer entities inside the interest radius, capped by `max_visible`, and reports hidden overflow as aggregate pressure.

This is the base rule:

```text
logical world
-> spatial cells
-> deterministic layers for hot cells
-> interest radius
-> capped visible set
-> hidden overflow as aggregate signal
```

That is the practical answer to the "thousands of players in one map" problem: the language can expose a single world model while the runtime keeps visibility, routing, and overload explicit.

## Demo

```powershell
cargo run -q -p matter-cli -- run examples\world_runtime_demo.matter
```

Expected behavior:

- `p1`, `p2`, and `p3` land in the same cell.
- Cell capacity is `2`, so the cell becomes hot and creates layers.
- `p1` sees two relevant entities and one nearby hidden overflow.
- `world.plan()` reports `hot_cells=1` and `degraded=true`.

## What This Is Not

This is not a network server yet. It is the language/runtime contract that a server, scheduler, or cluster coordinator can use later.

The correct production path is:

```text
world backend contract
-> routing and interest model
-> cell ownership
-> cross-cell event transfer
-> server process orchestration
-> persistence and replay
```
