# Skirmish Player-Table Restoration Design

## Goal

Make the ordinary offline Skirmish player table open with retail-convincing
native chrome and locally persisted player presentation, without changing the
verified dialog geometry.

## Architecture Context

Dialog `0x102` is rendered by the existing skirmish-shell app renderer.
`src/ui/skirmish_shell/layout.rs` owns the resource-derived rectangles;
`src/app_skirmish_shell_render/controls.rs` owns combo faces and swatches;
`src/app_skirmish_shell_render/text.rs` owns collapsed labels; and
`src/app_skirmish_session.rs` already owns the process-lifetime `RA2MD.INI`
snapshot and hydrates the shell before first paint.

The native edit control is seeded from the current player-name global rather
than a literal. The installed retail `RA2MD.INI` supplies the corresponding
local preferences in `[MultiPlayer]`:

- `Handle=5b,4e,65,77,20,50,6c,61,79,65,72,5d` -> `[New Player]`;
- `Color=2`;
- `Side=Americans`;
- `ColorEx=-1`;
- `SideEx=-1`.

The original reference screenshot shows that same name, blue color slot 2, and
America, while current Rust falls back to `Player`, color slot 0, and an
explicit start position. The existing app configuration profile remains a
higher-priority explicit user override for the player name.

## Impact Analysis

Expected production touchpoints:

- `src/app_skirmish_session.rs`
  - read bounded `[MultiPlayer]` local preferences beside the existing
    `[Skirmish]` snapshot;
  - hydrate player name, concrete stock country, and concrete color when valid.
- `src/ui/skirmish_shell/state/player_name.rs`
  - initialize the local start combo to `Auto` for a fresh shell.
- `src/app_skirmish_shell_render/controls.rs`
  - use the verified native fixed eight-entry swatch table.
- `src/app_skirmish_shell_render/text.rs`
  - resolve the collapsed `Auto` start label through
    `GUI:RandomAsSymbols`, just like the dropdown.

No changes are planned for `src/ui/skirmish_shell/layout.rs`. Existing unrelated
working-tree changes in `src/app_skirmish_shell_render.rs` and
`src/app_skirmish_shell_render/chrome.rs` are outside this design and will be
preserved.

The changes affect front-end presentation and launch inputs only. They do not
alter simulation tick order, RNG consumption, entity state, or deterministic
gameplay.

## Chosen Approach

Use the existing offline-skirmish runtime as the single owner of both relevant
`RA2MD.INI` sections:

1. Keep the current `[Skirmish]` snapshot behavior.
2. Add a small internal local-preferences value loaded from `[MultiPlayer]`.
3. Decode the comma-separated hexadecimal `Handle`, cap it through the existing
   19-character edit-state constructor, and apply it during shell hydration.
4. Map a valid `Side` token through `SkirmishCountry::country_name()`.
5. Apply a valid `Color` in the native normal range `0..7`.
6. Leave malformed or unsupported fields at current Rust defaults.
7. Let an explicitly configured VERA20k profile name override the retail INI
   handle, preserving the current precedence.
8. Keep start selection independent of persisted player preferences and default
   the fresh local row to `Auto`.
9. Draw color faces from the native UI table rather than rules color schemes.

This approach matches the architecture already used for `[Skirmish]`, avoids a
second file reader in `app.rs`, and does not hardcode the user's current retail
values.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — The player table must retain the verified fixed
  rectangles, 24px combo faces, 26px row stride, 20px arrow reserve, and 2px
  text/swatch insets. Moving these controls would create visible drift.
  [doc: `SKIRMISH_0X102_PLAYER_TABLE_VISUAL_CHROME_SPACING_RECHECK_GHIDRA_REPORT.md`
  §§ Exact logical geometry, Visual composition ledger]
- `MILESTONE-BLOCKING` — Local name is state sourced, not the literal
  `"Player"`. The active native dialog seeds edit `0x6A0` from
  `DAT_00A8B380`; the installed retail preference decodes to `[New Player]`.
  [GHIDRA `0x006AE6F2..0x006AE735`; retail `RA2MD.INI [MultiPlayer] Handle=`]
- `MILESTONE-BLOCKING` — The local color face must use the persisted valid slot
  when present. The installed retail state is slot 2, matching the reference
  screenshot. [retail `RA2MD.INI [MultiPlayer] Color=2`; screenshot]
- `MILESTONE-BLOCKING` — Lobby swatches are the fixed binary table, not
  `[Colors]` HSV schemes. [GHIDRA `0x008316A8`, `0x004E43C0`;
  doc: `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md` §3]
- `MILESTONE-BLOCKING` — A fresh unreserved start selection is Random/Auto and
  displays localized `GUI:RandomAsSymbols`; the 38px collapsed face must show
  the symbols rather than clipped `"Random"`. [GHIDRA `0x004E50C0`;
  doc: `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md` §3]
- `COMPOUNDING` — The app configuration profile name stays the highest-priority
  explicit user setting. Reversing precedence would make configuration appear
  ineffective. [current `src/app.rs` initialization order]
- `COMPOUNDING` — Invalid `Handle`, `Color`, or `Side` values must not prevent
  shell construction. Each preference falls back independently.
- `EXACTIFICATION-RESIDUAL` — `ColorEx` and `SideEx` semantics outside the
  ordinary stock offline range remain deferred. The installed standard fixture
  uses `-1`, so they do not affect this representative scenario.
- `EXACTIFICATION-RESIDUAL` — Exact DirectDraw packed-color conversion and
  final font pixels remain unchecked; source RGB, geometry, asset, and label
  paths are preserved honestly without claiming pixel parity.
- `UNKNOWN-RISK` — Writing changed local name/color/side back to
  `[MultiPlayer]` is not part of this visual opening-state slice. Existing
  in-process shell state remains authoritative after edits; cross-process
  preference write parity should be handled in a dedicated persistence slice.

## Design

### Components

`OfflineSkirmishRuntime` gains an internal local-preference snapshot. It is
constructed from the same bytes already read for `RA2MD.INI`, so no new I/O
owner or app-layer coupling is introduced.

The renderer gains a named constant table for the eight native lobby swatches.
The existing swatch call sites continue to receive a color index; only their
source data changes.

The collapsed start label uses the existing localization helper and same CSF key
as dropdown items.

### Interfaces / Contracts

- Preference parsing is private to `app_skirmish_session`.
- Handle decoding accepts comma-separated hexadecimal bytes and rejects an
  entirely invalid/empty value.
- Country matching is ASCII-case-insensitive against the stock
  `country_name()` values.
- Color is applied only for indices `0..8`.
- Shell hydration applies retail preferences before the optional app-profile
  name override in `app.rs`.
- Existing public render/state interfaces remain unchanged where possible.

### Data Flow

```text
RA2MD.INI bytes
  -> [Skirmish] persisted snapshot
  -> [MultiPlayer] local preferences
  -> OfflineSkirmishRuntime::hydrate_shell
  -> optional configured profile-name override
  -> existing row initialization/reservation repair
  -> first skirmish paint
```

### Error Handling

Missing file, missing section, malformed hex, unknown side, and out-of-range
color are non-fatal and independently retain current defaults. Existing warning
behavior for file read/parse failures remains sufficient; no player-facing
modal is added.

### Testing Strategy

Focused tests will cover:

- decoding the installed-style hexadecimal handle;
- rejecting malformed/empty handle data without damaging other preferences;
- hydrating `[New Player]`, America, and color slot 2 from a temporary fixture;
- configured profile precedence remains untouched by keeping the existing
  post-hydration override;
- local shell start defaults to `Auto`;
- collapsed Auto uses `GUI:RandomAsSymbols`;
- all eight native swatch RGB values, including proof that differing rules
  color schemes cannot change them.

Run the focused skirmish session/state/render tests serially, then one
`cargo check -q` if no other session owns Cargo. Final visual acceptance is the
user's 800x600 runtime screenshot.

## Architectural Decisions

The design follows the existing app-owned process-lifetime persistence pattern
and keeps UI rendering above `sim/`. No new dependency direction is introduced.
The verified resource layout remains the single geometry authority.

The only deliberate deferral is `[MultiPlayer]` write-back and extended
preference semantics. It is recorded rather than silently represented as native
parity.

## Alternatives Considered

### Renderer-only correction

Change only swatches and the Random label. This is smallest, but it leaves the
most conspicuous screenshot differences (`Player`, gold, wrong side authority)
despite the original state being available in the retail INI. Rejected as
insufficient for the requested restoration.

### Hardcode the reference screenshot

Set `[New Player]`, America, and blue directly in shell defaults. This would look
right for one machine but ignore real retail preferences and break other users'
chosen state. Rejected as parity drift.

### Full `[MultiPlayer]` read/write parity now

Model every base/extended/network preference and update them on dialog close.
This is architecturally plausible but broadens a visual restoration into a
larger persistence contract with unverified `ColorEx`/`SideEx` branches.
Deferred in favor of the bounded read/hydrate slice.

