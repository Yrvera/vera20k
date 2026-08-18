# Skirmish Color Ownership Design

## Goal

Make color combos in the Skirmish setup dialog mutually exclusive across player slots — once a slot claims a color, that color disappears from the other rows' dropdowns, and is freed when the slot picks the sentinel ("Random") or is deactivated via AI-type → None.

## Architecture Context

**Rust skirmish shell** lives at `src/ui/skirmish_shell/`. The relevant module is
`src/ui/skirmish_shell/state/combos.rs`, which already implements an analogous
"exclusive across rows" pattern for the Start-position combo via
`start_position_taken_by_other_row` (lines 397–410). That function is the
template this design follows.

**Current color handling (no exclusivity):**

- `combo_items(SkirmishComboId::Color(_))` returns `[ColorSentinel(-2), Color(0)..Color(7)]` unconditionally (`combos.rs:282–284`).
- `apply_combo_selection(SkirmishComboId::Color(0), ...)` writes `state.player_color_index = color` (`combos.rs:489–491`).
- `apply_combo_selection(SkirmishComboId::Color(row), ...)` writes `opponent.color_index = color` (`combos.rs:492–496`).
- `apply_combo_selection(SkirmishComboId::Color(_), ColorSentinel(_))` is a no-op (`combos.rs:497`).
- No global ownership table; selecting the same color in two rows leaves both rows holding it. Player-visible drift.

**Slot model:** `SkirmishShellState` has `player_color_index: usize` for slot 0 and a `Vec<SkirmishShellOpponent>` for AI slots 1–7, each opponent carrying `color_index: usize`. There is no current "no claim" representation — every row always has *some* `color_index` value, even when its row is inactive.

**gamemd mechanism** (for reference, not for verbatim porting):

```
FUN_004E4C20 (color CBN_SELCHANGE handler):
  1. Walk the 8-entry ownership table; clear the entry whose owner == this slot
  2. Read new selection's item-data (color index, or -2 for sentinel)
  3. If not sentinel: write ownership[selected_color] = this_slot
  4. Refresh all 8 color combos

FUN_004E45A0 (color combo population):
  For each color X in table:
    If ownership[X] == this_slot OR ownership[X] == None:
      add color X to dropdown

FUN_006ADC20 (per-row enable, AI-type → None):
  Calls FUN_004E49A0(row, -2) to release the row's color ownership
```

Per CLAUDE.md "Internals are not the spec — outputs are," we don't mirror the C struct. We derive ownership from existing per-slot state.

## Impact Analysis

**Files touched:**

- `src/ui/skirmish_shell/state/combos.rs` — main changes (filter, selection handlers, helper)
- `src/ui/skirmish_shell/state/player_name.rs` — add one field to `SkirmishShellState` and one to `SkirmishShellOpponent`, plus default init
- `src/ui/skirmish_shell/state/tests.rs` — add unit tests
- (Possibly) `src/skirmish_launch.rs` — verify `launch_session` still works correctly when a slot has no claimed color

**Files NOT touched (verified):**

- Render layer (`src/app_skirmish_shell_render*`) — reads `color_index` for the selected swatch; that field still exists with the same semantics when claimed. No changes needed.
- `src/ui/skirmish_shell/state/launch.rs` — reads `opponent.color_index`. We'll need to handle "slot has no claim" but the existing color-cycle path already maps to `(color+1)%8` so a default exists.

**Blast radius:** small. The change is additive (one new bool per row + one helper function + two small selection-handler updates) and mirrors an existing pattern in the same file.

**Determinism:** not a sim/ change; no lockstep concern.

**Migration:** none. Existing state initialization paths get one new default value (`color_claimed: true` for slot 0, `true` for default-active AI rows, `false` for inactive rows).

## Chosen Approach

**Approach A — Derived ownership.** Add a `color_claimed: bool` per slot (player + each opponent). Ownership is *derived* from per-slot state via a `color_claimed_by(state, color_index)` helper that walks rows and returns the first row where `color_claimed && color_index == query`.

This mirrors how `start_position_taken_by_other_row` already works — derived from per-row state, no separate ownership array.

**Why not Approach B (mirror gamemd's table):** redundant state; same color would live in both the opponent's `color_index` field and the ownership table. Two sources of truth invites sync bugs.

**Why not Approach C (Option<u8> everywhere):** wider refactor of working rendering code for stylistic gain only. YAGNI; same observable behavior is achievable with A.

## Tiny-Detail Ledger

Constraints the implementation must preserve. Each item must have a clear home in the design.

| # | Detail | Source | Where it lives in design |
|---|---|---|---|
| 1 | 8 color entries (Rust HOUSE_COLOR_COUNT = 8) | `[doc: fn-fun-004e45a0]` | Unchanged; `combo_items` keeps `(0..HOUSE_COLOR_COUNT)` |
| 2 | Sentinel `-2` ("Random") is the FIRST item in every color dropdown | `[doc: fn-fun-004e45a0 §Random/empty item]` | Unchanged; `combo_items` keeps `std::iter::once(ColorSentinel(-2)).chain(...)` order |
| 3 | Filter rule: include color X for slot N iff `owned_by[X] == N OR owned_by[X] == None` | `[doc: fn-fun-004e45a0 §Color table iteration]` | New `color_claimed_by_other_row(state, row, color)` helper called inside `combo_items(Color)` filter |
| 4 | Selection cascade: refresh ALL combos when any slot changes color | `[doc: fn-fun-004e4c20 §Phase 4]` | Implicit in state-driven render — `combo_items` is recomputed every paint, so updating one slot's claim instantly affects the others' next-frame dropdowns |
| 5 | Release before claim: clear N's previous ownership before writing new | `[doc: fn-fun-004e4c20 §Phase 1]` | Automatic — we only write a single slot's `color_index` / `color_claimed`; the prior value is overwritten in place |
| 6 | Sentinel select = release only (no new claim) | `[doc: fn-fun-004e4c20 line 78-81]` | `apply_combo_selection(Color(N), ColorSentinel(_))` sets `color_claimed = false`; leaves `color_index` as-is (acts as cached prior selection for default-on-reactivate) |
| 7 | AI-type → None cascade releases color | `[doc: fn-fun-006adc20 + fn-fun-004e49a0]` | `apply_combo_selection(AiType(idx), None)` sets `opponent.color_claimed = false` (plus existing row-type write) |
| 8 | Initial state: dialog opens with all rows holding their default color claim | `[doc: fn-fun-004e43c0 + fn-fun-004e48e0]` | `SkirmishShellState::default()` sets `player_color_claimed = true` and `opponent.color_claimed = row_type.is_active()` |
| 9 | Slot 0 follows same rules as AI rows | `[doc: fn-fun-004e45a0 control-ID table]` | `combo_items(Color(0))` uses the same filter as `Color(row)` |
| 10 | Currently-selected color must stay visible in its OWN combo | `[mirror of start_position pattern]` | Filter exempts the row's own selection (same as start-position pattern: `selected == Some(...) \|\| !taken_by_other`) |
| 11 | UNKNOWN — if all 8 colors are claimed and an inactive row activates, behavior is unspecified by current decode | `[UNKNOWN — needs RE]` | Implemented defensively: activation does NOT auto-claim a color; row stays with `color_claimed = false` and shows only sentinel + colors it can legally pick (zero in worst case). Tested explicitly. |
| 12 | Out of scope: spectator/observer sentinel branch | `[doc: fn-fun-004e4770]` | Separate parity row; not addressed here |
| 13 | Out of scope: persistence to `[Skirmish]` INI Slot00..07 | `[doc: SessionClass__ReadSkirmishSettings]` | Separate session-restore task |

## Design

### Components

**State changes** (in `state/player_name.rs`):

- `SkirmishShellState` gains: `pub player_color_claimed: bool` (default `true`).
- `SkirmishShellOpponent` gains: `pub color_claimed: bool` (default depends on `row_type.is_active()` at construction).

**New helpers** (in `state/combos.rs`):

```rust
/// Returns Some(row_index) if any active+claimed row OTHER than `row` owns this color, else None.
fn color_claimed_by_other_row(
    state: &SkirmishShellState,
    row: usize,
    color: usize,
) -> Option<usize>;

/// Returns the current color claim for `row`, or None if `row` is sentinel.
fn selected_color_claim(state: &SkirmishShellState, row: usize) -> Option<usize>;
```

The `selected_color_claim` returns `None` when the row's `color_claimed == false`, otherwise `Some(row's color_index)`. Used by the filter's "include the row's own current selection" exception.

### Interfaces / Contracts

**`combo_items(Color(row))` — modified filter:**

```rust
SkirmishComboId::Color(row) => {
    let selected = selected_color_claim(state, row);
    let mut items = vec![SkirmishComboItem::ColorSentinel(-2)];
    for color in 0..HOUSE_COLOR_COUNT {
        if selected == Some(color) || color_claimed_by_other_row(state, row, color).is_none() {
            items.push(SkirmishComboItem::Color(color));
        }
    }
    items
}
```

Structurally identical to the existing `Start(row)` arm. Sentinel is always present at index 0.

**`apply_combo_selection(Color(N), ...)` — modified to set/clear claim:**

```rust
(SkirmishComboId::Color(0), SkirmishComboItem::Color(color)) => {
    state.player_color_index = color.min(HOUSE_COLOR_COUNT - 1);
    state.player_color_claimed = true;
}
(SkirmishComboId::Color(0), SkirmishComboItem::ColorSentinel(_)) => {
    state.player_color_claimed = false;
}
(SkirmishComboId::Color(row), SkirmishComboItem::Color(color)) => {
    if let Some(opponent) = state.opponents.get_mut(row - 1) {
        opponent.color_index = color.min(HOUSE_COLOR_COUNT - 1);
        opponent.color_claimed = true;
    }
}
(SkirmishComboId::Color(row), SkirmishComboItem::ColorSentinel(_)) => {
    if let Some(opponent) = state.opponents.get_mut(row - 1) {
        opponent.color_claimed = false;
    }
}
```

**`apply_combo_selection(AiType(idx), ...)` — release color on deactivate (ledger #7):**

```rust
(SkirmishComboId::AiType(idx), SkirmishComboItem::AiType(row_type)) => {
    let team_default = inactive_ai_team_default(state);
    if let Some(opponent) = state.opponents.get_mut(idx) {
        opponent.row_type = row_type;
        opponent.enabled = row_type.is_active();
        opponent.team = team_default;
        if let Some(difficulty) = row_type.difficulty() {
            opponent.difficulty = difficulty;
        }
        // NEW: release color on deactivate; keep claim on activate
        opponent.color_claimed = row_type.is_active() && opponent.color_claimed;
    }
}
```

Note the AND with `opponent.color_claimed`: activating doesn't auto-claim (ledger #11 — defensive); deactivating always releases.

### Data Flow

1. Player clicks color combo arrow → `handle_combo_mouse_down` opens dropdown.
2. Render path calls `combo_items(Color(row))` → filter returns the row's view of claimable colors.
3. Player clicks an item → `apply_combo_selection(Color(row), Color(X))` writes `color_index = X, color_claimed = true`.
4. Next paint: every row's `combo_items(Color(N))` recomputes; rows where `N != row` now exclude color X (since `color_claimed_by_other_row(state, N, X) == Some(row)`).
5. Player picks the sentinel → `color_claimed = false`. Color is now visible to all other rows.
6. Player changes AI row M's type to None → `opponent.color_claimed = false`. M's color is freed.

No explicit "refresh all combos" call needed — the filter is recomputed every frame, so updates propagate instantly via state-driven render.

### Error Handling

- Defensive caps on color values via `color.min(HOUSE_COLOR_COUNT - 1)` (already present, kept).
- `color_claimed_by_other_row` is total: returns `None` if `color` is out of range (no panic).
- `apply_combo_selection` continues to silently ignore mismatched id/item pairs via the existing `_ => {}` arm.

### Testing Strategy

Unit tests in `state/tests.rs`:

1. **Default state:** all 8 default rows + player have claims; filter for any row excludes the other 7 colors.
2. **Player picks color 3, AI row 1 had color 3:** AI row 1's dropdown no longer shows color 3 (since the player now claims it). AI row 1's `color_index` is unchanged but its `color_claimed` is whatever it was — if it was true, the filter still excludes it; in practice the filter only checks ownership, so two rows pointing at color 3 with both claimed = true is a transient state we shouldn't allow.

   Wait — there's a gotcha. If the player picks color 3 and an AI row also had color 3 claimed, both now have `color_claimed = true` on color 3. The filter says "any *other* row claiming this color" — so both rows think they own it. Resolution: when row N picks color X, we should explicitly release any other row's claim on color X. Add to `apply_combo_selection(Color(N), Color(X))`:

   ```rust
   // Before writing N's claim, release any other row that currently claims X
   if state.player_color_claimed && state.player_color_index == color && N != 0 {
       state.player_color_claimed = false;
   }
   for (idx, opp) in state.opponents.iter_mut().enumerate() {
       if opp.color_claimed && opp.color_index == color && idx + 1 != N {
           opp.color_claimed = false;
       }
   }
   ```

   With this, the post-condition is: only one row at most claims any given color.

3. **Sentinel picked then real color picked again:** test the round-trip.
4. **AI-type → None releases color:** verify the AI row's color is gone from the ownership view; another row can now claim it.
5. **AI-type → Easy reactivates without claiming color:** verify defensive ledger #11 — activation does not auto-claim.
6. **Player can pick the color they already have selected:** filter must keep "self-owned" colors visible. (`selected_color_claim` exception.)
7. **All 8 colors claimed:** trying to claim a color forces another row to release it.

## Architectural Decisions

**Patterns followed:** the design directly mirrors `start_position_taken_by_other_row` / `selected_start_position` in the same file. Same shape, same conventions, same "self-owned items stay visible" exception. No new patterns introduced.

**Patterns deviated:** none.

**Tech debt:** the "evict any other claimant" logic in `apply_combo_selection(Color(N), Color(X))` is necessary because the state-driven approach lets a transient "two rows claim color X" appear if we don't actively evict. This is the one small cost of Approach A vs Approach B. The eviction is a single linear scan (8 rows × 1 color check). Acceptable.

**Determinism:** N/A (UI layer, not sim/).

## Alternatives Considered

- **Approach B (mirror gamemd's `[Option<usize>; 8]` table):** rejected for bidirectional sync cost. Would need to keep the table and the per-row `color_index` aligned through every UI action, every init path, and every test. Approach A derives ownership and avoids the sync burden.
- **Approach C (`color: Option<u8>` instead of `color_index: usize` + bool):** rejected for refactor blast radius. Same observable output via A, with less risk to the render layer that already works.

## Out of scope (deferred parity items)

- **Spectator/observer color sentinel-only path** (`FUN_004E4770`) — separate MISSING row in `_parity.md`. Different observable output (the entire combo becomes single-entry locked). Should be its own brainstorm if/when observer mode lands.
- **Session restore from `[Skirmish]` INI** — separate large task. The `color_claimed` boolean's default in this design is `true` for active rows, matching the "fresh skirmish" case. When session restore lands, restoration logic should set both `color_index` and `color_claimed` per the persisted slot triple.
- **9-color reconciliation** — `FUN_004E43C0` initializes 9 entries (string IDs `0x1DB..0x1E3`); `FUN_004E45A0` iterates 8 (`0x8B4040..0x8B40A0`). Either one entry is reserved/unused, or `FUN_004E43C0` includes the "random" label. Rust stays at 8 colors per `HOUSE_COLOR_COUNT`; revisit if a visible 9th color appears in retail gameplay.

## Known parity hazards to watch in implementation

1. **The eviction step** (Testing Strategy item 2) is the one place where this design diverges from a pure functional read — make sure the unit tests cover the "another row was holding it" case explicitly.
2. **The "currently-selected stays visible" exception** must be tested per-row, not just for slot 0. Mirror the start-position test pattern.
3. **AI-type → Easy after Easy → None** must NOT silently re-claim a color another row took during the deactivation gap. The defensive `color_claimed = row_type.is_active() && opponent.color_claimed` covers this — verify in tests.
