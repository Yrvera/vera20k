---
title: Shell / Menu-UI Parity Roadmap
date: 2026-07-06
status: ROADMAP (not an approved implementation plan — every workstream still routes through /brainstorm → /design-review)
inputs:
  - docs/gap-scans/2026-07-06-disparity-scan-shell-ui.md  (89 confirmed gaps: H1..H19, M1..M39, L1..L34; 72 NV items; GH-1..GH-22)
  - docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md  (§6 service design, §7 retire list, §8 A-track slices + acceptance tests)
  - docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md  (Framework B, D-series-corrected)
  - docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/rust-baseline.md  (shipped-slice state @ dev 5bf8dfc4)
  - docs/plans/2026-05-31-shell-substrate-plan.md, 2026-06-01-shell-substrate-slice4-plan.md,
    2026-06-16-shell-substrate-slice5b-pump-swap-plan.md, 2026-06-10-ui-gadget-substrate-plan.md
---

# Shell / Menu-UI Parity Roadmap

## Governing process (READ FIRST — non-negotiable)

**This program follows "Rust-native structure, gamemd-native semantics," anchored on
`docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.** We do not port
gamemd's Win32 `HWND`/subclass/vtable plumbing or its C++ class tree; we reproduce the
*observable* behavior contract (the study's G-/O-/D-/S-series clauses) with idiomatic Rust.

**Every non-trivial workstream in this roadmap MUST route through `/brainstorm` →
`/design-review` against that study (and the D-series-corrected
`SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md` for Framework-B work) BEFORE any
implementation. Direct patches to shell behavior are forbidden.** The tiny-detail ledger
(exact frame indices, tick counts, colors, offsets, cue identities, alignment enums) is
what makes these surfaces indistinguishable; a design pass that does not carry it forward
will drift. `/design-review` must confirm each parity claim is grounded in a cited
research doc / INI / asset / source or is marked UNKNOWN — no invented equivalence.

**Only QUICK-WIN items** (single-constant or single-mapping fixes with an existing test
seam, listed in their own section) **may skip the `/brainstorm` step — and they still
require a named test** added or extended in the same change.

**Parity-certification rule.** A parity claim in any follow-on doc, commit message, or
status note **names its executable check** ("verified by `test_x` / fixture `y`") **or is
labeled UNVERIFIED.** Prose never upgrades a status. Rust-vs-prior-Rust goldens (state
hashes, replay fixtures, byte-identical emission asserts) are regression ratchets, not
parity evidence; only gamemd-derived references (Ghidra emulation vectors, live capture,
retail file bytes) certify. Where no such instrument exists yet for a surface (most shell
render/timing/cue behavior), the honest label is UNVERIFIED-pending-instrument.

**No status ledger lives in this doc.** Completion is machine-derived: `git log` for what
shipped, the named per-workstream tests for whether it is correct. Do not add a
hand-maintained "done/in-progress" table here or anywhere — it rots (CLAUDE.md memory
policy). Statuses come from `git log --grep` + the cited tests, period.

Track vocabulary used below: **A-track** = the `ui::gadget` Framework-A substrate slices
A0–A6 (study §8; A0–A6 already SHIPPED per rust-baseline §1). **B-track** = the `ui::shell`
Framework-B slices 0–6 (rust-baseline §1; Slices 0–6 SHIPPED). "New slice" = work with no
existing slice number, to be numbered by its `/write-plan`.

---

## Workstreams by engine service

One section per SVC-* service that carries confirmed findings. Findings are *referenced*,
not restated — read them in the scan by ID. Size = S (≤ ~1 file, days) / M (multi-file,
substrate touch) / L (new dialog/flow or cross-cutting). Each names the exact `/brainstorm`
topic and the study §8 acceptance-test clause pattern its `/design-review` must carry.

### WS-1 · SVC-PAINT — in-game Options 0xBBB captions & pressed frames

- **Scope (closes):** H2 (captionless 0xBBB buttons), H3 (checkbox labels), M2 (mode-2
  MNBTTN pressed frame), M3 (pressed +2px sink, GH-14), M20 (SIDEBTTN frame-2 unused),
  M22 (disabled Button color/overlay), L7 (id→bg table / mode-0), L20 (shell PAL 6→8-bit),
  L14 (first-paint order).
- **Track:** B-track extension of Slice 5a-ii/5a-iii (0xBBB emitter already SHIPPED) +
  Slice 3/4 paint seam.
- **Dependencies:** H2 caption CSF keys are an NV enabler (DROP30) — resolve before wiring
  captions; M22 target color is NV22/DROP82/DROP75. M2/M3/M20 fix targets are already
  proven by GH-14/DC4 (see QUICK-WINS for M3).
- **Size:** M.
- **REQUIRED PROCESS:** `/brainstorm 0xBBB owner-draw caption + pressed-frame paint parity`
  → `/design-review`, anchored on OPTIONS_0XBBB chrome report + study §5-D5 (owner-draw
  asset truth) / §2.7 record map.
- **Acceptance shape:** study §8-A-style headless emission test — byte-identical
  `SpriteInstance` + `ShellTextDraw` list assertion (the Slice 4A pattern) for each control
  kind, plus a per-frame-index assertion (pressed→frame1 for mode-2, frames 2/3/4 to the
  identical rect for type-1); clause pattern like G22/G12 draw-cadence tests.

### WS-2 · SVC-KEYNAV — paused-hotkey gate, Esc apply/persist, three-stage routing

- **Scope (closes):** H4 (gameplay hotkeys incl. Delete-sell fire behind 0xBBB), H7 (Esc
  closes 0xBBB skipping apply/persist), M4 (Esc lands on wrong screen from 0x102), L21
  (three-stage IsDialogMessageA→TranslateAcceleratorA→filter-hook + Tab traversal), L22
  (Enter on quit-confirm = dismiss, GH-1), L23 (Tab in name-edit blurs).
- **Track:** B-track; builds on Slice 2 DialogController `on_key` + D-B3 (exit-confirm Esc
  already through the stack).
- **Dependencies:** H7 Esc-mapping target is NV4 (0xBBB proc Esc result 1=persist vs
  discard); M4 is NV6; L22 fix direction settled by GH-1. Resolve NV4 before H7.
- **Size:** M.
- **REQUIRED PROCESS:** `/brainstorm shell keyboard routing: paused-hotkey suppression +
  Esc/Enter result mapping` → `/design-review`, anchored on study §5 D3 / B1 + O12 (offline
  pump runs net-service only, no main-loop keyboard dispatch).
- **Acceptance shape:** headless input-replay per study §8 (the A6 "scripted click/key
  session → identical state" pattern); O12-matrix test (modal open offline → no gameplay
  hotkey mutates sim); D3 three-stage routing unit test.

### WS-3 · SVC-PUMP — freeze-behind-modal + session-mode discriminator

- **Scope (closes):** H5 (sim advances behind save/load offline), M1 (blocker globals +
  hardcoded SessionMode), M8 (camera/radar/power/gadget recompose behind 0xBBB), M9 (gadget
  idle tick not pause-gated), M12 (no "any dialog open" query → sidebar strip-scroll must
  suppress, GH-4), L27 (`current_session_mode` hardcoded Skirmish).
- **Track:** B-track Slice 5 sub-step 3 + 5b (`service_tick` swap already SHIPPED, C2
  assertion present).
- **Dependencies:** M1/L27 are network-load-bearing only (deferred until a network session
  ships — see Deferred). M8/M9/M12 are offline-visible now (camera scrolls behind Options).
  M12 consumer proven by GH-4 (sidebar reader 0x006A6A00; GScreen-flip reader is a proven
  wgpu no-op — non-gap).
- **Size:** M.
- **REQUIRED PROCESS:** `/brainstorm modal-pump recomposition freeze (offline O12 matrix +
  S2 any-dialog-open query)` → `/design-review`, anchored on study §5 O12 / S2 +
  MODAL_PUMP_00623120 contract.
- **Acceptance shape:** study §8 O12-matrix test (offline modal open → gadget tick + render
  recompose + sim frame counter frozen, tooltips live); an `any_dialog_open()` query test
  that suppresses the sidebar strip-scroll (M12).

### WS-4 · SVC-SLIDE — slide-tick length, OUT/6B consumers, static reveal

- **Scope (closes):** M13 (every slide 1 tick too long, GH-5/GH-19), M14 (slide-OUT modeled
  but zero consumers, GH-3), M15 (0x6B slide-in never driven, GH-20), M16 (campaign/movies
  slides absent), M17 (right-panel statics show full text during slide), M18 (SDMPBTN/
  SDWRNTMP chrome ramps static/absent), L2 (Back/Exit own trailing slot, GH-19), L4 (save
  re-arm), L5 (kind-1 reveal no highlight gradient), L6 (frame-coupled cadence < 33 fps).
- **Track:** B-track Slice 6 (IN-only SHIPPED); extends `ui/shell/slide.rs` +
  `app_shell_transition.rs`.
- **Dependencies:** M13/L2 fix targets proven (GH-5/GH-19: N_A = owner-draw buttons only).
  M14 direction proven by GH-3; M15 by GH-20; slide frame-direction + 0x4ED/0x4EC pairing
  by GH-21 (docs currently inverted). M16 campaign/movies slide consumers depend on those
  flows existing (WS-9/WS-10). L4 depends on native save/load (WS-9).
- **Size:** M (M13/L2/M14/M15 are S-ish each; the reveal + chrome-ramp set is M).
- **REQUIRED PROCESS:** `/brainstorm shell slide OUT/6B consumers + N_A tick length +
  static-reveal chrome ramps` → `/design-review`, anchored on study §5-D2 include-set +
  SHELL_FIRST_PAINT / SKIRMISH_STATIC_REVEAL docs (carry GH-5/GH-19/GH-20/GH-21 corrections).
- **Acceptance shape:** slide tick-count unit test (14/12/11 not 15/13/12); slide-OUT
  consumer test on every allow-listed close; reveal-blank-until-0x4EE test; clause pattern
  = study §8 slide/O7 tests.

### WS-5 · SVC-TOOLTIP — in-game box composition + placement + coverage

- **Scope (closes):** H6 (box fill/outline/color, reclassified per GH-10), M11 (regions only
  0xE2 + sidebar; non-0xE2 shells get empty set), M28 (placement collapsed to one offset),
  M31 (ToolTips toggle unwired to `set_enabled`), M32 (padding/wrap), M33 (cameo text
  shape/power/CSF), M34 (tabs/scroll empty + power unregistered), M35 (cameo mis-anchor —
  folded in M28).
- **Track:** A-track A4 (shared tooltip service SHIPPED, 1000 ms); this is the composition +
  placement + registration-coverage layer on top.
- **Dependencies:** H6/M28/M32/M33/M34 fix targets are VERIFIED-LIVE (TOOLTIP_* docs). M31
  is a one-call wiring fix (see QUICK-WINS). NV15 (zero-delay cameo hover flicker) and NV17
  (paused immediate-show) are riders — resolve before the placement redesign.
- **Size:** M.
- **REQUIRED PROCESS:** `/brainstorm in-game tooltip box composition + placement + region
  coverage` → `/design-review`, anchored on study §5 S1 + TOOLTIP_MANAGER_SIDEBAR_OVERLAP /
  TOOLTIP_TEXT_SOURCE / TOOLTIP_GLYPH_RASTER.
- **Acceptance shape:** study §8-A4 S1 tests — inclusive-edge rect boundary test, box-metric
  golden (solid-black fill + 1-px outline + same-color text), placement byte-0/byte-1 target
  test, cameo text-format test (space→LF, cost+power CSF 0xC6E).

### WS-6 · SVC-TEXT — status-line align, title/version, glyph atlas, loading text

- **Scope (closes):** M23 (status line 0x695 h-center vs native left, GH-12), M24 (0x100
  hover status strip never shown), M25 (0x101 faces from STT not GUI keys), M27 (loading-row
  text fallback), L15 (glyph atlas 0x20..0x180 truncated), L16 (missing-glyph fallback
  wrong), L17 (mid-reveal no restart), L18 (title x=635 vs 638, GH-12), L19 (version numeric
  format, GH-11).
- **Track:** SVC-TEXT spans B-track (`render/shell_text.rs`, `bit_font.rs`) + per-shell text
  emitters; no single slice — several are QUICK-WINS (M23, L18, L19).
- **Dependencies:** M23/L18/L19 fix targets proven (GH-11/GH-12). M24 needs the 0x100 STT
  table + tooltip-driver SP branch. L15/L16 are a glyph-atlas rebuild (touches font
  raster). NV18/NV19/NV21 are alignment/reveal riders.
- **Size:** M for the atlas + loading-text; the align/title/version items are S (QUICK-WINS).
- **REQUIRED PROCESS (for the non-quick-win parts):** `/brainstorm shell glyph atlas
  coverage + missing-glyph fallback + loading-screen text layer` → `/design-review`,
  anchored on BITFONT doc + LOAD_PROGRESS_MANAGER / LOADING_FIRST_RENDERER.
- **Acceptance shape:** BITFONT byte-golden per glyph range; loading-text fallback content
  test; clause pattern = study §8 text-emission tests.

### WS-7 · SVC-CURSOR — hide OS cursor process-wide over dialogs

- **Scope (closes):** M5 (OS arrow shown over 0xBBB / F5 panel / quit-to-menu → double or
  wrong cursor), L24 (SHP cursor occluded under egui placeholder dialogs).
- **Track:** New (no dedicated slice today).
- **Dependencies:** L24 resolves as the egui placeholders (WS-8/WS-9/WS-10) become native;
  M5 is the standalone `ShowCursor(0)`-equivalent + software-cursor gate fix.
- **Size:** S.
- **REQUIRED PROCESS:** `/brainstorm process-wide OS-cursor suppression + software-cursor
  gate over dialogs` → `/design-review`, anchored on MAIN_MENU_CURSOR_SHP_AND_RULES
  (ShowCursor(0) verified-from-binary) + O10 draw-last.
- **Acceptance shape:** cursor-visibility state test across pause / F5 / quit-to-menu
  transitions (OS cursor never visible while a dialog/overlay is up); SHP blitted last.

### WS-8 · SVC-FLOW/SVC-DIALOG — skirmish lobby session + AI-row model

- **Scope (closes):** H16 ([Skirmish] INI session persistence), H17 (AI rows hide/reveal
  with start-slot count + force −1), H18 (auto start-position assignment metric), M7
  (per-slot AI difficulty flattened), M10 (team combos not disabled when AlliesAllowed=false),
  M35 (same-team → error modal vs silent auto-repair), M36 (Random Colour sentinel), M37
  (0x583 Create-Random-Map dialog + dead stub), L32 (Gate-4 acceptance vtable), L33 (country
  combo hardcoded), L34 (team "None" label hardcoded).
- **Track:** New (SURFACE-heavy) built on the shipped 0x102 board; several rows also touch
  B-track Slice 4 control substrate.
- **Dependencies:** H18 native algorithm is NV7 (resolve first), M7 encoding is NV8 (may
  invert Easy↔Hard — resolve first). M35/M36/M10/L32 fix targets are VERIFIED-LIVE (_parity
  gate decompiles). M37 0x583 setup is DOC-INHERITED structural absence.
- **Size:** L.
- **REQUIRED PROCESS:** `/brainstorm skirmish lobby session persistence + AI-row visibility +
  gate validation` → `/design-review`, anchored on the study (Framework-B §3.5 census) + the
  LOBBY / _parity gate reports. **Do not implement H18/M7 until NV7/NV8 are resolved.**
- **Acceptance shape:** [Skirmish] round-trip test (read → mutate → write → re-read exact);
  AI-row visibility test per map start-slot count; gate-validation tests (silent auto-repair,
  team-disable, Gate-4 reject) mirroring the _parity gate verdicts.

### WS-9 · SVC-FLOW/SVC-DIALOG — native Load/Save/Delete family

- **Scope (closes):** H8 (native 0xB7/0x2B4/0x2B5 + runner 0x00558DD0, DWL_USER −1 channel,
  front-end anim, save-success confirm + slide re-arm, `SAVE_%04lX.%3s` naming — replaces
  the egui panel + invented F5/M/N bindings), M6 (descriptor coverage for 0xB7/0x2B4/0x2B5),
  L4 (save re-arm — with WS-4), L26 (Load-enable attribute filter).
- **Track:** New B-track dialog family (Framework B), consuming Slice 1/2/3 descriptor +
  controller + paint seams.
- **Dependencies:** unblocks H5 (WS-3) fully (sim freeze behind the native surface) and L4
  (WS-4). Save-sound contract is NV40. L26 exact scan parity is engine-native-saves-bounded.
- **Size:** L.
- **REQUIRED PROCESS:** `/brainstorm native Load/Save/Delete dialog family (0xB7/0x2B4/0x2B5)`
  → `/design-review`, anchored on dialog-delta §2.5/§7 + study §5-D (record map, DWL_USER,
  three-stage keyboard).
- **Acceptance shape:** descriptor layout golden-rect test per dialog; result-channel routing
  test (Main_Game case-9 map); modal-pump freeze test (subsumes H5).

### WS-10 · SVC-FLOW — main-menu Options 0xD5, campaign, movies/credits

- **Scope (closes):** H9 (main-menu Options 0xD5 launcher), H10 (campaign select 0x94),
  H11 (Movies & Credits 0x101 native replace-in-place), H12 (movie picker 0x129), H13 (Sneak
  Preview RENEGADE.BIK), H14 (credits roll), M16 (campaign/movies slides — with WS-4), M21
  ([Movies] parse), M25 (0x101 faces — with WS-6), M26 (Bink 16-bit quantization), M29
  (in-game abort-mission confirm), M30 (blocking movie wrapper), L8 (EXPANDMD family scan,
  GH-2 — see QUICK-WINS), L9 (FinalMovie chain), L10 (VQA fallback + BIK→VQA resolver).
- **Track:** New; splits into 0xD5-launcher (H9), campaign (H10, deferred — not active build
  focus), and movies/credits playback (H11–H14 + M21/M26/M30).
- **Dependencies:** H9 checkbox/slider→field map is NV50/NV51 (resolve first — the case5
  table wires two checkboxes wrong). Campaign (H10, M29, L9) is blocked on the campaign
  system (Deferred). Movies need the blocking-playback + [Movies] parse (M21/M30). M29 abort
  dialog is NV56 (locate first; do NOT clone 0x120).
- **Size:** L (each sub-flow is its own `/write-plan`).
- **REQUIRED PROCESS:** one `/brainstorm` per sub-flow —
  `/brainstorm main-menu Options launcher dialog 0xD5`,
  `/brainstorm movies & credits playback flow (0x101/0x129 + blocking wrapper)`,
  (campaign deferred) — each → `/design-review`, anchored on OPTIONS_DIALOG_CASE5 /
  MOVIES_CREDITS_DIALOG_PLAYBACK + Framework-B §3.5. **Resolve NV50/NV51 before H9, NV56
  before M29.**
- **Acceptance shape:** 0xD5 checkbox/slider→field wiring test (correct id map); movie
  resolver order test (BIK→VQA); credits scroller content test; blocking-loop gate test.

### WS-11 · SURFACE/SVC-FLOW — loading-screen composition

- **Scope (closes):** H15 (mmpb player start markers), M27 (loading-row text — with WS-6),
  M38 (16-shade ColorScheme bar remap), M39 (milestone ramp gaps 13..25 / 90), L1 (binary_
  frame 15 Hz vs ~63 Hz — after per-consumer trace), L14 (first-paint order — with WS-1).
- **Track:** New (SURFACE); `app_loading.rs` + loading chrome.
- **Dependencies:** H15 marker scale/offset registers are UNRESOLVED (U4) — resolve before
  the marker layer. M38 ramp gen (FUN_0068c3b0) undecoded. L1 needs a per-consumer trace
  before its severity/fix can be pinned (owned by the sim-pacing system, not shell).
- **Size:** M.
- **REQUIRED PROCESS:** `/brainstorm skirmish loading-screen marker + 16-shade bar + milestone
  ramp composition` → `/design-review`, anchored on LOADING_SCREEN_MARKERS_BAR_HANDOFF /
  LOADING_FIRST_RENDERER / PROGBARM_PROGRESSCLASS.
- **Acceptance shape:** marker-placement + ColorScheme-remap golden per active slot; milestone
  emission sequence test (no 12→30 / 86→93 jumps).

### WS-12 · SVC-SOUND — modal/0xBBB cue coverage + audio config

- **Scope (closes):** M19 (0x6B slide-in start cue — with WS-4, GH-22), L11/L13 (paint-
  transition GenericClick on 0x100 / choose-map), L12 (PENGO ack), L29 (SoundVolume
  hardcoded), L30 (7 [AudioVisual] keys unparsed), L31 (empty-value clear vs preserve).
- **Track:** Per-slice sound plumbing (largely SHIPPED for served surfaces); this is the
  residual-cue + config layer.
- **Dependencies:** 0xBBB/modal cue existence is heavily NV (NV28/NV29/NV30/NV40 — whether
  gamemd's subclass pass plays the cue at all). Resolve those before adding cues; do not
  fabricate a cue the binary does not play. L30 consumers unlocated (NV33).
- **Size:** S–M.
- **REQUIRED PROCESS:** `/brainstorm shell/modal sound-cue coverage + [Audio] config`
  → `/design-review`, anchored on SHELL_UI_SOUND_PLAYBACK_PLUMBING / GLOBAL_SOUNDS +
  study §5 S-series. **Resolve NV28–NV30 before wiring 0xBBB/modal cues.**
- **Acceptance shape:** per-cue emission-point test gated on the resolved binary answer;
  [Audio] SoundVolume read-path test; empty-value preserve test.

### WS-13 · SVC-ASSET — mixfile / palette / movie decode

- **Scope (closes):** L8 (EXPANDMD family scan, GH-2 — QUICK-WIN), M26 (Bink quantization —
  with WS-10), M21 ([Movies] parse — with WS-10), L10 (VQA resolver — with WS-10), L20 (shell
  PAL 6→8-bit — with WS-1).
- **Track:** Pre-substrate assets layer; mostly folds into WS-1/WS-10.
- **Dependencies:** palette-startup order is NV (unchecked). Largely a consumer of the movie
  work in WS-10.
- **Size:** S (L8/L20) + folds into WS-10 (M21/M26/L10).
- **REQUIRED PROCESS:** L8/L20 are QUICK-WINS (below). M21/M26/L10 go through WS-10's
  `/brainstorm`. No separate brainstorm for the quick-wins.
- **Acceptance shape:** EXPANDMD descending-scan mount-order test; PAL `<<2` expansion
  byte-golden.

---

## Quick wins (trivial, existing test seam — MAY skip /brainstorm; still need a named test)

Each is a single-constant / single-mapping / single-call fix whose gamemd target is already
proven by the Ghidra pass (GH-*) or is a pure wiring gap. Cite the finding ID; add/extend the
named test in the same change.

- **QW-1 · M13 + L2** — slide tick length: `total_ticks_for` folds Back/Exit into `slot_count`;
  N_A counts owner-draw buttons only (5/3/2, GH-5/GH-19). Fix the operand; extend `slide.rs`
  tick-count test (14/12/11). Seam: `src/ui/shell/slide.rs` (8 tests).
- **QW-2 · M3** — pressed button sinks art +2px Y and label +2px Y; native blits frames 2/3/4
  to the identical rect, only nudges text (+1x, top −2→−1, GH-14). Seam:
  `src/app_main_menu_shell_render.rs` + `render/shell_paint.rs` (17 tests).
- **QW-3 · M23** — status line 0x695 is LEFT-aligned, not h-centered (GH-12, align enum 0x10).
  Seam: `src/app_main_menu_shell_render.rs`.
- **QW-4 · L18** — title 0x694 x = 638, not 635 (drop the version-line inset, GH-12). Seam:
  `src/ui/main_menu_shell/layout.rs`.
- **QW-5 · L19** — version label = `GUI:Version` + numeric `%d.%3.3dTUC`; never concatenate raw
  VERSION.TXT (GH-11). Seam: `src/app.rs` version build + `app_main_menu_shell_render.rs`.
- **QW-6 · M31** — `apply_in_game_options` never calls `TooltipService::set_enabled` (the flag
  exists; stale comment). One call + a wiring test. Seam: `src/app_options_persist.rs`.
- **QW-7 · L8** — expansion scan hardcodes `expandmd01.mix`; GH-2 proved EXPANDMD99→00
  descending family scan (order load-bearing for override priority). Seam:
  `src/assets/asset_manager.rs`. (Add a mount-order test.)
- **QW-8 · M2** — mode-2 modal pressed frame → frame 1 (frame 2 = timer/highlight; type-3
  disabled selects no frame), per state-frames report / DC4. Seam: `src/render/shell_paint.rs`.
- **QW-9 · L20** — shell chrome PAL 6→8-bit must use `<<2` (63→252); `from_bytes_gamemd_ui`
  exists but no chrome loader calls it. Swap the loader call + a byte-golden test. Seam:
  `src/render/{main_menu,skirmish}_shell_chrome.rs`.

Borderline (verify the target is genuinely a one-liner in `/design-review` before treating as
QW): **L34** (team "None" → CSF GUI:NoneAsSymbols 0x45F) and **L16** (missing-glyph fallback)
— L16 touches the font raster, so route it through WS-6, not here.

---

## Research-first queue (resolve the named binary question BEFORE implementing the surface)

Every open NV rider that gates a workstream, mapped to its tool + doc/binary anchor. Do the
research first; a doc-conflicted reading cascades into wrong behavior. (NV items already
answered by GH-1..GH-22 survive only as doc-patch obligations — handled by `/audit`, below.)

| NV | Question | Target | Anchor |
|---|---|---|---|
| NV4 | 0xBBB proc Esc → close, result 1=persist vs discard? (H7) | `/re-investigate` | proc 0x00622B50 (OPTIONS_PROC §2/§12) |
| NV6 | 0x102 global accelerator VK_ESCAPE→IDCANCEL? (M4) | `/re-investigate` | FUN_006AE3F0 / dialog-manager |
| NV7 | Start-point auto-assign farthest-available vs first-unused? (H18) | `/re-investigate` | ScenarioClass::AssignStartingPoints 0x005EE9D0 |
| NV8 | Per-slot AI difficulty encoding — item-data vs ordinal (Easy↔Hard invert)? (M7) | `/re-investigate` | packed global 0xA8B27C + SetDifficulty consumer |
| NV50/NV51 | 0xD5 checkbox/slider→field map (0x601/0x602/0x529/0x52B)? (H9) | `/re-investigate` | OPTIONS_DIALOG_CASE5 vs OPTIONS_PROC / UNITACTIONLINES |
| NV56 | Locate in-game Abort Mission confirm (id, controls, shutdown)? (M29) | `/re-investigate` | RT_DIALOG 0xCF candidate (do NOT clone 0x120) |
| NV54 | 0x120 quit-confirm template — 3-control y=155 vs 4-control y=135? | `/decode-system` (rsrc walk) | RT_DIALOG dir; VALIDATION_MODAL vs RT_DIALOG_0X120 |
| NV28/NV29/NV30 | Do 0xBBB / quit-confirm / validation subclass passes play a cue? (WS-12) | `/re-investigate` | FUN_0060F9A0 child-subclass → OwnerDraw_* |
| NV40 | Load/Save/Delete button-sound + load-success cue contract (WS-9) | `/re-investigate` | 0x525/0x527/0x528 (no sound doc exists) |
| NV1 | Loading side-icon resolver + PCX set (H15/WS-11) | `/re-investigate` | ProgressClass +0x80; FUN_004e3560 role |
| NV15/NV17 | Cameo zero-delay hover flicker; paused immediate-show (WS-5) | `/re-investigate` | 0x0072429E / 0x00724247 (DAT_00A8F7D8) |
| NV22 | Disabled Button label color #480000 vs yellow-kept (M22) | `/re-investigate` | SHELL_BUTTON_PAINT_DETAILS §1 vs SKIRMISH_OWNERDRAW color |
| L1 (per-consumer) | binary_frame 15 Hz 4×-slow consumer set (WS-11) | `/trace-action` | LIVE_SKIRMISH_PACING §3-§5 (owned by sim-pacing) |

**Doc-patch obligations (not binary questions — GH-1..GH-22 already resolved; run `/audit`
per CLAUDE.md auto-patch rule):** the ~24 doc errors in the scan's "Doc errors discovered"
section (SHELL_TRANSITION +0xC1 setter, C11 N+6/N+8, C9 hover-flash, D-B2 tooltip conflation,
TITLE_TEXT version/v-center, pressed +2px, 0x55F/0x71B census, slide direction/pairing,
MNBTTN pressed frame, etc.). These block nothing but rot if left — patch with the inline
Ghidra-call citation. Also apply the study's §5-D patch obligations to
SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md (D1 record map, D2 55+19 include-set, D3
three-stage keyboard, D4 DWL_USER(8), D5 owner-draw assets).

---

## Suggested order (dependency-aware)

**Do-first shortlist (≤5, ranked by severity × frequency; all cheap and unblocked):**

1. **QW-1 (M13/L2)** — every shell first-paint is 1 tick too long; proven fix, one operand,
   existing test. Highest frequency (every shell entry), lowest cost.
2. **WS-2 · H4** — gameplay hotkeys (incl. Delete-sell mutating the sim) fire behind the
   0xBBB pause overlay; fires every keypress while paused, corrupts state. Gate keyboard on
   `paused` (mouse already gated). (Full three-stage routing L21 is later; the gate is the
   urgent slice.)
3. **WS-5 · H6** — in-game tooltip box (wrong fill/outline/color) shows on every hover ≥1s,
   every match; VERIFIED-LIVE fix target.
4. **QW-2 + QW-3 (M3, M23)** — pressed-button sink + status-line align; every main-menu
   click/hover; both proven by GH-14/GH-12, both quick-wins.
5. **WS-3 · M8/M9/M12** — recomposition behind the offline modal (camera visibly scrolls
   when the cursor rests at a screen edge while Options is open); offline-visible now, and
   M12 (sidebar strip-scroll suppression) is proven by GH-4.

**Then, dependency-aware:**

- **Wave A (cheap, every-frame correctness):** remaining QUICK-WINS (QW-4/5/6/7/8/9), then
  WS-6 text align/title/version cluster. No cross-deps.
- **Wave B (every-pause 0xBBB):** WS-1 (0xBBB captions H2/H3 — resolve DROP30 caption keys
  first), WS-2 (H7 — resolve NV4 first), WS-7 (M5 cursor). WS-3 network pieces (M1/L27)
  stay deferred.
- **Wave C (every shell transition):** WS-4 slide consumers (M14/M15/M17/M18 + M19 cue) —
  all fix targets proven by GH-3/GH-19/GH-20/GH-21/GH-22.
- **Wave D (every lobby open):** WS-8 — but resolve NV7 (H18) and NV8 (M7) FIRST; M10/M35/
  M36/M37/L32/L33/L34 are independent and can land alongside.
- **Wave E (every skirmish load):** WS-11 loading composition (H15 needs U4; L1 needs the
  per-consumer trace — defer L1 to sim-pacing).
- **Wave F (missing native flows, own designs):** WS-9 (Load/Save/Delete — unblocks H5/L4),
  then WS-10 0xD5 launcher (resolve NV50/NV51 first). Campaign (H10/M29/L9) and full
  movies/credits playback (H11–H14/M21/M26/M30) are **Deferred** — campaign is not the
  active build focus (scan "Deferred / blocked"), movies wait on the blocking-playback
  wrapper.

**Deferred (blocked, not scheduled):** campaign flow (H10, M29, L9, NV45/NV46/NV56);
network/lockstep pump pieces (M1, L27); binary_frame 15 Hz severity (L1, needs per-consumer
trace). See the scan's "Deferred / blocked by other systems" section.

**TS-legacy / WOL — NOT gaps (do not schedule):** Framework-A TS shell-control wing, dropship
loadout, WOL/ladder + LAN/IPX online wing, modem/serial, `bud_*` disabled art, FogOfWar
darkening. Their absence is CORRECT (scan "TS-legacy / WOL filtered out").
