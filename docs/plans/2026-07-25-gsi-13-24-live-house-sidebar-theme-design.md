# GSI-13.24 Live-House Sidebar Theme Design

Date: 2026-07-25

Status: self-approved after adversarial repair; implementation not yet started

## Goal

Close the earliest load-bearing divergence in LOOP-005 stage 6: an ordinary
explicit Soviet or Yuri skirmish whose editable local player name is `Player`
or `Commander` currently selects Allied sidebar chrome for the entire match.

This slice changes only local theme authority. Exact GCLOCK progress timing and
frame selection remain the next residual inside GSI-13.24.

## Native Contract

Active `gamemd.exe` selects the local side from the chosen house/country type,
not from a map display-name roster:

- `[Sides]` processing writes the side index to `HouseTypeClass+0xBC`.
- `Read_Scenario` resolves the first session-node country, reads
  `HouseTypeClass+0xBC`, and stores it at `Scenario+0x34B8`
  (`0x0068479D..0x006847C9`).
- `Full_Init` repeats the non-campaign selection at
  `0x00687794..0x00687833`; the campaign/local-house arm reads
  `HouseClass+0x34 -> HouseTypeClass+0xBC`.
- Stock indices are Allied `0`, Soviet `1`, Yuri `2`.

The former `NewSidebar`/theater label for `Scenario+0x34B8` was stale and has
been corrected in the research corpus.

Grounding:

- `docs/research/COUNTRY_SIDE_TYPE_CLASSES.md` §4.3/§5;
- `docs/research/SCENARIO_INIT_DEEP_DIVE.md`, correction note and phase 11;
- `docs/research/INIT_LAYOUT_CONSTANTS_GHIDRA_REPORT.md` §2;
- `docs/research/SIDE_MIXFILE_INIT_GHIDRA_REPORT.md` §1.

## Current Rust Closed Loop

1. `SkirmishLaunchSession.player_name` carries the editable local name.
2. `normalized_launch_slots` and `populate_launch_houses` create a live
   `HouseState` under that dynamic name with the selected
   `LaunchCountry::side_index()` value.
3. match transition pins the dynamic name in `local_player_owner`.
4. `preferred_local_owner_name` returns the pinned owner.
5. `current_sidebar_theme` calls a resolver that searches only the parsed map
   `HouseRoster`.
6. The dynamic explicit owner normally has no roster row, so the resolver
   returns `None` and `current_sidebar_theme` falls back to Allied.

The wrong theme flows into chrome, GCLOCK texture and palette route, Ready
tint, gadget dimensions and hit rectangles, tooltip regions, and neighboring
radar/sidebar consumers.

## Architecture and Impact

The app/render layer may read immutable `Simulation` house identity. No
simulation module will depend on render/UI code, and no deterministic state,
RNG, scheduler order, tick timing, asset order, or draw order changes.

Affected production surface:

- `src/app_sidebar_render.rs`: extract one GPU-free source resolver and route
  `current_sidebar_theme` through it.
- `src/app_skirmish.rs`: add full explicit-launch regression coverage using the
  exact resolver used by production.

No app-state field, launch-state duplicate, country-name inference, or
simulation mutation is introduced.

## Chosen Resolution Order

The pure resolver accepts the current optional `Simulation`, `HouseRoster`,
and preferred owner name:

1. Preserve the existing matching `HouseRoster` decision exactly.
2. Only when no roster row matches the owner, look up the owner in live
   `sim.houses`.
3. Map only valid live side indices: `0 -> Allied`, `1 -> Soviet`,
   `2 -> Yuri`.
4. If the live owner is absent or its side index is unknown, return `None`;
   `current_sidebar_theme` retains its existing final Allied fallback.

Roster-first is intentional and fidelity-monotonic for this bounded slice.
Generic/map-loaded `HouseState.side_index` can currently default to Allied when
a stock map supplies `Country=` but no `Side=`; all sampled loose stock maps
omit `Side=`. Making every live house globally authoritative now could degrade
working map-roster paths. The ordinary explicit dynamic owner is distinguishable
because it has no roster row, so its launch-created live side closes the target
without replacing existing behavior.

The remaining collision case—an editable explicit player name that exactly
matches a map roster row—is documented, not silently certified.

## Alternatives Rejected

### Add explicit launch houses to `HouseRoster`

Rejected because it duplicates live house authority and expands the meaning of
a map-parsing structure. Other roster consumers could change.

### Cache `SidebarTheme` in `AppState` at launch

Rejected because it creates derived state that can drift from the pinned owner
and dev/sandbox owner switching.

### Make all live `HouseState.side_index` values override the roster

Rejected for this slice because generic stock map houses do not yet have a
proven side-population path. This could turn a correctly inferred Soviet roster
row into Allied.

### Infer from player name, country display string, or asset availability

Rejected because native authority is numeric HouseType side identity. Display
strings and fallback assets are consumers, not authority.

## Player-Experience Ledger

- Milestone-blocking: explicit Soviet/Yuri local player receives Allied sidebar
  throughout ordinary play.
- Compounding: one wrong theme selects chrome, GCLOCK, button geometry, tint,
  hit-test, and tooltip behavior.
- Preserved behavior: every owner already resolved by `HouseRoster` follows the
  exact pre-slice path.
- Unknown-risk fallback: missing/invalid live side still reaches the existing
  final Allied fallback.
- Residual: exact owner-to-GPU/swapchain pixels remain unverified; the existing
  ignored retail test proves only selected Yuri theme through side-two retail
  assets, palette, CPU GCLOCK atlas, and instance construction.

## Validation Design

Red-first, GPU-free tests:

1. Apply a complete explicit launch with `player_name = "Commander"`,
   bases disabled, and zero starting-unit budget, then call the exact pure
   resolver used by `current_sidebar_theme`.
2. Feed `SkirmishLaunchApplyResult.local_owner` into that resolver rather than
   repeating the fixture string, proving launch output -> theme authority.
3. Table-cover America -> Allied, Russia -> Soviet, and Yuri -> Yuri.
4. Prove an owner absent from live simulation preserves the existing
   `HouseRoster` resolution.
5. Prove a matching roster row precedes an unproven/conflicting live side,
   guarding against the identified map-house degradation.
6. Prove an unknown live side with no roster row returns `None` rather than
   silently becoming Allied inside the helper.

Downstream validation:

- focused `app_skirmish` and sidebar-theme tests;
- ignored retail Yuri generic-route/GCLOCK production-load-path test;
- `cargo check -q -p vera20k`;
- production binary build;
- branch and post-merge validation under the single Cargo lease.

No desktop control is required. No pixel-parity claim will be made.

## Residuals

- Generic/map-loaded house side population and canonical stock country aliases
  (`Russians`, `Africans`, `Arabs`, `Confederation`) remain unexact.
- A dynamic player name colliding with a map roster row remains roster-first.
- Allied-hardcoded radar animation initialization is a neighboring residual.
- GCLOCK frame 1 suppression, ETA-derived float reconstruction, lost factory
  progress steps, auxiliary CameoEntry smoothing, and exact tick visibility
  remain the next stage-6 work.

## Adversarial Review and Self-Approval

Initial review verdict was REPAIR:

- exercise the exact production resolver, not only `HouseState.side_index`;
- preserve Allied and fallback regressions;
- do not map unknown side values to Allied inside the helper;
- account for map houses whose live side defaults to zero.

The repaired design addresses each objection with one shared pure seam,
roster-first monotonic ordering, valid-side-only mapping, and full launch tests.

Final independent design review: `APPROVE`, no P1 findings. Its P2 requests
(explicit research-document paths and passing the launch result's
`local_owner` into the resolver test) are incorporated above.

Why should this be approved? It fixes the ordinary dynamic-owner break using
the already-created live side byte, changes no deterministic state, removes no
working fallback, and can be validated through the production resolver without
desktop control.

What evidence could still make it wrong? Proof that ordinary explicit launch
owners are always inserted into `HouseRoster`, that `HouseState.side_index`
does not preserve the resolved launch country, or that a matching roster row
must be overridden in the target path. Current source traces disprove the first
two; the third is intentionally retained as a named residual rather than
risking generic-map degradation.

Decision: APPROVE this bounded design for implementation after the current
Cargo/dev owner releases both leases and the feature worktree incorporates the
exact released `dev`.
