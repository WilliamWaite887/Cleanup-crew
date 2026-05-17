# Planet Level Design Reference

Game loop: 3 stations → Planet 1 → 3 stations → Planet 2 → 3 stations → Planet 3

Each planet cycle the player collects 3 clue types from stations:
- **Code Fragment** (cyan pickup) → `StationCodes.codes[i]` — digit 0–9
- **Color Chip** (red pickup) → `StationColors.colors[i]` — 0=RED 1=GRN 2=BLU 3=YLW
- **Symbol Chip** (purple pickup) → `StationSymbols.symbols[i]` — 0=▲ 1=● 2=■ 3=⬡ 4=✦ 5=⊕

All 3 clue resources persist across stations and reset after PlanetWin.

---

## Planet 1 — Sequential Terminals

**Theme:** Learn the sequence. Each terminal feeds the next.

### Flow

```
Stations → collect code / color / symbol
  ↓
Code Door (C tile) — enter 3-digit code → receive Signal A (random 1–5)
  ↓
Color Terminal (K tile) — enter 3 colors → receive Signal B (random 1–5)
  ↓
Symbol Terminal (Y tile) — enter 3 symbols → receive Signal C (random 1–5)
  ↓
FreqMaster (F tile) — enter signals A/B/C → boss arena gate opens
  ↓
Fight main boss
```

### Mechanics

- All 4 terminals are at fixed, accessible locations — no exploration required
- Terminals must be solved roughly in order (FreqMaster locked until all 3 signals exist)
- Player tracks 3 collected clues → 3 generated signals = 6 values total, but only 3 at a time
- Vault (optional reward room, C tile code door at col 23): same 3-digit code unlocks it

### Tile Characters

| Tile | Component | Purpose |
|------|-----------|---------|
| `C` | `CodeDoor` | 3-digit code entry; on solve → Signal A |
| `K` | `ColorTerminal` | 3-color entry; on solve → Signal B |
| `Y` | `SymbolTerminal` | 3-symbol entry; on solve → Signal C |
| `F` | `FreqMaster` | 3-signal entry; on solve → opens boss arena |

### Files

- Map: `assets/planet/planet1_level.txt` (300×200)
- Code: `src/planet.rs` — `build_planet1_rooms()`, all terminal/signal systems
- Signals resource: `PlanetSignals { signals: [Option<u8>; 3] }`

---

## Planet 2 — Terminal-Decoded Dials

**Theme:** Solve first, then hunt. Terminals tell you what to set; dials are hidden in rooms.

### Flow

```
Stations → collect code / color / symbol
  ↓
Terminal A (K2 tile) — enter 3-digit code → reveals Dial A target (random 0–9)
Terminal B (L tile) — enter 3 colors    → reveals Dial B target (random 0–3)
Terminal C (Z tile) — enter 3 symbols   → reveals Dial C target (random 0–5)
  ↓
Explore level — find Dial A, B, C buried in 3 enemy rooms
Set each dial to its target value (W/S to cycle, Enter to confirm)
  ↓  (all 3 dials correct)
Boss door (P tile) becomes interactive — press E to open
  ↓
Fight main boss
```

### Mechanics

- Terminals are in accessible areas; dials are in enemy rooms deeper in the level
- Dial targets are random values generated on terminal solve (not the raw station clues)
- Each dial is locked until its corresponding terminal is solved ("LOCKED — solve terminal first")
- Dial visual states: dim (unset) → neutral (wrong) → green tint (correct)
- Boss door shows "Calibration incomplete" while any dial is wrong/unset
- Vault included (same `C` tile + 3-digit code system as Planet 1)

### Escalation vs Planet 1

| | Planet 1 | Planet 2 |
|---|---|---|
| Terminal interactions | 4 (3 clue + FreqMaster) | 3 (clue entry only) |
| Physical dials | 0 | 3 (in enemy rooms) |
| Boss gate | Keypad entry (FreqMaster) | E press after all dials correct |
| Navigation demand | Low (fixed landmarks) | High (dials hidden in rooms) |
| Total distinct interactions | 4 | 7 (3 terminals + 3 dials + boss door) |

### New Tile Characters

| Tile | Component | Purpose |
|------|-----------|---------|
| `K2` / `L` / `Z` | Code/Color/Symbol terminal (P2 variant) | On solve → sets `DialTargets[i]` |
| `B` | `DialButton { dial_idx, dial_type, current }` | Physical dial in enemy room |
| `P` | `PlanetBossDoor` | Opens when all dials correct |

> Note: exact tile characters for P2 terminals TBD during map design — may reuse K/Y/C with planet-index dispatch.

### New Code (Planet 2)

**New resource:**
```rust
pub struct DialTargets {
    pub targets: [Option<u8>; 3],  // [A, B, C]; set by terminal solves
}
```

**New components/systems in `src/planet.rs`:**
- `DialButton { dial_idx: usize, dial_type: DialType, current: u8 }`
- `DialType` enum: `Code | Color | Symbol`
- `dial_proximity` — proximity + E key opens single-slot keypad UI
- `update_dial_ui` — W/S cycle value, Enter confirm; locked if `DialTargets[i]` is None
- `check_all_dials` — each frame; marks `PlanetBossDoor.ready` when all dials match targets
- `PlanetBossDoor { ready: bool }`
- `planet_boss_door_proximity` — shows "[E] Open" when ready; "Calibration incomplete" otherwise
- Terminal solve hook: on correct P2 terminal solve, generate random target and set `DialTargets[i]`

**Tile parsing in `src/map.rs`:**
- `B` → spawn `DialButton`
- `P` → spawn `PlanetBossDoor`

**Dispatch in `src/planet.rs`** (add `planet_idx == 1` arms):
- `planet_map_file` → `"assets/planet/planet2_level.txt"`
- `planet_boss_spawn` → P2 boss position
- `planet_vault_rewards` → P2 vault reward positions
- `build_planet_rooms` → `build_planet2_rooms()`

### Map Layout Principles (Planet 2)

- Spawn room: south/east (same convention as P1)
- Terminal rooms: near spawn, accessible before clearing enemy rooms
- 3 enemy rooms (each with one `B` dial): distributed across the level — no required order
- Boss arena: north/west; `P` boss door at its entrance
- Vault: optional reward room locked by `C` code door (same as P1)
- Map size: 300×200 (same as P1) — `assets/planet/planet2_level.txt` (to be created)

---

## Planet 3 — Dial → Mini Boss → Signal → Main Boss

**Theme:** Everything you've learned, combined. Set dials to unlock a mini boss; kill it to get signals; enter signals to unlock the real boss.

### Flow

```
Stations → collect code / color / symbol
  ↓
Solve Terminal A/B/C → receive Dial targets A/B/C (same as Planet 2)
  ↓
Find Dial A/B/C in enemy rooms → set each to its target
  ↓  (all 3 dials correct)
Mini boss gate (G2 tile) opens
  ↓
Enter mini boss arena → fight mini boss (weaker than main boss)
  ↓  (mini boss dies)
Receive Signal X, Y, Z (random 1–5 each) — displayed as popup
  ↓
Override Terminal (O tile) — enter X/Y/Z (same as P1 FreqMaster)
  ↓
Main boss arena gate opens
  ↓
Fight main boss
```

### Mechanics

- Phases 1–2 identical to Planet 2 (terminals → dial targets → set dials)
- Mini boss gate: opens automatically when all dials are correct; locked otherwise
- Mini boss is a scaled-down version of the main boss (e.g. 0.5× health multiplier)
- On mini boss death: `PlanetSignals` resource populated with 3 random values (1–5)
- Override Terminal: same `TerminalKind::Freq` keypad as Planet 1's FreqMaster
  - Locked ("Need all signals") until `PlanetSignals` is fully populated
  - Correct entry opens main boss arena gate
- No vault on Planet 3

### Escalation vs Planet 2

| | Planet 2 | Planet 3 |
|---|---|---|
| Terminals | 3 | 3 |
| Physical dials | 3 | 3 |
| Mini boss fight | No | Yes |
| Signal system | No | Yes (same as Planet 1) |
| Override Terminal | No | Yes (FreqMaster equivalent) |
| Total distinct steps | 3 phases | 5 phases |
| Values to track | 3 dial targets | 3 dial targets + 3 signals = 6 |

### New Tile Characters (Planet 3)

| Tile | Component | Purpose |
|------|-----------|---------|
| `B` | `DialButton` | Same as Planet 2 |
| `G2` | Mini boss gate | Opens when all dials correct |
| `O` | `OverrideTerminal` | FreqMaster-equivalent; gated on PlanetSignals |

> Planet 3 terminals may reuse same tile chars as P2 with planet-index dispatch.

### New Code (Planet 3)

**New component:**
```rust
pub struct OverrideTerminal { pub unlocked: bool }
// gated: TerminalKind::Freq logic, but checks PlanetSignals not the old signal chain
```

**New systems in `src/planet.rs`:**
- `mini_boss_gate_check` — each frame; removes gate collision when all dials correct
- `watch_mini_boss_death` — detects mini boss death; generates PlanetSignals; shows popup
- `OverrideTerminal` proximity + session: re-use `TerminalSession` + `TerminalKind::Freq` flow

**Dispatch (add `planet_idx == 2` arms):**
- `planet_map_file` → `"assets/planet/planet3_level.txt"`
- `planet_boss_spawn` → P3 main boss position
- `planet_vault_rewards` → empty slice (no vault)
- `build_planet_rooms` → `build_planet3_rooms()`

### Map Layout Principles (Planet 3)

- Spawn room: south/east
- Terminal rooms: near spawn (accessible early)
- 3 enemy rooms (each with one `B` dial): mid-level, require exploration
- Mini boss arena: sealed room between dial rooms and main boss area; gate at entrance
- Override Terminal: antechamber between mini boss room and main boss arena
- Main boss arena: far north
- No vault
- Spatial flow: spawn → terminals → dial rooms → mini boss gate → mini boss arena → override terminal → main boss
- Map size: 300×200 — `assets/planet/planet3_level.txt` (to be created)

---

## Shared Implementation Notes

### Existing Systems to Reuse

| System | Used by |
|--------|---------|
| `StationCodes / StationColors / StationSymbols` | P2 + P3 terminal solve input |
| `TerminalSession` keypad UI (W/S/A/D/Enter) | P2 terminals, P3 Override Terminal |
| `PlanetSignals { signals: [Option<u8>; 3] }` | P3 post-mini-boss signal entry |
| `aabb_overlap()` proximity check | All new interaction systems |
| `FreqMaster` proximity/UI pattern | P3 Override Terminal |
| `build_planet1_rooms()` | Reference for room bounds format |
| Dispatch match in `planet_map_file/boss_spawn/vault_rewards/build_planet_rooms` | Add P2/P3 arms |

### Implementation Order

1. Design `assets/planet/planet2_level.txt` ASCII map
2. Add P2 tile parsing (`B`, `P`) in `src/map.rs`
3. Implement `DialTargets`, `DialButton`, `PlanetBossDoor` systems in `src/planet.rs`
4. Wire P2 terminal solve → `DialTargets` generation
5. Add P2 dispatch arms; test full P2 loop end-to-end
6. Design `assets/planet/planet3_level.txt` ASCII map
7. Add `O` tile parsing and `OverrideTerminal` in `src/map.rs` + `src/planet.rs`
8. Implement mini boss gate, mini boss death → `PlanetSignals` pipeline
9. Add P3 dispatch arms; test full P3 loop end-to-end
