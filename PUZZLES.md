# Cleanup Crew – Planet Puzzle Design

## Overview

Each set of 3 space stations hides clues for 3 types of puzzles on the
following planet. All 3 clue types are collected across the same 3-station
cycle, one piece per station. On the planet, solve 3 sub-puzzles using the
station clues to earn signal strengths, then enter those signal strengths
into the Frequency Master terminal to unlock the best vault.

---

## Station Phase  (3 stations per planet cycle)

Each station spawns exactly **one pickup of each clue type** in a different room:

| Clue Type       | Sprite Tint | Values      | HUD Label |
|-----------------|-------------|-------------|-----------|
| Number Fragment | Cyan        | Digit 0–9   | CODE      |
| Color Chip      | Matching    | 4 colors    | CLR       |
| Symbol Chip     | Purple      | 6 symbols   | SYM       |

**Colors:** Red / Green / Blue / Yellow  
**Symbols:** ▲ ● ■ ⬡ ✦ ⊕

All collected clues appear in the HUD (bottom-right corner) and in the
TAB screen under STATION CLUES.

---

## Planet Phase  (one planet per 3-station cycle)

### Puzzle 1 — Code Door  (map tile: `C`)

- **Clue source:** 3 number digits collected across the 3 stations
- **Mechanic:** Walk up to the door, press E to open the keypad.
  Use ↑↓ to change a digit (0–9), ←→ to move between the 3 slots, E to submit.
- **On solve:** Door opens. Small vault with loot unlocks.
  **Signal Strength A** is revealed (a value 1–5).

---

### Puzzle 2 — Color Terminal  (map tile: `K`)

- **Clue source:** 3 color clues collected across the 3 stations
- **Mechanic:** Same keypad UI as the code door.
  Each slot cycles through the 4 colors: RED → GRN → BLU → YLW.
  Set each slot to the color found on the matching station.
- **On solve:** Terminal unlocks. Small vault with loot unlocks.
  **Signal Strength B** is revealed (a value 1–5).

---

### Puzzle 3 — Symbol Terminal  (map tile: `Y`)

- **Clue source:** 3 symbol clues collected across the 3 stations
- **Mechanic:** Same keypad UI. Each slot cycles through the 6 symbols:
  ▲ → ● → ■ → ⬡ → ✦ → ⊕.
  Set each slot to the symbol found on the matching station.
- **On solve:** Terminal unlocks. Small vault with loot unlocks.
  **Signal Strength C** is revealed (a value 1–5).

---

### Puzzle 4 — Frequency Master  (map tile: `F`)

- **Clue source:** Signal Strengths A, B, C earned from puzzles 1–3
- **Mechanic:** Same keypad UI. Each slot cycles through values 1–5,
  displayed as signal bar graphs (▰▱▱▱▱ through ▰▰▰▰▰).
  Requires all 3 signal strengths to be revealed before the terminal
  becomes active.
- **On solve:** The **master vault** opens — highest-tier loot in the level.

---

## Signal Strengths — How They Work

Signal strengths are **randomly generated when the planet loads** and stored
internally. They are hidden until you solve the corresponding sub-puzzle.
When you solve puzzle 1, the game shows "Signal Strength: 3" (for example)
as a popup. Write it down or check the TAB screen — the inventory panel
shows all 3 signals once revealed.

Signal strengths are **not carried between planet cycles** — each new planet
generates a fresh set.

---

## Inventory Screen (TAB)

The TAB screen shows the STATION CLUES section on the right panel:

```
── STATION CLUES ──
  CODE   [ 4 ] [ ? ] [ 7 ]
  CLR    [RED] [ ? ] [BLU]
  SYM    [ ▲ ] [ ? ] [ ⊕ ]
  SIG    [ 3 ] [ ? ] [ ? ]   ← revealed as you solve sub-puzzles
```

---

## Developer Notes

- All 4 puzzles reuse the existing `CodeDoor` keypad UI pattern from `src/planet.rs`.
- Map tile characters: `C` = code door, `K` = color terminal, `Y` = symbol terminal, `F` = freq master.
- Signal strengths live in `PlanetSignals { signals: [Option<u8>; 3] }` — NOT persisted to `SavedPlayerBuffs`.
- Station clues (`StationCodes`, `StationColors`, `StationSymbols`) DO persist via `SavedPlayerBuffs` across stations.
- All clues reset to `[None; 3]` after the planet is cleared and the next 3-station cycle begins.
