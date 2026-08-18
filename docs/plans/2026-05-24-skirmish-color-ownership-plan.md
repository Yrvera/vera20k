# Skirmish Color Combo Ownership — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make color combos in the Skirmish setup dialog mutually exclusive across
player slots — picking a color in one row removes it from every other row's
dropdown, and picking the sentinel ("Random") or deactivating the row via
AI-type → None frees that color.

**Architecture:** Pure UI-layer change in `src/ui/skirmish_shell/state/`. Mirrors
the existing `start_position_taken_by_other_row` pattern in the same file. No
sim/, render/, or net/ changes. Per-slot `color_claimed: bool` field plus two
small helper functions plus three modified `apply_combo_selection` arms plus
one modified AI-type arm. Determinism: N/A — this is the host dialog, not
gameplay.

**Design Doc:** `docs/plans/2026-05-24-skirmish-color-ownership-design.md`

---

## Grounding Summary

- **What the docs say** (PROOFED via `/decode-system skirmish-cell-ui`, 2026-05-24):
  - `fn-fun-004e4c20-color-selection.md` — handler walks ownership table, clears
    self-entry, writes new entry, refreshes all combos.
  - `fn-fun-004e45a0-color-helper.md` — populates each color combo by including
    color X iff `ownership[X] == this_slot OR ownership[X] == None`.
  - `fn-fun-004e49a0-color-sentinel.md` — release-only path for sentinel/-2.
  - `fn-fun-004e43c0-color-label-loader.md` — initializes 9 label IDs at
    table base; Rust stays at 8 (HOUSE_COLOR_COUNT).
  - `fn-006adc20-row-enable.md` — AI-type → None cascade calls the color
    sentinel-release path; AI-type → active does NOT auto-claim a color.
  - `struct-colortableentry.md` — verified-body layout at base `0x8B4038` is
    `{+0x00 label_ptr, +0x04 swatch_rgb, +0x08 owner/flags}` (proofer-2 corrected
    the original INVERTED layout; the synthesis `_system.md §Edge cases #4`
    carries the correction).
- **What Ghidra verification confirmed:** all RE claims above are PROOFED upstream;
  this plan does not need to re-run Ghidra. Per CLAUDE.md "Internals are not the
  spec — outputs are," we do NOT mirror the C ownership table verbatim. We derive
  ownership from per-slot state — identical observable result, simpler internals.
- **Which repo pattern this mirrors:** `start_position_taken_by_other_row` /
  `selected_start_position` at `src/ui/skirmish_shell/state/combos.rs:386-410`
  and the Start arm of `combo_items` at lines 285-301. Same shape, same
  "self-owned items stay visible" exception.
- **Which INI keys drive behavior:** none. The 8 colors are hardcoded gameplay
  metadata (`HOUSE_COLOR_COUNT`); the dropdown items are RE-driven from
  `[Colors]` in `rulesmd.ini` for label/swatch, but the *exclusivity logic*
  itself reads no INI keys.
- **What's still unknown after grounding:**
  - All-8-claimed → activate behavior is unspecified by current decode
    (`fn-fun-006adc20-row-enable.md §Edge cases`). Implemented defensively per
    Tiny-Detail Ledger #11: activation does NOT auto-claim.
  - Sentinel + launch interaction: after the user picks sentinel for slot N,
    what color should the *launched* slot receive? Design treats the sentinel
    as "release only," leaving `color_index` as a cached prior selection that
    `launch_session` reads as-is. This matches stock gamemd's
    `SessionClass::ProcessRandomAssignments`-style late-binding model, but
    full sentinel→launch verification is deferred (see Deferred Open Questions).

## Key Technical Decisions

- **Approach A (derived ownership) over Approach B (mirror ownership table):**
  one source of truth per slot, no bidirectional sync to keep aligned.
  — **Confidence:** high — **Source:** design doc §Chosen Approach, mirrors the
  existing `start_position_taken_by_other_row` pattern.
- **`color_claimed: bool` field over `color: Option<u8>` rewrite:** preserves
  the cached prior color for default-on-reactivate semantics; avoids
  refactoring the render layer that reads `color_index` unconditionally.
  — **Confidence:** high — **Source:** design doc §Alternatives Considered,
  Tiny-Detail Ledger #6.
- **Explicit eviction in the color-selection handler:** when slot N picks
  color X, the handler must walk other slots and clear any prior claim on X
  before writing N's. Without this, Approach A's derived model can transiently
  show two rows both believing they own X.
  — **Confidence:** high — **Source:** design doc §Testing Strategy item 2 +
  §Known parity hazards #1.
- **AI-type → active does NOT auto-claim:** the AiType handler ANDs the
  preserved `color_claimed` with `row_type.is_active()`, so deactivating
  always releases and activating only re-asserts a previously-held claim.
  — **Confidence:** medium — **Source:** design doc Tiny-Detail Ledger #11 +
  `fn-fun-006adc20-row-enable.md`. The "all 8 claimed → activate" edge case
  is unspecified by current decode; defensive default is documented and
  tested explicitly.
- **Modify `SkirmishShellOpponent` in `state.rs` (not `player_name.rs` as the
  design says):** the design names `player_name.rs` for both struct edits,
  but `SkirmishShellOpponent` is actually defined in `state.rs:215-225`.
  `SkirmishShellState` lives in `player_name.rs:206-238`. Plan reflects the
  real file boundaries.
  — **Confidence:** high — **Source:** direct read of repo, this turn.
- **Leave `apply_action(SelectColor(...))` at `hit_test.rs:288-301` alone for
  now:** `SkirmishShellAction::SelectColor` is defined but never emitted by
  any code path (grep confirms `SelectColor(` only appears at its definition
  and handler). Dead-letter; flagged in §Deferred Open Questions for cleanup
  when wired in. Plumbing claim+eviction through it now is YAGNI.
  — **Confidence:** high — **Source:** grep of `src/`, this turn.

## Open Questions

### Resolved During Planning

- **Where does `SkirmishShellOpponent` live?** → `src/ui/skirmish_shell/state.rs:215-225`,
  not `player_name.rs` (design doc is slightly off).
- **Does the existing test `skirmish_color_dropdown_normal_population_omits_initialized_row_8`
  still pass?** → No. Default state has `opponents[0]` (Easy) claiming color 1, so
  slot 0's filtered dropdown would no longer contain `Color(1)`. The test must
  be updated to deactivate all AI rows before asserting the unfiltered list.
- **Does the existing test `combo_arrow_opens_dropdown_and_selects_color_row`
  still pass?** → Yes, by coincidence. The test clicks dropdown row index 4; in
  the default-claim filtered list `[sentinel, color(0), color(2), color(3), color(4),
  color(5), color(6), color(7)]`, row 4 is `color(3)` — same outcome as before.
- **Is `SelectColor` action live?** → No. Dead-letter, see Key Technical
  Decisions.
- **Does `launch_session` care about `color_claimed`?** → No. It reads
  `state.player_color_index` and `opponent.color_index` directly
  (`launch.rs:124,137`). The cached `color_index` is sufficient on its own —
  no `launch.rs` changes required.

### Deferred to Implementation

- **Sentinel + launch:** after a user clicks "Random" for a slot and then
  presses Start Game, gamemd assigns a color at session bootstrap via
  `SessionClass::ProcessRandomAssignments`. Verifying that path end-to-end
  against gamemd is a separate task — the current plan stays at the
  documented "use cached `color_index`" behavior. If a future verify-doc pass
  shows different observable output, revisit.
- **All-8-claimed activation:** if all 8 colors are claimed and a user
  changes an inactive row's AI type to Easy, the row activates without a
  color. Display behavior in the original at this corner case is unverified;
  defensive default is implemented and tested.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/ui/skirmish_shell/state/player_name.rs` | Add `player_color_claimed: bool` field to `SkirmishShellState`; update `Default` impl. |
| Modify | `src/ui/skirmish_shell/state.rs` | Add `color_claimed: bool` field to `SkirmishShellOpponent`; update `default_opponents()` to initialize per `row_type.is_active()`. |
| Modify | `src/ui/skirmish_shell/state/combos.rs` | Add `selected_color_claim` and `color_claimed_by_other_row` helpers; rewrite `SkirmishComboId::Color(_)` arm of `combo_items`; rewrite the four color/sentinel arms of `apply_combo_selection`; add eviction loop; modify the AiType arm of `apply_combo_selection`. |
| Modify | `src/ui/skirmish_shell/state/tests.rs` | Add unit tests for ownership semantics; update one existing test that assumes the unfiltered 8-color list. |

## Interface Changes

- `SkirmishShellState::player_color_claimed: bool` — new public field. Read by
  `combo_items(SkirmishComboId::Color(_))` filter; written by
  `apply_combo_selection(SkirmishComboId::Color(0), ...)` arms.
- `SkirmishShellOpponent::color_claimed: bool` — new public field. Read by
  `combo_items(SkirmishComboId::Color(_))` filter; written by
  `apply_combo_selection(SkirmishComboId::Color(row), ...)` arms and the
  `apply_combo_selection(SkirmishComboId::AiType(idx), ...)` arm.
- **Consumers checked:**
  - `src/ui/skirmish_shell/state/launch.rs:122-141` — reads
    `state.player_color_index` and `opponent.color_index` only. Unchanged.
  - `src/app_skirmish_shell_render/controls.rs` — reads
    `selected_combo_item(state, Color(_))` for the painted swatch. Unchanged.
  - `src/ui/skirmish_shell/state/hit_test.rs:288-301` — `SelectColor` action
    handler exists but is never emitted; left as-is (see §Deferred Open
    Questions).
  - Existing tests at `state/tests.rs:937,949,970,1047,1085,1088,1096,1504,1515`
    that set `player_color_index` / `opponents[N].color_index` directly: still
    valid; do not set `color_claimed`, so `Default` initialization continues to
    apply.

## Sim Checklist

Not applicable — no `sim/` files touched.

## Risk Areas

- **Eviction loop correctness** — the only place this plan diverges from a pure
  functional read. Mishandling lets two rows both think they own the same
  color. Covered by Task 6 tests 3 and 7.
- **`Default` impl ordering** — `SkirmishShellState::default()` calls
  `default_opponents(...)` and also initializes `player_color_claimed`. The
  two fields must agree on the "all default rows start with claims" invariant.
  Verified by Task 6 test 1.
- **Existing tests that exercise the color dropdown** — exactly one test
  (`skirmish_color_dropdown_normal_population_omits_initialized_row_8`)
  asserts the unfiltered 8-color list and breaks. Updated in Task 3.
- **Render layer regression** — `selected_combo_item(state, Color(_))` still
  reads the same `color_index`, regardless of `color_claimed`. The painted
  swatch behavior is unchanged. Manual visual verification deferred to
  Task 8 (full skirmish_shell test suite) which exercises rendering paths
  via the trackbar/dropdown integration tests.

## Parity-Critical Items

These are the player-observable behaviors the plan must produce. All are
hostroom-only (no in-game effect); player visibility is "every host dialog,
every match."

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 3 | Color X disappears from slot M's dropdown when slot N claims X | Every host dialog. Without this, two players show the same color in the lobby. | Task 6 test 2 (`color_claim_excludes_color_from_other_rows_dropdown`). Manual visual: open Skirmish, change player color to 4, verify AI 1's dropdown no longer lists color 4. |
| Task 4 | Eviction — picking color X frees any other row's prior claim on X | Every collision. Without it, two rows both believe they own X and the next refresh state is undefined. | Task 6 test 3 (`color_selection_evicts_prior_claimant`). |
| Task 4 | Sentinel ("Random") release frees the color for all other rows | Every "let the engine pick" choice. | Task 6 test 4 (`sentinel_release_makes_color_available_to_other_rows`). |
| Task 5 | AI-type → None releases the color | Common — disabling an AI slot in a smaller match. Without this, the freed slot's color stays unselectable everywhere. | Task 6 test 5 (`ai_type_none_releases_color`). |
| Task 5 | AI-type re-activation does NOT silently re-claim a color another row took | Niche but observable: deactivate AI 2 (color 5), activate AI 3, give AI 3 color 5, reactivate AI 2 — AI 2 must NOT re-grab color 5. | Task 6 test 6 (`ai_type_reactivate_does_not_auto_claim`). |
| Task 3 | Slot N's currently-selected color stays visible in slot N's own dropdown | Player must be able to re-select their current color (i.e., it can't be filtered out by its own claim). | Task 6 test 7 (`color_filter_keeps_self_selection_visible_per_row`). |

---

## Tasks

### Task 1: Add `color_claimed` fields and initialize defaults

**Why:** All downstream changes depend on this field existing on both the
player slot and each opponent. Done first as a pure additive change with no
behavioral consequence — every consumer continues to read `color_index`
unchanged, and `combo_items` doesn't yet consult `color_claimed`.

**Files:**
- Modify: `src/ui/skirmish_shell/state/player_name.rs:206-238` (struct + Default)
- Modify: `src/ui/skirmish_shell/state.rs:215-262` (opponent struct + default_opponents)

**Pattern:** Existing `enabled: bool` and `country_random: bool` per-opponent
booleans. New field follows the same shape — `pub`, `bool`, initialized in
the constructor function.

**Step 1: Add field to `SkirmishShellState`**

Edit `src/ui/skirmish_shell/state/player_name.rs`. Inside the
`SkirmishShellState` struct definition (currently lines 206-238), add the
new field right after `player_color_index`:

```rust
pub player_color_index: usize,
pub player_color_claimed: bool,
pub player_start_position: StartPosition,
```

Then update the `Default` impl (currently lines 240-277). After the
`player_color_index: 0,` line add:

```rust
player_color_index: 0,
player_color_claimed: true,
player_start_position: settings.start_position,
```

**Step 2: Add field to `SkirmishShellOpponent`**

Edit `src/ui/skirmish_shell/state.rs:215-225`. Add the new field after
`color_index`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellOpponent {
    pub enabled: bool,
    pub row_type: SkirmishAiRowType,
    pub country: SkirmishCountry,
    pub country_random: bool,
    pub color_index: usize,
    pub color_claimed: bool,
    pub start_position: StartPosition,
    pub team: i32,
    pub difficulty: AiDifficulty,
}
```

**Step 3: Initialize the new field in `default_opponents`**

Edit `src/ui/skirmish_shell/state.rs:233-262`. The current per-opponent
literal sets `enabled: idx == 0`. Mirror that for `color_claimed` keyed on
`row_type.is_active()` — which is true for the Easy default at index 0 and
false for the None-default rows 1..7:

```rust
.map(|(idx, country)| {
    let row_type = if idx == 0 {
        SkirmishAiRowType::Easy
    } else {
        SkirmishAiRowType::None
    };
    SkirmishShellOpponent {
        enabled: idx == 0,
        row_type,
        country,
        country_random: false,
        color_index: (idx + 1) % HOUSE_COLOR_COUNT,
        color_claimed: row_type.is_active(),
        start_position: StartPosition::Auto,
        team: -2,
        difficulty: AiDifficulty::Easy,
    }
})
```

Note: the original closure inlined `row_type` twice (once in the field
initializer, once via `if idx == 0`). The new closure binds it to a local so
the `color_claimed` initializer can call `.is_active()` without duplicating
the conditional. Equivalent semantics — only `idx == 0` ends up Easy/active.

**Step 4: Verify build**

Run: `cargo check -p ra2_game`
Expected: PASS (no semantic changes yet, just new field).

**Step 5: Commit**

```
state: add color_claimed flag to skirmish shell slots

Mirrors the start_position_taken_by_other_row ownership pattern. No
behavioral change yet — combo_items and apply_combo_selection still
ignore the flag; wired up in subsequent commits.
```

---

### Task 2: Add ownership helper functions in `combos.rs`

**Why:** Helpers are pure read-only functions over `&SkirmishShellState`.
Adding them before any caller exists lets us land them as a self-contained,
testable unit and reference them by name in Task 3.

**Files:**
- Modify: `src/ui/skirmish_shell/state/combos.rs` (add two functions near
  the existing `selected_start_position` / `start_position_taken_by_other_row`
  at lines 386-410)

**Pattern:** Direct mirror of `selected_start_position` (lines 386-395) and
`start_position_taken_by_other_row` (lines 397-410). Same shape — `row == 0`
branch for the player slot, `opponents[row - 1]` for AI rows.

**Step 1: Insert the two helpers**

After `start_position_taken_by_other_row` (which ends at line 410), insert:

```rust
fn selected_color_claim(state: &SkirmishShellState, row: usize) -> Option<usize> {
    if row == 0 {
        if state.player_color_claimed {
            Some(normal_color_index(state.player_color_index))
        } else {
            None
        }
    } else {
        state.opponents.get(row - 1).and_then(|opponent| {
            if opponent.color_claimed {
                Some(normal_color_index(opponent.color_index))
            } else {
                None
            }
        })
    }
}

fn color_claimed_by_other_row(
    state: &SkirmishShellState,
    row: usize,
    color: usize,
) -> Option<usize> {
    if row != 0
        && state.player_color_claimed
        && normal_color_index(state.player_color_index) == color
    {
        return Some(0);
    }
    state.opponents.iter().enumerate().find_map(|(idx, opponent)| {
        let opponent_row = idx + 1;
        if opponent_row != row
            && opponent.color_claimed
            && normal_color_index(opponent.color_index) == color
        {
            Some(opponent_row)
        } else {
            None
        }
    })
}
```

Notes:
- `selected_color_claim` mirrors `selected_start_position` but returns `None`
  when `color_claimed == false`. The filter uses this to decide whether the
  row's own current selection should be force-included.
- `color_claimed_by_other_row` mirrors `start_position_taken_by_other_row`
  but uses `find_map` (returning the owner row index) instead of `any`. The
  filter only needs `.is_none()` truthiness, but returning the row index
  also makes the helper useful for tests asserting *which* row holds a color.
- Both helpers go through `normal_color_index` (line 100) so an
  out-of-range `color_index` is capped exactly like the rest of `combos.rs`
  already does.

**Step 2: Verify build**

Run: `cargo check -p ra2_game`
Expected: PASS. The helpers are dead code until Task 3 wires them in — but
they're `fn` (not `pub fn`), so the Rust 2024 dead-code lint may fire.

If the dead-code warning fires, suppress it temporarily with
`#[expect(dead_code, reason = "wired up in next commit")]` on each helper.
Remove the attribute in Task 3 when the filter starts calling them.

**Step 3: Commit**

```
state/combos: add selected_color_claim and color_claimed_by_other_row

Pure read-only helpers over SkirmishShellState. Wired into combo_items
and apply_combo_selection in the next commit.
```

---

### Task 3: Wire the helpers into the `Color(_)` `combo_items` filter

**Why:** Makes the player-observable change — claimed colors stop appearing
in other rows' dropdowns. Lands before the eviction work (Task 4) so the
filter can be tested in isolation against fixtures that pre-set
`color_claimed` directly, without depending on the new selection-handler
logic.

**Files:**
- Modify: `src/ui/skirmish_shell/state/combos.rs:282-284` (current
  `SkirmishComboId::Color(_)` arm)
- Modify: `src/ui/skirmish_shell/state/tests.rs:1513-1543` (existing test
  that assumes the unfiltered 8-color list)

**Pattern:** Direct mirror of the `SkirmishComboId::Start(row)` arm at
`combos.rs:285-301` — sentinel-first, then loop with self-keep exception.

**Step 1: Replace the `Color(_)` arm**

In `combos.rs`, replace lines 282-284 (current arm):

```rust
SkirmishComboId::Color(_) => std::iter::once(SkirmishComboItem::ColorSentinel(-2))
    .chain((0..HOUSE_COLOR_COUNT).map(SkirmishComboItem::Color))
    .collect(),
```

with the filtered version, keyed on the row index:

```rust
SkirmishComboId::Color(row) => {
    let selected = selected_color_claim(state, row);
    let mut items = vec![SkirmishComboItem::ColorSentinel(-2)];
    for color in 0..HOUSE_COLOR_COUNT {
        if selected == Some(color)
            || color_claimed_by_other_row(state, row, color).is_none()
        {
            items.push(SkirmishComboItem::Color(color));
        }
    }
    items
}
```

If you applied `#[expect(dead_code, ...)]` in Task 2, remove those
attributes now — both helpers are live.

**Step 2: Update the broken existing test**

The existing test `skirmish_color_dropdown_normal_population_omits_initialized_row_8`
at `tests.rs:1513-1543` asserts the full `[sentinel, 0..7]` slot-0 dropdown.
With the new filter, `opponents[0]` (Easy by default) claims color 1, so
slot 0's dropdown loses `Color(1)`. Update the test to deactivate every AI
row first, so the dropdown returns the full unfiltered set:

```rust
#[test]
fn skirmish_color_dropdown_normal_population_omits_initialized_row_8() {
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 8;
    // Deactivate all AI rows so the filter sees no other claimants and the
    // dropdown matches the historical unfiltered set.
    for opponent in &mut shell.opponents {
        opponent.row_type = SkirmishAiRowType::None;
        opponent.color_claimed = false;
    }
    let maps = [test_map_entry("map.mmx")];

    let items = combo_items(&shell, &maps, SkirmishComboId::Color(0));

    assert_eq!(items.first(), Some(&SkirmishComboItem::ColorSentinel(-2)));
    assert_eq!(
        &items[1..],
        &[
            SkirmishComboItem::Color(0),
            SkirmishComboItem::Color(1),
            SkirmishComboItem::Color(2),
            SkirmishComboItem::Color(3),
            SkirmishComboItem::Color(4),
            SkirmishComboItem::Color(5),
            SkirmishComboItem::Color(6),
            SkirmishComboItem::Color(7),
        ]
    );
    assert!(!items.contains(&SkirmishComboItem::Color(8)));
    assert_eq!(
        selected_combo_item(&shell, SkirmishComboId::Color(0)),
        Some(SkirmishComboItem::Color(7))
    );
    assert_eq!(
        selected_combo_item_index(&shell, &maps, SkirmishComboId::Color(0)),
        Some(8)
    );
}
```

**Step 3: Verify build + sanity-check the unchanged existing test**

Run: `cargo test -p ra2_game --lib state::tests::combo_arrow_opens_dropdown_and_selects_color_row`
Expected: PASS. (Filtered dropdown row 4 in the default state happens to land
on `Color(3)`, same as before — verified in §Open Questions resolution.)

Run: `cargo test -p ra2_game --lib state::tests::skirmish_color_dropdown_normal_population_omits_initialized_row_8`
Expected: PASS after the update above.

**Step 4: Commit**

```
state/combos: filter Color(_) dropdown by per-row ownership

combo_items now hides colors claimed by another active row. The row's
own current selection is force-included so the player can re-pick what
they already have. apply_combo_selection still writes color_index
unconditionally; eviction and AI-type cascade land in the next commits.

Updates one existing test that pre-dated the filter.
```

---

### Task 4: Update the color selection handlers (claim, release, evict)

**Why:** This is the writer side of the ownership model. Each new arm sets
or clears `color_claimed`, and the `Color(N), Color(X)` path additionally
evicts any other slot's stale claim on X — without this, Approach A's
derived ownership model transiently allows two rows to both believe they
own X (see design §Testing Strategy item 2).

**Files:**
- Modify: `src/ui/skirmish_shell/state/combos.rs:489-497` (the four
  Color/Sentinel arms of `apply_combo_selection`)

**Pattern:** Existing color arms write `color_index`; new logic adds
`color_claimed = true|false` and (for Color) an eviction prelude that walks
all other slots and clears their claim if it would collide.

**Step 1: Replace the four arms**

In `combos.rs`, replace lines 489-497 (current four arms):

```rust
(SkirmishComboId::Color(0), SkirmishComboItem::Color(color)) => {
    state.player_color_index = color.min(HOUSE_COLOR_COUNT - 1);
}
(SkirmishComboId::Color(row), SkirmishComboItem::Color(color)) => {
    if let Some(opponent) = state.opponents.get_mut(row - 1) {
        opponent.color_index = color.min(HOUSE_COLOR_COUNT - 1);
    }
}
(SkirmishComboId::Color(_), SkirmishComboItem::ColorSentinel(_)) => {}
```

with the eviction-aware version:

```rust
(SkirmishComboId::Color(0), SkirmishComboItem::Color(color)) => {
    let color = color.min(HOUSE_COLOR_COUNT - 1);
    evict_other_color_claimants(state, 0, color);
    state.player_color_index = color;
    state.player_color_claimed = true;
}
(SkirmishComboId::Color(row), SkirmishComboItem::Color(color)) => {
    let color = color.min(HOUSE_COLOR_COUNT - 1);
    evict_other_color_claimants(state, row, color);
    if let Some(opponent) = state.opponents.get_mut(row - 1) {
        opponent.color_index = color;
        opponent.color_claimed = true;
    }
}
(SkirmishComboId::Color(0), SkirmishComboItem::ColorSentinel(_)) => {
    state.player_color_claimed = false;
}
(SkirmishComboId::Color(row), SkirmishComboItem::ColorSentinel(_)) => {
    if let Some(opponent) = state.opponents.get_mut(row - 1) {
        opponent.color_claimed = false;
    }
}
```

The four-arm replacement preserves the existing `_ => {}` fallthrough at
line 514 — mismatched id/item pairs continue to be silently ignored.

**Step 2: Add the `evict_other_color_claimants` helper**

After the `color_claimed_by_other_row` helper added in Task 2, insert:

```rust
/// Release any row other than `row` that currently claims `color`. Called
/// before writing `row`'s new claim so the derived ownership model never
/// shows two rows holding the same color simultaneously.
fn evict_other_color_claimants(
    state: &mut SkirmishShellState,
    row: usize,
    color: usize,
) {
    if row != 0
        && state.player_color_claimed
        && normal_color_index(state.player_color_index) == color
    {
        state.player_color_claimed = false;
    }
    for (idx, opponent) in state.opponents.iter_mut().enumerate() {
        let opponent_row = idx + 1;
        if opponent_row != row
            && opponent.color_claimed
            && normal_color_index(opponent.color_index) == color
        {
            opponent.color_claimed = false;
        }
    }
}
```

Notes:
- The function intentionally only clears `color_claimed`, leaving
  `color_index` intact — the evicted row keeps its cached prior color so the
  player can see what they had if they later re-pick from the now-shorter
  list (mirrors Ledger #6's "cached prior selection" semantics).
- The `row != 0` guard for the player branch is the same shape as the
  `color_claimed_by_other_row` helper's player branch.

**Step 3: Verify build**

Run: `cargo check -p ra2_game`
Expected: PASS.

**Step 4: Commit**

```
state/combos: claim, release, and evict on color selection

apply_combo_selection now sets/clears color_claimed and evicts other
rows' stale claims when a slot picks a color another slot was holding.
Tests in the next commit.
```

---

### Task 5: Cascade color release through the AI-type handler

**Why:** Closes the remaining ownership lifecycle gap. Without this,
deactivating an AI row leaves its color un-pickable elsewhere; activating
an AI row that previously had a color silently re-grabs it even if another
row took it during the gap.

**Files:**
- Modify: `src/ui/skirmish_shell/state/combos.rs:456-466` (the existing
  `(AiType(idx), AiType(row_type))` arm)

**Pattern:** Extends the existing AiType arm with one new line at the end.
Defensive AND of `row_type.is_active()` with the preserved
`opponent.color_claimed` — so deactivating always releases, but activating
only re-asserts a claim that was already true. Per Tiny-Detail Ledger #11.

**Step 1: Modify the AiType arm**

In `combos.rs`, locate the `(SkirmishComboId::AiType(idx), SkirmishComboItem::AiType(row_type))`
arm at line 456-466. Add one new line inside the `if let Some(opponent) = ...`
block, after the existing difficulty assignment:

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
        // Release color claim on deactivate; activation does not auto-claim
        // even if the row was previously holding a color — another slot may
        // have grabbed it during the deactivation gap.
        opponent.color_claimed = row_type.is_active() && opponent.color_claimed;
    }
}
```

The comment is intentional — it documents the defensive AND so a future
reader doesn't "simplify" it to `opponent.color_claimed = row_type.is_active()`,
which would re-introduce the silent-re-claim bug.

**Step 2: Verify build**

Run: `cargo check -p ra2_game`
Expected: PASS.

**Step 3: Commit**

```
state/combos: release AI color claim on AI-type deactivate

opponent.color_claimed is now ANDed with row_type.is_active() in the
AiType selection handler. Deactivating always releases; activating
only re-asserts a previously-held claim.
```

---

### Task 6: Unit tests for ownership semantics

**Why:** Locks in every parity-critical behavior from §Parity-Critical Items
as an executable spec. The hazards section of the design (`§Known parity
hazards to watch in implementation`) explicitly calls out the three tests
that protect the trickiest cases — eviction, self-keep, and AI re-activation.

**Files:**
- Modify: `src/ui/skirmish_shell/state/tests.rs` (append at end of file,
  before any closing module brace if present)

**Pattern:** Mirrors the existing `start_dropdown_omits_starts_reserved_by_other_rows`
at `tests.rs:1568-1581`. Each test constructs a fresh
`SkirmishShellState::default()`, mutates it to set up the scenario, and
asserts via `combo_items` or `apply_combo_selection` through the existing
public APIs.

**Step 1: Append the test block**

Add at the end of `tests.rs`:

```rust
#[test]
fn color_default_state_each_row_excludes_other_claimed_colors() {
    // Test 1 — Default state: every active row's filter excludes the other
    // 7 default colors and includes its own + sentinel.
    let mut shell = SkirmishShellState::default();
    // Activate every AI row so all 8 slots hold claims.
    for opponent in &mut shell.opponents {
        opponent.row_type = SkirmishAiRowType::Easy;
        opponent.color_claimed = true;
    }
    let maps = [test_map_entry("map.mmx")];

    for row in 0..SKIRMISH_AI_SLOT_COUNT + 1 {
        let items = combo_items(&shell, &maps, SkirmishComboId::Color(row));
        // Sentinel + exactly one color visible: this row's own.
        assert_eq!(items.len(), 2, "row {row} should see sentinel + self only");
        assert_eq!(items[0], SkirmishComboItem::ColorSentinel(-2));
        let expected_color = if row == 0 {
            shell.player_color_index
        } else {
            shell.opponents[row - 1].color_index
        };
        assert_eq!(items[1], SkirmishComboItem::Color(expected_color));
    }
}

#[test]
fn color_claim_excludes_color_from_other_rows_dropdown() {
    // Test 2 — Player claims color 4; AI row 1's filter loses color 4.
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 4;
    shell.player_color_claimed = true;
    // Make AI row 1 (slot index 0 in opponents) inactive so it doesn't
    // confuse the assertion with its own default claim.
    shell.opponents[0].row_type = SkirmishAiRowType::None;
    shell.opponents[0].color_claimed = false;
    let maps = [test_map_entry("map.mmx")];

    let items = combo_items(&shell, &maps, SkirmishComboId::Color(1));

    assert!(items.contains(&SkirmishComboItem::ColorSentinel(-2)));
    assert!(!items.contains(&SkirmishComboItem::Color(4)));
    // Spot-check that other colors are still present.
    assert!(items.contains(&SkirmishComboItem::Color(0)));
    assert!(items.contains(&SkirmishComboItem::Color(7)));
}

#[test]
fn color_selection_evicts_prior_claimant() {
    // Test 3 — Player picks color 5 while AI row 1 already claimed color 5.
    // After the selection, AI row 1 must no longer claim 5 (color_claimed
    // false), even though its cached color_index can remain.
    //
    // Note: the dropdown filter HIDES Color(5) from slot 0 (since slot 1
    // owns it), so we cannot drive this through handle_option_mouse_down
    // on a dropdown row. apply_combo_selection is the right entry point —
    // it is the same function the mouse handler calls, just bypassing the
    // dropdown row-index lookup. The eviction logic lives there, so this
    // exercises the production code path.
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 0;
    shell.player_color_claimed = true;
    shell.opponents[0].row_type = SkirmishAiRowType::Easy;
    shell.opponents[0].color_index = 5;
    shell.opponents[0].color_claimed = true;

    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::Color(0),
        SkirmishComboItem::Color(5),
    );

    assert_eq!(shell.player_color_index, 5);
    assert!(shell.player_color_claimed);
    assert!(
        !shell.opponents[0].color_claimed,
        "AI row 1 must have its claim evicted when the player takes its color"
    );
    // Cached color_index stays — the evicted row keeps its prior color in
    // case the user later re-picks from the dropdown.
    assert_eq!(shell.opponents[0].color_index, 5);
}

#[test]
fn sentinel_release_makes_color_available_to_other_rows() {
    // Test 4 — AI row 1 claims color 3; AI row 1 selects sentinel; AI row 2
    // can now see color 3 in its dropdown.
    let mut shell = SkirmishShellState::default();
    // Player off the relevant color so it doesn't confound the assertion.
    shell.player_color_index = 0;
    shell.player_color_claimed = true;
    shell.opponents[0].row_type = SkirmishAiRowType::Easy;
    shell.opponents[0].color_index = 3;
    shell.opponents[0].color_claimed = true;
    shell.opponents[1].row_type = SkirmishAiRowType::Easy;
    shell.opponents[1].color_index = 6;
    shell.opponents[1].color_claimed = true;
    let maps = [test_map_entry("map.mmx")];

    let before = combo_items(&shell, &maps, SkirmishComboId::Color(2));
    assert!(!before.contains(&SkirmishComboItem::Color(3)));

    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::Color(1),
        SkirmishComboItem::ColorSentinel(-2),
    );

    assert!(!shell.opponents[0].color_claimed);
    assert_eq!(shell.opponents[0].color_index, 3, "cached color preserved");
    let after = combo_items(&shell, &maps, SkirmishComboId::Color(2));
    assert!(after.contains(&SkirmishComboItem::Color(3)));
}

#[test]
fn ai_type_none_releases_color() {
    // Test 5 — AI row 1 (Easy, color 4) → None. color_claimed clears and
    // AI row 2's filter regains color 4.
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 0;
    shell.player_color_claimed = true;
    shell.opponents[0].row_type = SkirmishAiRowType::Easy;
    shell.opponents[0].color_index = 4;
    shell.opponents[0].color_claimed = true;
    shell.opponents[1].row_type = SkirmishAiRowType::Easy;
    shell.opponents[1].color_index = 7;
    shell.opponents[1].color_claimed = true;
    let maps = [test_map_entry("map.mmx")];

    let before = combo_items(&shell, &maps, SkirmishComboId::Color(2));
    assert!(!before.contains(&SkirmishComboItem::Color(4)));

    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::AiType(0),
        SkirmishComboItem::AiType(SkirmishAiRowType::None),
    );

    assert!(!shell.opponents[0].color_claimed);
    let after = combo_items(&shell, &maps, SkirmishComboId::Color(2));
    assert!(after.contains(&SkirmishComboItem::Color(4)));
}

#[test]
fn ai_type_reactivate_does_not_auto_claim() {
    // Test 6 — AI row 1 starts None+color_claimed=false; switching to Easy
    // must NOT silently set color_claimed to true. Another row may have
    // taken its prior color during the deactivation gap.
    let mut shell = SkirmishShellState::default();
    shell.opponents[0].row_type = SkirmishAiRowType::None;
    shell.opponents[0].color_index = 4;
    shell.opponents[0].color_claimed = false;

    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::AiType(0),
        SkirmishComboItem::AiType(SkirmishAiRowType::Easy),
    );

    assert_eq!(shell.opponents[0].row_type, SkirmishAiRowType::Easy);
    assert!(shell.opponents[0].enabled);
    assert!(
        !shell.opponents[0].color_claimed,
        "AI row 1 reactivation must NOT auto-claim its cached color"
    );
    // Cached color_index preserved — user can re-pick if still available.
    assert_eq!(shell.opponents[0].color_index, 4);
}

#[test]
fn color_filter_keeps_self_selection_visible_per_row() {
    // Test 7 — Every active row sees its own claimed color in its own
    // dropdown, even though every other row's filter would exclude it.
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 2;
    shell.player_color_claimed = true;
    shell.opponents[0].row_type = SkirmishAiRowType::Easy;
    shell.opponents[0].color_index = 5;
    shell.opponents[0].color_claimed = true;
    let maps = [test_map_entry("map.mmx")];

    let player_items = combo_items(&shell, &maps, SkirmishComboId::Color(0));
    assert!(player_items.contains(&SkirmishComboItem::Color(2)));
    assert!(!player_items.contains(&SkirmishComboItem::Color(5)));

    let ai_items = combo_items(&shell, &maps, SkirmishComboId::Color(1));
    assert!(ai_items.contains(&SkirmishComboItem::Color(5)));
    assert!(!ai_items.contains(&SkirmishComboItem::Color(2)));
}

#[test]
fn all_colors_claimed_activation_leaves_row_without_claim() {
    // Test 8 — Defensive Ledger #11. Claim all 8 colors across 8 active
    // rows, deactivate one, deactivate it again. Activating it does NOT
    // re-grab a color and does NOT steal another row's color.
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 0;
    shell.player_color_claimed = true;
    for (idx, opponent) in shell.opponents.iter_mut().enumerate() {
        opponent.row_type = SkirmishAiRowType::Easy;
        opponent.color_index = idx + 1; // colors 1..7 (HOUSE_COLOR_COUNT - 1)
        opponent.color_claimed = true;
    }
    // The 8th opponent currently holds color_index = 7 (valid for 8-color set).

    // Deactivate AI row 1 (slot index 0) — its color 1 is released.
    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::AiType(0),
        SkirmishComboItem::AiType(SkirmishAiRowType::None),
    );
    // Another slot grabs color 1 before AI row 1 reactivates.
    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::Color(2),
        SkirmishComboItem::Color(1),
    );
    // Reactivate AI row 1.
    apply_combo_selection_for_test(
        &mut shell,
        SkirmishComboId::AiType(0),
        SkirmishComboItem::AiType(SkirmishAiRowType::Easy),
    );

    assert_eq!(shell.opponents[0].row_type, SkirmishAiRowType::Easy);
    assert!(
        !shell.opponents[0].color_claimed,
        "reactivation must not silently re-grab a color another row took"
    );
    assert!(
        shell.opponents[1].color_claimed && shell.opponents[1].color_index == 1,
        "the other row's claim on color 1 must be preserved"
    );
}
```

**Step 2: Expose `apply_combo_selection` for tests**

The test-only `apply_combo_selection_for_test` shim is needed because
`apply_combo_selection` is a private `fn` (not `pub fn`) inside
`combos.rs`. Add a `#[cfg(test)]` re-export in `state.rs`. After the
existing `#[cfg(test)]` line at `state.rs:11` use block, add:

```rust
#[cfg(test)]
pub(super) use self::combos::apply_combo_selection as apply_combo_selection_for_test;
```

And confirm `apply_combo_selection` is visible to its parent module — it
currently has no visibility modifier, so it's module-private. Promote to
`pub(super)`:

In `combos.rs:450`, change:

```rust
fn apply_combo_selection(
```

to:

```rust
pub(super) fn apply_combo_selection(
```

This is the smallest visibility bump that lets the `state.rs` test re-export
work — the function remains private outside the `state` module.

**Step 3: Verify**

Run: `cargo test -p ra2_game --lib state::tests::color`
Expected: All 7 new color tests PASS plus the AI/state tests they call into.

Run: `cargo test -p ra2_game --lib state::tests`
Expected: Full state tests module PASS — no regressions.

**Step 4: Commit**

```
state/tests: cover color ownership semantics

Adds 8 tests for the new color_claimed lifecycle: default-state filter,
cross-row exclusion, eviction on claim, sentinel release, AI-type → None
cascade, AI-type reactivation defensive default, self-selection
visibility, and all-8-claimed reactivation edge case.

Promotes apply_combo_selection to pub(super) so the test module can drive
it through the same path the public combo-mouse-down handler uses.
```

---

### Task 7: Verify launch_session still works with `color_claimed = false` slots

**Why:** Cheap regression guard. The design states `launch_session` reads
`color_index` directly and the new bool doesn't affect it — verify that with
an explicit assertion so a future refactor can't silently drop the cached
color when `color_claimed` is false.

**Files:**
- Modify: `src/ui/skirmish_shell/state/tests.rs` (append one more test)

**Pattern:** Mirrors the existing `launch_session_packs_selected_map_and_enabled_slots`
at `tests.rs:932`.

**Step 1: Add the test**

Append to `tests.rs`:

```rust
#[test]
fn launch_session_uses_cached_color_index_when_claim_false() {
    // After picking the sentinel, color_claimed goes false but color_index
    // remains as the cached prior selection. launch_session must use that
    // cached value — gamemd's late-binding random assignment is a separate
    // concern (see plan §Deferred Open Questions).
    let mut shell = SkirmishShellState::default();
    shell.player_color_index = 3;
    shell.player_color_claimed = false;
    shell.opponents[0].row_type = SkirmishAiRowType::Easy;
    shell.opponents[0].color_index = 6;
    shell.opponents[0].color_claimed = false;
    let maps = [test_map_entry("map.mmx")];
    let modes = stock_skirmish_modes();

    let session = launch_session(&shell, &maps, &modes).expect("session");

    assert_eq!(session.local.color_index, 3);
    assert_eq!(session.opponents[0].color_index, 6);
}
```

**Step 2: Verify**

Run: `cargo test -p ra2_game --lib state::tests::launch_session_uses_cached_color_index_when_claim_false`
Expected: PASS.

**Step 3: Commit**

```
state/tests: lock launch_session's read-from-cached-color contract

Regression guard: even when color_claimed is false (after sentinel pick),
launch_session must read the cached color_index. Late-binding random
assignment is a separate scope (deferred).
```

---

### Task 8: Full skirmish_shell regression + cargo check

**Why:** Last-mile safety check. Confirms no other test in the workspace
silently depended on the old "always-8-colors" Color dropdown behavior, and
that the new fields don't trip any compile-time guard elsewhere.

**Files:** None modified directly — verification only.

**Step 1: Run the full skirmish_shell test suite**

Run: `cargo test -p ra2_game --lib state::tests`
Expected: PASS. Every existing test from the file (~80 tests) plus the 8
new ones from Tasks 6-7.

**Step 2: Run cargo check across the workspace**

Run: `cargo check`
Expected: PASS. If any consumer (renderer, app shell) silently broke on the
new struct fields, this catches it.

**Step 3: Run clippy on the touched files**

Run: `cargo clippy -p ra2_game --lib -- -D warnings 2>&1 | rg "skirmish_shell/state"`
Expected: no warnings on the touched files. If `find_map` triggers a
"use .any()" suggestion in `color_claimed_by_other_row`, ignore it — the
helper intentionally returns the row index for test ergonomics.

**Step 4: Manual visual smoke (if a build environment is handy)**

This step is not a hard gate (no integration harness for the shell yet),
but exercising it once catches drift that unit tests can't. Run the
skirmish shell, open the color dropdown for slot 0, change to color 5,
open AI row 1's dropdown — confirm color 5 is missing. Change AI row 1 to
None — confirm AI row 2's dropdown regains AI row 1's prior color.

**Step 5: Final commit**

Only commit if any fixes were needed in Steps 1-3. Otherwise the prior
commits already capture the change.

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-24-skirmish-color-ownership-design.md`
- **Ghidra reports (PROOFED 2026-05-24 via `/decode-system skirmish-cell-ui`):**
  - `ra2-rust-game-docs/skirmish-cell-ui/_system.md` — synthesis (corrected
    `colortableentry` layout in §Edge cases #4)
  - `ra2-rust-game-docs/skirmish-cell-ui/_parity.md` — 89-row parity report
  - `ra2-rust-game-docs/skirmish-cell-ui/fn-fun-004e4c20-color-selection.md`
  - `ra2-rust-game-docs/skirmish-cell-ui/fn-fun-004e45a0-color-helper.md`
  - `ra2-rust-game-docs/skirmish-cell-ui/fn-fun-004e49a0-color-sentinel.md`
  - `ra2-rust-game-docs/skirmish-cell-ui/fn-fun-004e43c0-color-label-loader.md`
  - `ra2-rust-game-docs/skirmish-cell-ui/fn-006adc20-row-enable.md`
  - `ra2-rust-game-docs/skirmish-cell-ui/struct-colortableentry.md` —
    corrected `{+0x00 label_ptr, +0x04 swatch_rgb, +0x08 owner/flags}` per
    proofer-2
- **gamemd.exe addresses** (kept here, not in Rust comments per CLAUDE.md
  `feedback_no_engine_refs_in_comments`):
  - `FUN_004E4C20` — color CBN_SELCHANGE handler
  - `FUN_004E45A0` — color combo population
  - `FUN_004E49A0` — color sentinel/release helper
  - `FUN_006ADC20` — AI row enable cascade
  - `FUN_004E43C0` — color label loader (initializes 9 string IDs)
  - `0x8B4038` — `g_ColorTableBase` (struct array base)
  - `FUN_004E4770` — spectator/observer color sentinel branch (out of scope)
  - `SessionClass::ProcessRandomAssignments` — late-binding random color
    assignment at session bootstrap (out of scope)
- **INI keys:** none — the exclusivity logic itself is data-free. Color
  labels and swatches still come from `rulesmd.ini [Colors]` via existing
  `src/rules/house_colors.rs`; that path is unchanged.
- **Related code:**
  - Pattern template — `src/ui/skirmish_shell/state/combos.rs:285-301`
    (Start arm) and `:386-410` (Start helpers)
  - `src/ui/skirmish_shell/state/launch.rs:122-141` — reads `color_index`
    only; unchanged
  - `src/ui/skirmish_shell/state/hit_test.rs:288-301` — `SelectColor` dead
    action; left as-is, flagged
  - `src/app_skirmish_shell_render/controls.rs` — paints swatch from
    `selected_combo_item(..., Color(_))`; unchanged
- **Prior commits:** branch is clean since `470fae5` ("Add native skirmish
  shell, loading flow, and PCX/PAL UI variants"). No mid-stream restructures
  to invalidate the design.

## Deferred Open Questions (post-plan)

- **`SkirmishShellAction::SelectColor` is dead.** Either wire it through
  the color-swatch click path (and route claim+eviction through it) or
  delete the action variant + handler. Separate cleanup; not blocking.
- **Sentinel + launch:** `launch_session` currently emits the cached
  `color_index` when `color_claimed == false`. Verify against gamemd's
  `SessionClass::ProcessRandomAssignments` whether the observable result
  matches; if not, plumb a "random color marker" through `SkirmishLocalSlot`
  / `SkirmishAiSlot`.
- **Spectator/observer color sentinel branch (`FUN_004E4770`)** — separate
  parity row in `_parity.md`. Different observable output (combo locks to
  single sentinel entry). Address when observer mode lands.
- **Session restore from `[Skirmish]` Slot00..07 INI** — initialization
  path that would set both `color_index` and `color_claimed` from
  persisted data. Separate large task.
- **9th color entry** — `FUN_004E43C0` initializes 9 string IDs vs
  `FUN_004E45A0`'s 8-entry iteration. Rust stays at `HOUSE_COLOR_COUNT = 8`.
  Revisit only if a 9th color appears in retail gameplay.
