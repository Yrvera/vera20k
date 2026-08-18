# Skirmish 0x102 — [MultiplayerDialogSettings] Key→Widget DRIFT Map (Slice 4F / O5)

**Address(es):** `0x006AE2C0` (offline launcher → creates dialog `0x102`), `0x006AE3F0` (dialog proc),
`0x006AE6E0` (init seeds controls), `0x006ACEE0` (Start packing re-reads controls), `0x00671EA0`
(`RulesClass__ReadMultiplayerDialogSettings`), `0x00697F10` (`SessionClass__ReadSkirmishSettings`),
`0x00665650` (`RulesClass` constructor seed).
**Investigation Mode:** exhaustive-slice (synthesis over three prior HIGH-confidence binary reports)
**Claimed Scope:** For the standard offline Yuri's Revenge Skirmish setup dialog `0x102`, classify each of
the 25 `[MultiplayerDialogSettings]` keys (plus the two widget-only keys `SuperWeaponsAllowed`/`BuildOffAlly`)
as **surfaced-as-widget** vs **stored-only**, and flag any surfaced option widget the Rust shell is MISSING.
**Non-Scope:** the in-game Options dialog; the online/WOL/LAN network-game lobby (a different dialog surface
the Rust shell does not target — see §5); per-row Side/Color/Start/Team/AI combos (separate mechanism, already
modeled); gameplay consumers of the stored options (covered by the packed-option report); CSF English text.
**Confidence:** HIGH. The `0x102` child inventory is a binary RT_DIALOG extraction (all 72 children, none
missing — matrix OQ-8), and the control→key bindings + init/packing paths were decompiled in the cited
reports. Cross-confirmed by three independent reports, the Rust shell's existing control set, and the master
plan's independent count.
**Active in YR:** Yes. `FUN_006AE2C0` creates `0x102` with proc `0x006AE3F0` on the standard offline path.

> **Ghidra availability note (this session):** no live Ghidra instance was running, so no fresh decompiles
> were taken. This report is a **synthesis** of already-extracted binary evidence in three HIGH-confidence
> skirmish-ui reports (cited inline). The load-bearing facts (the 72-child RT_DIALOG inventory; the
> control→Rules-field bindings; the absence of runtime-created option controls) were all binary-verified in
> those reports. Two confirmatory-only spot-checks are listed as DEFERRED in §8.

## 1. Overview

gamemd's offline Skirmish setup dialog `0x102` surfaces **exactly 8 game-option widgets**: five checkbox
`Button`s and three trackbars. Every other `[MultiplayerDialogSettings]` key is **stored-only** — read once
into `RulesClass` and consumed at match launch by gameplay systems, never shown as a `0x102` child control.
The five trackbar-bound keys (`MinMoney`/`MaxMoney`/`MoneyIncrement`/`MinUnitCount`/`MaxUnitCount`)
parameterize the *range* of the two numeric trackbars rather than being widgets themselves.

**O5 verdict: the Rust shell is MISSING ZERO option widgets.** Its 5 checkboxes + 3 trackbars are a 1:1
match for the binary's `0x102` option-widget set.

## 2. The 8 surfaced option widgets (binary-verified)

| Control id | Class | Resource title | INI key | Bind / note | In Rust shell? |
|---|---|---|---|---|---|
| `0x54E` | Button (checkbox `0x50000003`) | `GUI:ShortGame` | `ShortGame` | Rules `+0x14B6`; Start→`DAT_00A8B262` | YES |
| `0x69A` | Button | `GUI:SuperWeaponsAllowed` | `SuperWeaponsAllowed` | Rules `+0x14B9` (ctor-seeded `1`; key absent in stock INI) | YES |
| `0x69D` | Button | `GUI:BuildOffAlly` | `BuildOffAlly` | Rules `+0x14BA` (ctor-seeded `1`; key absent in stock INI) | YES |
| `0x693` | Button | `GUI:MCVRepacks` | `MCVRedeploys` | Rules `+0x14B8`; Start→`DAT_00A8B320` | YES |
| `0x696` | Button | `GUI:CratesAppear` | `Crates` | Rules `+0x14B1`; Start→`DAT_00A8B261` | YES |
| `0x529` | Trackbar | game speed | `GameSpeed` | **inverted**: `DAT_00A8B268 = 6 − TB_GETPOS(0x529)` | YES (inverted) |
| `0x511` | Trackbar | credits | `Money` | value `DAT_00A8B25C = TB_GETPOS(0x511)` | YES |
| `0x50C` | Trackbar | unit count | `UnitCount` | value `DAT_00A8B270 = TB_GETPOS(0x50C)` | YES |

Evidence: control inventory + classes + titles from
`SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` (rows 5,6,8,9,10,11,49,50); checkbox→Rules-field
bindings + init/packing from `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md` §2–§4; trackbar
bindings + GameSpeed inversion from `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md` §2.

## 3. Full 25-key (+2) DRIFT classification

Categories: **widget** (a `0x102` child control) · **trackbar-bound** (parameterizes a trackbar's
min/max/step, not its own control) · **stored-only** (read into Rules, consumed at launch) · **mode-derived**
(sourced from the selected game mode, not this global) · **unmodeled** (stored-only + drives a system this
engine does not implement).

| INI key | Stock value | Classification | 0x102 widget | Rust shell status | Missing widget? |
|---|---|---|---|---|---|
| MinMoney | 5000 | trackbar-bound | — (Credits min) | seeded `SkirmishTrackbarBounds` (`bc3ae055`) | no |
| Money | 10000 | **widget** | `0x511` Credits trackbar | present + seeded | **no** |
| MaxMoney | 10000 | trackbar-bound | — (Credits max) | seeded | no |
| MoneyIncrement | 100 | trackbar-bound | — (Credits step) | seeded | no |
| MinUnitCount | 0 | trackbar-bound | — (UnitCount min) | seeded | no |
| UnitCount | 10 | **widget** | `0x50C` UnitCount trackbar | present + seeded | **no** |
| MaxUnitCount | 10 | trackbar-bound | — (UnitCount max) | seeded | no |
| TechLevel | 10 | stored-only | none | stored in `GameOptions`/`launch_options_base` | no |
| GameSpeed | 1 | **widget** | `0x529` GameSpeed trackbar (inverted) | present (inverted) + seeded | **no** |
| AIDifficulty | 0 | stored-only | none (per-opponent AI-type combos are separate) | stored; overridden by opponent slots | no |
| AIPlayers | 0 | stored-only | none (AI count derives from opponent slots) | stored; overridden by opponent slots | no |
| BridgeDestruction | yes | stored-only | none (forced bridge flag `DAT_00A8B260` at Full_Init) | stored (`bridges_destroyable`) | no |
| ShadowGrow | no | **unmodeled** | none | not modeled (documented `game_options.rs:101-106`) | no |
| Shroud | yes | stored-only | none | stored (`shroud`) | no |
| Bases | yes | stored-only | none | stored (`bases`) | no |
| TiberiumGrows | yes | stored-only | none | stored (`tiberium_grows`) | no |
| Crates | yes | **widget** | `0x696` CratesAppear checkbox | present | **no** |
| CaptureTheFlag | no | **unmodeled** | none | not modeled (documented) | no |
| HarvesterTruce | no | stored-only | none | stored (`harvester_truce`) | no |
| MultiEngineer | no | stored-only (desupported, INI:3010) | none | stored (`multi_engineer`) | no |
| AlliesAllowed | no | **mode-derived** | none (gates team-combo enable; per-mode override) | mode-derived (`selected_mode_must_ally`) | no |
| ShortGame | yes | **widget** | `0x54E` ShortGame checkbox | present | **no** |
| FogOfWar | no | stored-only (TS-legacy; fog staging at launch) | none | stored (`fog_of_war`, default off) | no |
| MCVRedeploys | yes | **widget** | `0x693` MCVRepacks checkbox | present | **no** |
| AllyChangeAllowed | yes | stored-only | none | stored (`ally_change_allowed`) | no |
| SuperWeaponsAllowed | (absent; ctor `1`) | **widget** | `0x69A` checkbox | present | **no** |
| BuildOffAlly | (absent; ctor `1`) | **widget** | `0x69D` checkbox | present | **no** |

**Tally:** 8 widgets · 5 trackbar-bound · 11 stored-only · 1 mode-derived · 2 unmodeled = 27 entries (25 INI
keys + the 2 widget-only keys). **Missing-widget DRIFT list: EMPTY.**

## 4. Why the stored-only keys are NOT widgets (mechanism)

The `0x102` proc never `CreateWindow`s option controls at runtime: `FUN_006AE6E0` (init) and `FUN_006ACEE0`
(Start) operate on the *existing* RT_DIALOG template children via `GetDlgItem` + `BM_SETCHECK`/`BM_GETCHECK`
(checkbox-mapping report §3). The matrix report enumerates all 72 template children (OQ-8: none missing) and
the resize path `FUN_0060C0C0` walks every child window — a runtime-created option control would appear in
both, and none do. The stored-only keys reach gameplay through the Rules read path
(`ReadMultiplayerDialogSettings@0x00671EA0` → `ReadSkirmishSettings@0x00697F10`) and are consumed at launch
(e.g. `BridgeDestruction`→forced flag `DAT_00A8B260` at `Full_Init@0x00686B20`; `FogOfWar`→SpecialFlags
`0x1000` staging) — never surfaced as a setup widget. Evidence: packed-option report §2/§3.

## 5. Network/WOL lobby (bounded non-scope)

`0x102` is confirmed the **offline** Skirmish setup dialog (matrix OQ-1; `0x006AE317..0x006AE328` passes id
`0x102`, proc `0x006AE3F0`). The online/WOL/LAN multiplayer lobby is a distinct dialog surface that the Rust
shell does not target for this slice; whether it exposes additional option widgets is out of scope for the
offline-Skirmish parity the shell implements. Recorded as DEFERRED (§8).

## 6. Current Rust Implementation Status

- **No widget work required.** The shell already surfaces all 8 option widgets (`src/ui/skirmish_shell/state/
  trackbars.rs` checkboxes + trackbars; combos for the per-row controls).
- **Seed already shipped:** values via `GameOptions::from_multiplayer_dialog_settings` →
  `SkirmishLaunchOptions::from_game_options` → `SkirmishShellState.launch_options_base` →
  `launch_session` (commit `1f54995f`); trackbar bounds via `SkirmishTrackbarBounds::from_multiplayer_dialog_settings`
  (commit `bc3ae055`, the prior O4 fix). Both wired at the lobby construction site
  (`src/app_init_helpers.rs:340,315`; `src/app.rs:2406,2413`).
- **`ShadowGrow`/`CaptureTheFlag`** remain correctly unmodeled (documented at `src/sim/game_options.rs:101-106`).
- The master plan §4F O4 line ("trackbar bounds stay HARDCODED / DRIFT-UNCHECKED") is **STALE** — superseded by
  `bc3ae055`.

## 7. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| `0x102` complete child inventory (72) | verified | matrix report (RT_DIALOG `0x102`, offset `0x4FF1E4`) | none |
| 5 checkbox → Rules-field bindings | verified | checkbox-mapping §2/§4 | none |
| 3 trackbar → global bindings + GameSpeed inversion | verified | packed-option §2 | none |
| No runtime-created option controls | verified | checkbox-mapping §3 (GetDlgItem/BM_*); matrix OQ-8 | none |
| Stored-only keys consumed at launch | verified | packed-option §2/§3 | none |
| Rust seed chain (values + trackbar bounds) | verified | `1f54995f`, `bc3ae055`; `app_init_helpers.rs`, `app.rs` | none |
| Online/WOL lobby option widgets | deferred | — | separate dialog; not shell-targeted |
| Live re-decompile of `0x006AE3F0`/`0x006AE6E0` this session | deferred | Ghidra offline | confirmatory only |

## 8. Open Questions — Final State

- `[RESOLVED] O5-1 — Which [MultiplayerDialogSettings] keys are 0x102 child widgets? → Exactly 8: ShortGame(0x54E), SuperWeaponsAllowed(0x69A), BuildOffAlly(0x69D), MCVRedeploys(0x693), Crates(0x696), GameSpeed(0x529), Money(0x511), UnitCount(0x50C).` (evidence: matrix rows 5/6/8/9/10/11/49/50; checkbox-mapping §2)
- `[RESOLVED] O5-2 — Does the Rust shell miss any surfaced option widget? → No; all 8 are present.` (evidence: §3 table vs `src/ui/skirmish_shell/state/trackbars.rs`)
- `[RESOLVED] O5-3 — Are the ~11 GameOptions-only keys + ShadowGrow/CaptureTheFlag widgets? → No; all stored-only/unmodeled, consumed at launch.` (evidence: matrix has no such child among 72; packed-option §2/§3)
- `[RESOLVED] O5-4 — Are the 5 trackbar-bound keys widgets? → No; they parameterize the 2 trackbars' ranges.` (evidence: ini:3018-3024; packed-option §2; `bc3ae055`)
- `[RESOLVED] O5-5 — Is 0x102 the offline Skirmish dialog id? → Yes.` (evidence: matrix OQ-1, `0x006AE317..0x006AE328`)
- `[DEFERRED] O5-6 — Does the online/WOL lobby surface additional option widgets?` (category: out-of-scope; reason: separate dialog surface the Rust shell does not target this slice; next-step-if-pursued: locate the network-lobby dialog id + child inventory.)
- `[DEFERRED] O5-7 — Fresh live decompile of 0x006AE3F0/0x006AE6E0 this session.` (category: needs-runtime-debugger [Ghidra offline]; reason: confirmatory only — bindings already decompiled in checkbox-mapping report; next-step-if-pursued: re-run when a Ghidra instance is up and spot-check the proc creates no extra controls.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x102` surfaces exactly 8 option widgets (5 checkbox + 3 trackbar) | matrix; checkbox-mapping §2; packed-option §2 | none — all present | `src/ui/skirmish_shell/state/trackbars.rs` | none (parity already met) | Shell shows ShortGame/SuperWeapons/BuildOffAlly/MCVRedeploys/Crates + GameSpeed/Credits/UnitCount; no other option checkbox/trackbar | **Do NOT add** TechLevel/Shroud/Bases/etc. as new widgets — they are stored-only in gamemd; adding them would be a divergence |
| ~11 stored-only keys + ShadowGrow/CaptureTheFlag have no widget | matrix (no such child); packed-option §2/§3 | none — correctly stored, not surfaced | `src/sim/game_options.rs` | keep stored-only; keep ShadowGrow/CaptureTheFlag unmodeled | A mod editing TechLevel/Shroud/… changes the launched match (via seed chain) but adds no dialog control | Do not equate "stored" with "should be a widget" |
| 5 trackbar-bound keys parameterize the 2 trackbars, are not widgets | ini:3018-3024; packed-option §2 | none — seeded (`bc3ae055`) | `src/ui/skirmish_shell/state/trackbars.rs` `SkirmishTrackbarBounds` | keep bounds seeded from MinMoney/MaxMoney/MoneyIncrement/MinUnitCount/MaxUnitCount | A mod setting MaxMoney=50000 widens the Credits trackbar range | Do not surface bound keys as their own controls |

### Stale Docs / Follow-up
- `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` §4F O4 ("trackbar bounds stay HARDCODED + DRIFT/UNCHECKED")
  and the §2 C14 table "UNCHECKED (O4)/(O5)" rows are **superseded**: O4 implemented by `bc3ae055`; O5 resolved
  here (zero missing widgets). The ~11 stored-only keys are confirmed stored-only, not a missing-widget gap.

## Sources
- `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` (72-child RT_DIALOG inventory)
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md` (checkbox→Rules-field bindings, init/Start packing)
- `docs/research/skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md` (trackbar bindings, GameSpeed inversion, stored-only consumers)
- `ini/rulesmd.ini:3017-3042` ([MultiplayerDialogSettings])
- Rust: `src/sim/game_options.rs`, `src/ui/skirmish_shell/state/trackbars.rs`, `src/app_init_helpers.rs:315,340`, `src/app.rs:2406,2413`; commits `1f54995f`, `bc3ae055`
- Ghidra addresses (from cited reports, not re-decompiled this session): `0x006AE2C0`, `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x00671EA0`, `0x00697F10`, `0x00665650`
