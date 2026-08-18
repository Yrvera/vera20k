# UI Gadget Substrate — Implementation Plan (A2 cameo strip, A3 tactical/minimap regions)

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. DOC-ONLY artifact:
> the code blocks here are the target source; do not assume they are already in `src/`.
>
> **EXECUTION TARGET:** the worktree `<local>/Documents/ra2-uigadget-a2a3-worktree`
> (branch `ui-gadget-substrate-a2a3` @ baseline commit `801bc09e` = dev HEAD with A0/A1/A4/A5
> merged). ALL file paths below are relative to that worktree; ALL verify commands run **in the
> worktree**, e.g. `cd <local>/Documents/ra2-uigadget-a2a3-worktree && cargo test -p vera20k <filter>`.
> Do NOT edit the main repo checkout. All commits land on branch `ui-gadget-substrate-a2a3`.
> The worktree's baseline `cargo check` is already green (cache warm).

**Goal:** Move the two remaining player-visible in-game click surfaces onto the shipped
Framework-A gadget substrate (`ui::gadget` + `app_gadget_input`), completing study slices **A2**
and **A3** of `GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §8:

- **A2 — Cameo strip.** The build/SW cameos become `SelectClass`-equivalent gadgets on the
  retained list (one ordered list = hit priority = draw order, O7/G20), the visible subset
  syncs per tab/scroll, and the cameo hover hook (Mouse_Enter/Leave, G7) drives the A4 tooltip
  zero-delay override. **Cameos keep firing on mouse-DOWN** (VERIFIED-LIVE: plain `ControlClass`,
  not the `ToggleClass` release machine) — A2 relocates the surface, it does **not** change the
  fire edge.
- **A3 — Invisible click regions.** The full-tactical catcher (mask `0x7F`, sticky) and the
  minimap/radar region (mask `0x9F`, sticky) become `ClickRegion` gadgets on the same retained
  list, so tactical/minimap clicks resolve through the gadget walk with sticky capture and the
  open-area click is the natural fallback. The existing selection/command/minimap handlers are
  **routed into**, never duplicated.

**Architecture:** No new modules. A2/A3 add two `GadgetBehavior` variants (`Cameo`, `ClickRegion`)
to `ui::gadget`, extend the existing `app_gadget_input` driver (cameo pool + two region gadgets +
result routing), and restructure `app_input::handle_mouse_input` to dispatch on a
**consumption-class** the driver returns. The gadget tick stays deterministic and wall-clock-free.
`sim/` is untouched — cameo/tactical/minimap clicks keep flowing through the existing
`app_commands` / `pending_commands` seam (study O14). The A4 tooltip service (already registering
cameo tips and exposing `set_delay_override`) is reused, not changed.

**Design Doc:** `docs/research/GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
(§1.A7/A8 responsibilities, §2.2/§2.5 family + population, §5 G-contract, §6 boundary, §8 slices)
+ the four A2/A3 grounding worknotes in
`docs/research/substrate/worknotes/gadget-dialog-20260610/`:
`plan-grounding-a2-cameo.md`, `plan-grounding-a3-tactical.md`, `plan-grounding-a3-minimap.md`,
`plan-grounding-a2a3-rust.md` (all written this session; every load-bearing fact carries an
inline Ghidra-call citation or a file:line cite at HEAD 801bc09e).

---

## Grounding Summary

- **A2 cameo contract (VERIFIED-LIVE, cameo lane):** SelectClass::Action `0x006AAD00` fires on
  the **press edge** — acts on LEFTPRESS (`0x1`) / RIGHTPRESS (`0x10`), **strips LEFTUP** (`0x8`),
  has **no** release branch. Plain `ControlClass` (ctor `0x006A4DC0`): **IsSticky=0**, mask
  **`0x19`** (LEFTPRESS|LEFTUP|RIGHTPRESS), W=60 H=48, ctor ID=0 → runtime ID **`1000 + visible
  index`**. The Action drives production/placement **directly in-place** and the tail-chained
  `ID|0x8000` is observationally inert (no consumer in `0x006A7780`'s set). Draw_Me is NOT
  overridden (the strip painter draws cameos; the gadget is a pure click rect + hover hook).
  Mouse_Enter `0x006AB990` saves the tooltip delay and forces it to **0**; Mouse_Leave
  `0x006AB9E0` restores it — and SelectClass is the ONLY live family class overriding hover.
- **A2 registration/tab-swap (VERIFIED-LIVE):** visible count = `rows × 2`,
  `rows = ((sidebarH − topAdj − stripTopY − 7 + sidebarW) / 0x32)` (÷50 px rows, 2 columns),
  `topAdj = 18` observer/special else `26`. Tab switch = **remove all 60 of the old tab's slots,
  then add only `rows×2` of the new tab's**, tab field committed between. (Our adaptive
  `SidebarView.items` already computes the post-scroll/post-tab visible set — the Rust driver
  mirrors `view.items`, reproducing the observable result without the 240-slot static array.)
- **A3 tactical catcher (VERIFIED-LIVE, byte-decoded):** init `0x004A86E0` → invisible,
  **sticky=1**, mask **`0x7F`** (all mouse except RIGHTUP `0x80` + KEYBOARD), rect = the live
  tactical viewport (`Set_View_Dimensions` writes `g_RadarViewportOffsetX/Y/Width/Height`),
  registered via `Add_A_Button` **only when `!g_IsMapEditor`**, **no result ID** (acts directly
  on Display). Action `0x004AAC10` → base `GadgetClass::Action 0x004E1530` → Sticky_Process
  (press `0x11` acquires, release `0x44` releases) → returns 1 (consumed, walk stops). Coordinate
  split: press/release use latched event coords, held/up use live mouse.
- **A3 minimap region (VERIFIED-LIVE):** init `0x00652870` → plain GadgetClass (no ID),
  **sticky=1**, mask **`0x9F`** (left press/held/release/up + right-press + right-up; **NOT**
  right-held `0x20`, **NOT** right-release `0x40`), rect `(0,0,0,0)` at ctor but Action
  self-tests against the radar-surface rect. Action `0x006539D0`: **ignores held** (`&0x22`
  early-out), gated on radar-active; **press edge (left or right) centers the camera** on the
  clicked cell; left-release commits the active command at the cell; left-up/right-up update the
  hover cursor. No result ID, no continuous follow-on-drag.
- **A3 catcher-vs-seed (resolved):** dispatch uses per-gadget `rect.contains` in **list order**
  (`tick.rs::clicked_on`), first-consumer-wins; the smallest-area `HIT_SEED_AREA = 786,432 px²`
  seed only gates **hover** (`hit_test`). The invisible catcher has no hover, so a viewport larger
  than the seed at high resolution does **not** break tactical click dispatch — the catcher
  dispatches via `rect.contains` at any resolution. (Rust-current lane open-Q #5 resolved.)
- **Rust anchors (rust lane, HEAD 801bc09e):** in-game list today owns only the 8 chrome buttons;
  cameos fire on mouse-DOWN via `sidebar::hit_test` (`mod.rs:405-438`, cameo loop `:410-414`) →
  `hit_test_item` (`:357-399`) → `apply_sidebar_action` (`app_input.rs:247-325`), all routing
  through `app_commands`/`targeting_mode` (no sim dep). Tactical/minimap entry is the inline
  `handle_mouse_input` body (`app_input.rs:51-168`): `try_begin_minimap_drag` (`:54`),
  `selection_state` begin/end_drag, `try_queue_context_order_at_screen_point`, SW launch,
  building place, right-cancel; minimap drag via `minimap_dragging` flag + `handle_cursor_moved_in_game`
  (`:179-213`). The driver already short-circuits on a `bool` consume (`:43-45`).
- **A4 reuse (already shipped):** `app_tooltips::sync_in_game_regions` already registers cameo
  tips (`CAMEO_TIP_ID_BASE = 1000`, name+cost) and `ui::tooltips::TooltipService::set_delay_override`
  exists (`tooltips.rs:195`). A2 only needs to **call** `set_delay_override(Some(0))` on cameo
  hover-enter and `None` on hover-leave.

---

## Key Technical Decisions

- **Cameos are a NEW `GadgetBehavior::Cameo`, not reused `Control`.** `Control` (`base_action`)
  would consume + post on the masked LEFTUP bit (mask `0x19` includes `0x8`), and would only mark
  right via RIGHTRELEASE — both wrong for cameos. `cameo_action` strips LEFTUP (no consume on a
  LEFTUP-only idle dispatch), fires on LEFTPRESS/RIGHTPRESS, and marks right via the RIGHTPRESS
  bit → `RESULT_RIGHT`. Faithfully models the gamemd primitive. — **Confidence: high.** Source:
  cameo lane §1a/§7; binary `0x006AAD00`.
- **The driver mirrors `SidebarView.items` for the cameo set, not a 240-slot static array.** Our
  adaptive layout already yields the visible, scrolled, per-tab cameo list; one cameo gadget per
  `view.items[i]` with `id = 1000 + i` reproduces gamemd's observable "registration order = hit +
  draw order, rows×2 visible, tab-swap remove/add" without porting the static block structure
  (Rust-native structure, gamemd-native semantics). — **Confidence: high.** Source: cameo lane §5;
  rust lane §2.
- **Cameo pool: grow-on-demand, disable-unused, never auto-shrink; stable handles.** Disabled
  gadgets are skipped by both `hit_test` and the broadcast walk (`tick.rs:83`, `:245`). A fixed
  growing pool avoids per-frame add/remove churn (which would otherwise risk clearing focus/hover
  slots mid-interaction). Cameos are never sticky so they never hold capture; growing the pool
  appends after the region gadgets, which is observationally irrelevant (see next). —
  **Confidence: high.** Source: rust lane §2.3.
- **Retained order: chrome (8) → cameo pool → minimap region → tactical region.** All four groups
  occupy **disjoint screen rects** (chrome in the upper sidebar, cameos in the side2 strip,
  minimap in the radar panel, tactical in the play area left of the sidebar — half-open rects,
  no shared pixel). So broadcast-walk first-consumer and smallest-area hover are unaffected by the
  relative order; the catcher being the "fallback" holds because its viewport rect is disjoint
  from every sidebar gadget. The substrate's ordering guarantee (smaller/earlier wins on overlap)
  is unit-tested on a synthetic overlapping list, not on the real disjoint one. — **Confidence:
  high.** Source: tactical lane §6.2; minimap lane §4.3.
- **A3 `ClickRegion` = `base_action` + sticky + invisible + no ID.** Behaviorally identical to a
  sticky `Plain` gadget; added as its own enum variant for self-documenting intent and to match
  the study's named 4-behavior family (§6.1). The driver identifies which region consumed by
  **stored handle** (`out.consumed_by` for the broadcast tier, the pre-tick sticky holder for the
  sticky tier), not by a fake posted ID (the regions post nothing in gamemd). — **Confidence:
  high.** Source: tactical lane §7; minimap lane §2e.
- **`handle_mouse_button_event` returns a `GadgetConsume` class (A3), replacing the `bool`.**
  `Consumed` (chrome/cameo) → caller returns; `Tactical` → caller runs the tactical body for this
  edge; `Minimap` → caller runs the minimap body; `NotConsumed` → legacy (dev/pause/producer on
  press). This keeps the tactical/minimap selection/command/drag code in `app_input` (called from
  the matched branch) instead of duplicating it in the driver. — **Confidence: medium — flagged
  for /review-plan** (the master mouse handler restructure is the one risky edit). Source: rust
  lane §5.3.
- **Tactical region rect = play area left of the sidebar; sourced from `view.panel_rect.x`.**
  gamemd uses `g_RadarViewportOffset*`; the Rust equivalent is the render area minus the sidebar
  panel. Re-synced each frame. Larger than `HIT_SEED_AREA` at high res — harmless (dispatch uses
  `rect.contains`, only hover uses the seed; catcher has no hover). — **Confidence: high.**
  Source: tactical lane §4.2/§6.2.
- **Minimap region rect = the live minimap screen rect (`active_minimap_screen_rect`), gadget
  DISABLED when radar offline / minimap absent.** Same rect source the existing minimap path uses,
  so routing matches today's hit region. — **Confidence: high.** Source: rust lane §4; minimap
  lane §2b.
- **Minimap drag SEMANTICS are PRESERVED as-is (continuous camera-follow), NOT flipped to
  gamemd's press-edge-only jump.** The minimap lane VERIFIED-LIVE that gamemd re-centers on press
  edges only (LEFTHELD is dropped) — the current Rust continuous-follow is a DRIFT. A3's scope is
  **input-path unification** (route the entry through the gadget walk + sticky capture); changing
  the drag feel is a separate parity decision the user must sign off (it may match RA2 muscle
  memory). A3 routes the minimap press into the existing `try_begin_minimap_drag` and leaves the
  `minimap_dragging` + `handle_cursor_moved_in_game` gesture intact. — **Confidence: high (as a
  scope boundary); the drift is logged as a Deferred item.** Source: minimap lane §2d/§4.2 + this
  plan's Open Questions.
- **`i32` pixel coords at the gadget boundary; cameo/region rects rounded once at the driver.**
  Matches the existing chrome-button sync (`rect_px`, `app_gadget_input.rs:76-83`) and the native
  integer hit math. — **Confidence: high.** Source: study G14.

---

## Open Questions

### Resolved During Planning (by the grounding lanes)

- *Do cameos fire on press or release?* — **PRESS.** Plain Control, mask `0x19`, LEFTUP stripped,
  no release branch (cameo lane §1a). Matches today's Rust behavior; A2 preserves the fire edge.
- *Are cameos a toggle/latch (is_on)?* — **No.** Plain Control; the armed-cameo highlight is the
  SelectClass `+0x34` highlight field (set by Mouse_Enter) and the app-level `targeting_mode`, not
  a gadget `is_on` latch (cameo lane §0/§3).
- *Cameo right-click marker?* — mask includes RIGHTPRESS (`0x10`); right fires on the press. The
  Rust `cameo_action` marks right via `RESULT_RIGHT` so the driver picks `hit_test_item(item,
  right=true)` (cameo lane §1; rust lane §1.2).
- *Does the catcher's full-viewport rect break the `HIT_SEED_AREA` cap?* — **No.** Dispatch uses
  `rect.contains` in list order; the seed gates only hover, and the catcher has no hover (tactical
  lane §6.2; `tick.rs:79-93` vs `:242-253`).
- *Model the minimap as a gadget owning the whole drag, or press-detect + flag gesture?* —
  **press-detect routed into the existing flag gesture** (lower risk; preserves current behavior).
  Sticky capture keeps the release bound to the minimap (rust lane §4; minimap lane §4.2).
- *Where do the new gadgets sit in retained order?* — chrome → cameos → minimap → tactical; all
  rects disjoint so order is observationally irrelevant (tactical lane §6.2).

### Deferred to Follow-up (do NOT invent values / do NOT silently change behavior)

- **Minimap continuous-follow vs gamemd press-edge jump (DRIFT, flagged for user decision).**
  gamemd re-centers only on press edges and ignores LEFTHELD; current Rust follows the cursor
  continuously while `minimap_dragging`. A3 PRESERVES the current behavior. Closing this drift is
  a separate, user-approved change (and may want the user's in-game observation first, since RA2
  muscle memory may expect drag-scroll). Evidence: minimap lane §2a/§2d/§4.2.
- **SBGadgetClass sidebar-body click catcher** (`0x00B07E58`, Action `0x006ABA40` = swallow +
  cursor reset, no ID; mask/sticky UNKNOWN this session) — **A6 scope**, not A2/A3. Without it, a
  click on empty sidebar chrome falls through to legacy `hit_test` (None) today; unchanged by
  A2/A3. Evidence: cameo lane §6.
- **Two radar-frame ShapeButton mode toggles** (`0x00B04978`/`0x00B04910`, IDs UNKNOWN) — **A6
  scope** (normal result-key ShapeButtons), not the A3 minimap catcher. Evidence: minimap lane §3.
- **Cameo tooltip CSF format** (CSF#0xC6E label unmapped) — A4 already ships the interim
  `"{name}\n${cost}"`; A2 does not change tooltip text, only the zero-delay-on-hover hook.
- **Tab/scroll tooltip numeric-CSF mapping** — A4 deferred (empty text). Unchanged by A2/A3.
- **`g_ScenarioClass+0x34b8` observer/special topAdj selector** (18 vs 26) — affects the gamemd
  row-count formula; our adaptive layout computes visible rows independently, so this is not a
  blocker, but pixel-exact row-count parity vs gamemd at a fixed resolution would need it
  (cameo lane §8). Out of A2 scope (geometry is R11 policy, study §6.4).

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/ui/gadget/list.rs` | add `GadgetBehavior::Cameo` + `GadgetBehavior::ClickRegion`; `GadgetSpec::cameo` + `GadgetSpec::click_region` ctors |
| Modify | `src/ui/gadget/button.rs` | add `cameo_action` (fire-on-press, LEFTUP-strip, left/right marker); route `Cameo`→`cameo_action`, `ClickRegion`→`base_action` in `dispatch_action` |
| Modify | `src/ui/gadget/mod.rs` | `CAMEO_FLAGS`, `TACTICAL_REGION_FLAGS`, `MINIMAP_REGION_FLAGS` constants |
| Modify | `src/app_gadget_input.rs` | cameo pool (build/grow/sync) + tactical/minimap region gadgets; cameo result mapping; cameo-hover tooltip override; `GadgetConsume` class + classification (A3) |
| Modify | `src/app_input.rs` | restructure `handle_mouse_input` to dispatch on `GadgetConsume`; extract `tactical_mouse`/`minimap_mouse` bodies; `hit_test_item`→`pub(crate)` consumer |
| Modify | `src/sidebar/mod.rs` | retire the cameo loop from `hit_test`; `pub(crate) fn hit_test_item`; update doc comment |
| Modify | `src/app_tooltips.rs` | (A2 T3) expose a `cameo_tip_id`-range helper if needed by the driver hover hook (else no change — driver owns the override) |

No file is expected to exceed ~600 lines of non-test code. `app_gadget_input.rs` grows from 316
to ~520 lines (driver — cohesive, allowed). `button.rs`/`list.rs` carry their existing large
`#[cfg(test)]` blocks plus the new behavior tests.

## Interface Changes

- **New public (crate) API:** `ui::gadget::list::GadgetBehavior::{Cameo, ClickRegion}`;
  `GadgetSpec::cameo(rect, id)`, `GadgetSpec::click_region(rect, flags)`; `ui::gadget::{CAMEO_FLAGS,
  TACTICAL_REGION_FLAGS, MINIMAP_REGION_FLAGS}`. Consumed only by `app_gadget_input`.
- **`sidebar::hit_test` narrows**: no longer returns cameo (`SidebarItem`) actions — the
  `for item in &view.items` loop (`mod.rs:410-414`) is removed. Sole caller
  `handle_sidebar_mouse_input` (`app_input.rs:234-245`) keeps compiling (it still handles
  pause/producer/dev buttons until A6).
- **`sidebar::hit_test_item` becomes `pub(crate)`** — the driver calls it to map a fired cameo id
  back to a `SidebarAction`.
- **`app_gadget_input::handle_mouse_button_event` return type changes `bool` → `GadgetConsume`**
  (A3 T3). Sole caller `app_input::handle_mouse_input` (`:43`) updated in the same task.
- **`InGameGadgets` gains fields:** `cameos: Vec<GadgetHandle>`, `tactical: Option<GadgetHandle>`,
  `minimap: Option<GadgetHandle>` (built lazily alongside `handles`).
- **`app_input` gains `pub(crate) fn tactical_mouse(...)` and `pub(crate) fn minimap_mouse(...)`**
  — extracted from the current `handle_mouse_input` body; called by the `GadgetConsume` match.

## Sim Checklist

This plan is **UI-layer only — no sim behavior/logic change**:

- [x] No sim behavior/logic change. No file under `src/sim/` is created, modified, or deleted by
  any task. (Unlike A4 T1, NO `ObjectType` field is added — `ui_name` already exists.)
- [x] No new dependency from `sim/` to ui/render/sidebar/audio/net.
- [x] Tick ordering in `World::advance_tick` unaffected; no state-hash change; no
  `SNAPSHOT_VERSION` change.
- [x] Player commands continue to enter the sim exclusively through the existing
  `app_commands::*` / `pending_commands` seam (study O14): cameo clicks map to the SAME
  `SidebarAction` handling; tactical/minimap clicks call the SAME selection/command handlers.
- [x] No `f32`/`f64` enters game logic — the new f32 use is render/UI-side (rect rounding) only;
  the gadget core stays integer-only.

## Risk Areas

- **`handle_mouse_input` restructure (A3 T3) is the highest-risk edit** — it rewrites the master
  in-game mouse handler. Mitigations: extract the existing bodies verbatim into
  `tactical_mouse`/`minimap_mouse` (no logic change, only relocation); the `GadgetConsume` match
  preserves the exact same call sequence; full `cargo test` + a manual selection/command/minimap
  checklist at the A3 gate. STOP and reassess if any existing selection/command test regresses.
- **Cameo release fall-through (A2-only interim).** A cameo press is consumed by the gadget
  (returns on press, no `begin_drag`); the non-sticky cameo does not consume the release (mask has
  no LEFTRELEASE), so the release falls through to the left-release body — **exactly as today**
  (the pre-A2 cameo path also returned on press and let the release fall through). A2 does not
  regress this; A3 T3 fixes it properly (a release whose class is not `Tactical` does not run the
  selection body). Documented, not a new bug.
- **Cameo pool order vs regions.** Cameos appended after the region gadgets; safe because all
  rects are disjoint (Key Decisions). If a future change makes them overlap, the order must be
  revisited — flagged in the driver comment.
- **Focus/capture during cameo pool growth.** Growing the pool (`add_tail`) never touches existing
  handles or focus slots; cameos are never sticky. Pool growth is gated to NOT run while a region
  holds capture (defensive) — see A2 T2 / A3 T2.
- **87-test shell net + all `ui/gadget/*` tests** must stay green — touched by nothing here;
  verified at every slice gate.
- **Parallel sessions:** the worktree is isolated at 801bc09e; if `cargo check` fails in files
  this plan does not touch, STOP — do not fix unrelated code.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| T1 | Cameo fires on PRESS (left build/arm, right cancel); LEFTUP stripped → idle-tick over a cameo does NOT fire/consume | Fires on every cameo click of every match; an idle-tick mis-fire would spuriously build/cancel | Unit: left-press posts `id\|0x8000`+consumes; right-press posts `id\|0xC000`+consumes; LEFTUP-only → 0, no consume, key untouched |
| T2 | Cameo id = `1000 + visible index`; result → `hit_test_item(item, right)` → existing `SidebarAction` | Every cameo click outcome (build/arm/SW/cancel/clear) must match the legacy path exactly | Unit: id→index→item mapping; manual: build/arm/SW/cancel cameo behave identically to pre-A2 |
| T3 | Cameo hover (G7) zeroes the tooltip delay; leaving a cameo restores 1000 ms | Continuous in normal play; cameo tips show instantly, others after 1 s (study §3.4) | Manual: hover cameo → tip immediate; hover tab → tip after 1 s; unit on the hover-class helper |
| T2/T4 | Cameo retained order = hit + draw order; cameo branch removed from `sidebar::hit_test` | One ordered list (O7/G20); no hit-vs-draw divergence | Build (no dead cameo path); manual cameo click + cameo draw unchanged |
| T5/T6 | Tactical catcher: invisible, sticky, mask `0x7F`, rect=play area; minimap: sticky, mask `0x9F`, rect=minimap; no result ID | A8-parity; sticky keeps a drag bound to its region across the sidebar boundary | Unit: press acquires capture, release releases; masked-0 sticky re-dispatch hits the holder |
| T6 | Catcher dispatches via `rect.contains` regardless of `HIT_SEED_AREA` | High-res viewport > seed must still catch clicks | Unit: a `>786,432 px²` ClickRegion still consumes a contained press in the broadcast walk |
| T7 | `GadgetConsume` routing: viewport click → tactical body; minimap → minimap body; chrome/cameo → return; sticky-captured release routes to its region even over the sidebar | The master input path; every in-game click depends on it | Full `cargo test`; manual: select/move/attack/box-select/SW/place/minimap-jump/right-cancel all unchanged; drag off tactical onto sidebar still completes on tactical |
| T7 | Minimap drag PRESERVED (continuous follow), drift logged not flipped | Avoid an unapproved behavior change | Manual: minimap drag scrolls as before; deferred-item note present |
| T8 | G14 ordering on overlap (smaller/earlier wins) + G15 sticky bypass | Substrate guarantees A3 relies on | Synthetic unit tests (overlapping catcher+button; masked-0 holder dispatch) |

---

## Tasks

Slice order: **A2** (cameo strip — additive, fire edge preserved) → **A3** (regions + the master
handler restructure). Each slice ends with a full `cargo test -p vera20k` gate.

---

### Task 1 (A2 T1): `Cameo` GadgetBehavior + `cameo_action`

**Why:** The cameo click surface needs a behavior that fires on the press edge, strips LEFTUP, and
marks left vs right — distinct from `Control` (which would mis-fire on LEFTUP and only mark right
on RIGHTRELEASE).

**Files:**
- Modify: `src/ui/gadget/mod.rs`
- Modify: `src/ui/gadget/list.rs`
- Modify: `src/ui/gadget/button.rs`

**Step 1: add the cameo mask constant.** In `src/ui/gadget/mod.rs`, after the existing flag
constants (near `STICKY_CTOR_MASK`), add:

```rust
/// SelectClass cameo construction mask (study cameo lane §3): LEFTPRESS |
/// LEFTUP | RIGHTPRESS. The cameo Action acts on the press bits and discards
/// LEFTUP; there is no release bit, so cameos fire on mouse-DOWN.
pub const CAMEO_FLAGS: u16 = FLAG_LEFT_PRESS | FLAG_LEFT_UP | FLAG_RIGHT_PRESS; // 0x19
```

**Step 2: add the `Cameo` variant + ctor.** In `src/ui/gadget/list.rs`, extend `GadgetBehavior`:

```rust
/// Which Action implementation a gadget runs.
#[derive(Debug, Clone, Copy)]
pub enum GadgetBehavior {
    /// Base action (G16): consume any masked flags.
    Plain,
    /// Control action (G13): post `id|0x8000` then base.
    Control,
    /// Toggle-button action (G22): the silent-press / fire-on-release machine.
    Button(ButtonState),
    /// Cameo action (A2 / SelectClass): fire on the press edge (left build/arm,
    /// right cancel); strip LEFTUP; post `id|0x8000` (`|0x4000` marks a right
    /// press) for the driver to map back to a SidebarAction. Not sticky, no
    /// toggle/latch state.
    Cameo,
    /// Invisible Action-only click region (A3 / tactical catcher + minimap):
    /// consume any masked flags, run sticky capture, post NO id. The driver
    /// identifies which region consumed by stored handle.
    ClickRegion,
}
```

Add a `cameo` ctor on `GadgetSpec` (after `button`):

```rust
    /// SelectClass cameo ctor (A2): mask 0x19, NOT sticky, `Cameo` behavior.
    /// `id` is the runtime cameo id (1000 + visible slot index).
    pub fn cameo(rect: GadgetRect, id: u16) -> Self {
        let mut spec = Self::new(rect, super::CAMEO_FLAGS, false);
        spec.id = id;
        spec.behavior = GadgetBehavior::Cameo;
        spec
    }
```

(`GadgetSpec::click_region` is added in A3 T1.)

**Step 3: add `cameo_action` + route it.** In `src/ui/gadget/button.rs`, import the extra flag
constants and add the function. Update the `use` line:

```rust
use super::{
    FLAG_LEFT_PRESS, FLAG_LEFT_UP, FLAG_RIGHT_PRESS, FLAG_RIGHT_RELEASE, PRESS_BITS, RELEASE_BITS,
    RESULT_BUTTON, RESULT_RIGHT,
};
```

Add the function (after `control_action`):

```rust
/// SelectClass cameo Action (A2): fires on the press edge. gamemd strips the
/// LEFTUP bit and acts only on LEFTPRESS/RIGHTPRESS — so an idle-tick dispatch
/// over a hovered cameo (which carries LEFTUP) is a no-op that does NOT consume
/// (the walk continues). On a real press it posts `id|0x8000` (left) or
/// `id|0x8000|0x4000` (right marker) and consumes. Cameos are not sticky, so no
/// capture runs.
pub(crate) fn cameo_action(g: &mut Gadget, masked: u16, key: &mut u16) -> u32 {
    // gamemd: `if (flags & LEFTUP) flags &= ~LEFTUP;`
    let masked = masked & !FLAG_LEFT_UP;
    let press = masked & (FLAG_LEFT_PRESS | FLAG_RIGHT_PRESS);
    if press == 0 {
        // No press edge left after the LEFTUP strip → no fire, no consume.
        return 0;
    }
    if g.id != 0 {
        *key = g.id | RESULT_BUTTON;
        if (press & FLAG_RIGHT_PRESS) != 0 {
            *key |= RESULT_RIGHT; // driver reads this as right_click
        }
    }
    g.is_to_redraw = true;
    1
}
```

Route it in `dispatch_action`:

```rust
    match g.behavior {
        GadgetBehavior::Plain | GadgetBehavior::ClickRegion => base_action(g, masked, focus),
        GadgetBehavior::Control => control_action(g, masked, key, focus),
        GadgetBehavior::Button(_) => toggle_action(g, masked, key, live, focus),
        GadgetBehavior::Cameo => cameo_action(g, masked, key),
    }
```

(`ClickRegion` is added to the enum in this task too — see Step 2 — but its `dispatch_action` arm
is wired here so the match is exhaustive; `base_action` gives the invisible-sticky behavior A3
needs. A3 T1 only adds the `click_region` ctor.)

**Step 4: tests.** Add to `button.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn cameo_fires_on_press_strips_leftup_a2() {
        let mut f = FocusState::new();
        let mut spec = GadgetSpec::new(GadgetRect::new(0, 0, 60, 48), crate::ui::gadget::CAMEO_FLAGS, false);
        spec.id = 1000;
        spec.behavior = GadgetBehavior::Cameo;
        let (mut l, a) = one_gadget(spec);
        let mut key: u16 = 0;
        // Left press → post id|0x8000, consume.
        assert_eq!(cameo_action(l.get_mut(a).unwrap(), 0x01, &mut key), 1);
        assert_eq!(key, 1000 | 0x8000);
        // Right press → post id|0x8000|0x4000, consume.
        key = 0;
        assert_eq!(cameo_action(l.get_mut(a).unwrap(), 0x10, &mut key), 1);
        assert_eq!(key, 1000 | 0x8000 | 0x4000);
        // LEFTUP only (idle-tick dispatch over a hovered cameo) → no fire, no
        // consume, key untouched.
        key = 0xBEEF;
        assert_eq!(cameo_action(l.get_mut(a).unwrap(), 0x08, &mut key), 0);
        assert_eq!(key, 0xBEEF);
        let _ = &mut f;
    }

    #[test]
    fn cameo_zero_id_posts_nothing_but_consumes() {
        let mut spec = GadgetSpec::new(GadgetRect::new(0, 0, 60, 48), crate::ui::gadget::CAMEO_FLAGS, false);
        spec.behavior = GadgetBehavior::Cameo;
        let (mut l, a) = one_gadget(spec);
        let mut key: u16 = 0x1234;
        assert_eq!(cameo_action(l.get_mut(a).unwrap(), 0x01, &mut key), 1);
        assert_eq!(key, 0x1234, "id 0 posts nothing");
    }
```

(`one_gadget` already exists in the test module.)

Add an end-to-end tick test (cameo fires on press, never on release/idle):

```rust
```

— place this in `tick.rs`'s test module instead (it exercises the full tick); see below.

In `src/ui/gadget/tick.rs` `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn cameo_fires_on_press_only_a2() {
        use crate::ui::gadget::CAMEO_FLAGS;
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let mut s = GadgetSpec::new(GadgetRect::new(0, 0, 60, 48), CAMEO_FLAGS, false);
        s.id = 1000;
        s.behavior = GadgetBehavior::Cameo;
        l.add_tail(s);
        // Prime current_list.
        tick(&mut l, &mut f, &idle(500, 500), &mut out);
        // Left press inside → fires 1000|0x8000.
        let r = tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(r, 1000 | 0x8000);
        assert_eq!(out.consumed_by, l.iter().next().map(|g| g.handle));
        // Idle tick over the cameo (left up): LEFTUP stripped → no fire, walk
        // not consumed.
        let r = tick(&mut l, &mut f, &idle(5, 5), &mut out);
        assert_eq!(r, 0);
        assert_eq!(out.consumed_by, None, "idle-over-cameo does not consume");
        // Release inside: cameo mask has no LEFTRELEASE → masked 0 → no dispatch.
        let r = tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_UP, 5, 5, false), &mut out);
        assert_eq!(r, crate::ui::gadget::KEY_LMB_UP, "release not consumed by cameo");
        assert_eq!(out.consumed_by, None);
    }
```

**Step 5: Verify.** `cargo test -p vera20k ui::gadget` → all existing + 3 new tests pass.

**Step 6: Commit.** `ui: A2 T1 — Cameo + ClickRegion GadgetBehavior; cameo_action (fire-on-press, LEFTUP-strip, left/right marker)`

---

### Task 2 (A2 T2): cameo gadget pool in the driver + result mapping

**Why:** The cameo click surface must live on the retained list (O7) and route a fired cameo id
back through the existing `hit_test_item` → `apply_sidebar_action` path (no behavior duplication).

**Files:**
- Modify: `src/sidebar/mod.rs` (make `hit_test_item` callable)
- Modify: `src/app_gadget_input.rs`

**Step 1: expose `hit_test_item`.** In `src/sidebar/mod.rs`, change
`fn hit_test_item(item: &SidebarItem, right_click: bool) -> SidebarAction` to
`pub(crate) fn hit_test_item(...)`. No body change. (The existing tests reference
`super::hit_test_item` — still valid.)

**Step 2: add cameo pool fields + the id base.** In `src/app_gadget_input.rs`, add to the id
constants block:

```rust
/// Cameo control id base (study cameo lane §2): runtime id = 1000 + visible
/// slot index. Mirrors the gamemd id space and the A4 tooltip id base
/// (`app_tooltips::CAMEO_TIP_ID_BASE`).
pub(crate) const ID_CAMEO_BASE: u16 = 1000;
```

Extend `InGameGadgets`:

```rust
pub(crate) struct InGameGadgets {
    pub list: GadgetList,
    pub focus: FocusState,
    pub out: TickOutput,
    pub left_held: bool,
    pub right_held: bool,
    pub handles: Option<SidebarButtonHandles>,
    /// Cameo gadget pool (A2): grown to fit the visible cameo count, never
    /// auto-shrunk; the tail past `view.items.len()` is disabled. Index in this
    /// Vec == cameo slot index == `id - ID_CAMEO_BASE`.
    pub cameos: Vec<GadgetHandle>,
}
```

Initialize `cameos: Vec::new()` in `InGameGadgets::new()`.

**Step 3: sync the cameo pool.** In `sync_gadgets`, after the chrome-button sync (after the
`scroll_up` sync line `:161`), add cameo pool grow + per-slot sync. Insert:

```rust
    sync_cameos(state, view);
}

/// Grow/sync the cameo gadget pool to mirror `view.items` (the already
/// visible/scrolled/per-tab cameo set). One cameo gadget per visible item, id =
/// ID_CAMEO_BASE + slot, rect from the item; the unused tail is disabled
/// (skipped by hit-test + walk). Cameos are appended after the chrome buttons
/// (and, post-A3, after the region gadgets) — all rects are disjoint so order is
/// observationally irrelevant (one pinned order for hit + draw per O7/G20).
fn sync_cameos(state: &mut AppState, view: &SidebarView) {
    let want = view.items.len();
    // Grow on demand (never while a gesture holds capture — defensive; cameos
    // are never sticky so this only guards the region capture added in A3).
    if state.in_game_gadgets.cameos.len() < want
        && state.in_game_gadgets.focus.sticky.is_none()
    {
        let zero = GadgetRect::new(0, 0, 0, 0);
        while state.in_game_gadgets.cameos.len() < want {
            let slot = state.in_game_gadgets.cameos.len();
            let id = ID_CAMEO_BASE + slot as u16;
            let h = state
                .in_game_gadgets
                .list
                .add_tail(GadgetSpec::cameo(zero, id));
            state.in_game_gadgets.cameos.push(h);
        }
    }
    let rects: Vec<GadgetRect> = view.items.iter().map(|it| rect_px(it.rect)).collect();
    let cameos = state.in_game_gadgets.cameos.clone();
    for (slot, h) in cameos.iter().enumerate() {
        if let Some(g) = state.in_game_gadgets.list.get_mut(*h) {
            match rects.get(slot) {
                Some(rect) => {
                    g.rect = *rect;
                    g.is_disabled = false;
                }
                None => {
                    g.is_disabled = true; // unused tail
                }
            }
        }
    }
}
```

(`SidebarView` is already imported; `view.items` is `Vec<SidebarItem>`.)

**Step 4: map a fired cameo id → action.** In `apply_gadget_result`, before the `_ => {}` arm,
add the cameo branch. The fn already computes `id = result & !(RESULT_BUTTON | RESULT_RIGHT)` and
has `view: &SidebarView`. Add:

```rust
        _ if (ID_CAMEO_BASE..ID_CAMEO_BASE + view.items.len() as u16).contains(&id) => {
            let slot = (id - ID_CAMEO_BASE) as usize;
            if let Some(item) = view.items.get(slot) {
                let right = (result & RESULT_RIGHT) != 0;
                let action = crate::sidebar::hit_test_item(item, right);
                crate::app_input::apply_sidebar_action(state, action);
            }
        }
```

(No sound for cameos — gamemd plays the per-action Voc/EVA inside the build path, already covered
by the existing build/queue handlers reached through `apply_sidebar_action`; the cameo gadget
itself plays no `GUITabSound`. This matches today's legacy cameo path, which also plays no extra
click sound.)

**Step 5: Verify.** `cargo test -p vera20k` (driver tests; the cameo result mapping is exercised
by the A2 T4 narrowing test + manual). Then manual: in-game, click a build cameo → queues the
build; right-click a queued cameo → cancels; SW cameo → arms; armed cameo left-click → clears —
all identical to pre-A2.

**Step 6: Commit.** `ui: A2 T2 — cameo gadget pool (mirrors SidebarView.items) + fired-id → hit_test_item → SidebarAction`

---

### Task 3 (A2 T3): cameo hover → tooltip zero-delay override (Mouse_Enter/Leave, G7)

**Why:** SelectClass::Mouse_Enter forces the tooltip delay to 0 (cameo tips show instantly);
Mouse_Leave restores 1000 ms. The gadget walk already reports hover transitions
(`out.hover_entered`/`hover_left`); A2 wires them to the A4 `set_delay_override` API.

**Files:**
- Modify: `src/app_gadget_input.rs`

**Step 1: add a cameo-membership helper.** In `app_gadget_input.rs`:

```rust
impl InGameGadgets {
    fn is_cameo(&self, h: GadgetHandle) -> bool {
        self.cameos.contains(&h)
    }
}
```

**Step 2: apply the hover override after each tick.** In `run_tick`, after the existing
`publish_pressed_visuals(state);` line, add:

```rust
    apply_cameo_hover_tooltip(state);
```

And add the function:

```rust
/// G7 hover hook (study cameo lane §4): entering a cameo forces the tooltip
/// delay to 0 (cameo tips show immediately on the next mouse-move); leaving a
/// cameo for a non-cameo (or nothing) restores the default 1000 ms. The walk's
/// hover transition (`out.hover_entered`/`hover_left`) is the Mouse_Enter/Leave
/// edge — reproducing SelectClass::Mouse_Enter/Leave's save-and-zero / restore.
fn apply_cameo_hover_tooltip(state: &mut AppState) {
    let entered = state.in_game_gadgets.out.hover_entered;
    let left = state.in_game_gadgets.out.hover_left;
    let entered_cameo = entered.is_some_and(|h| state.in_game_gadgets.is_cameo(h));
    let left_cameo = left.is_some_and(|h| state.in_game_gadgets.is_cameo(h));
    if entered_cameo {
        state.tooltips.set_delay_override(Some(0));
    } else if left_cameo {
        // Left a cameo and did NOT enter another cameo this tick → restore.
        state.tooltips.set_delay_override(None);
    }
}
```

(Cameo→cameo moves: `entered_cameo` stays true → override stays 0. Cameo→non-cameo:
`entered_cameo` false, `left_cameo` true → restore. Non-cameo→cameo: enter sets 0. Matches the
study's "stays 0 while over any cameo, restored on leaving the strip".)

**Step 3: Verify.** Unit-testable via the hover fields, but primarily a manual check:
`cargo test -p vera20k` stays green; in-game, hover a cameo → tooltip appears immediately; move to
a tab → tip waits ~1 s; move off the sidebar → no tip. Confirm `set_delay_override` is the only
caller (no double-restore fighting the A4 driver — A4's `sync_in_game_regions` does not touch the
override).

**Step 4: Commit.** `ui: A2 T3 — cameo hover drives the tooltip zero-delay override (Mouse_Enter/Leave, G7)`

---

### Task 4 (A2 T4): retire the cameo branch from `sidebar::hit_test`; A2 slice gate

**Why:** Cameos now live on the gadget list; the legacy `hit_test` cameo loop is dead and must be
removed so there is ONE hit path for cameos (R7 / O7). The dev/pause/producer buttons stay on
`hit_test` until A6.

**Files:**
- Modify: `src/sidebar/mod.rs`

**Step 1: remove the cameo loop.** In `sidebar::hit_test` (`mod.rs:405-438`), delete the
`for item in &view.items { ... }` block (`:410-414`). The function keeps the panel gate +
pause/producer/dev branches. Update the doc comment to note cameos are now substrate-owned:

```rust
/// Legacy press-path hit-test for the surfaces NOT yet on the gadget substrate
/// (pause/producer, dev buttons). Tabs/repair/sell/scroll (A1) AND the cameos
/// (A2) are owned by `app_gadget_input`; they are deliberately absent here.
pub fn hit_test(view: &SidebarView, x: f32, y: f32, right_click: bool) -> SidebarAction {
    if !view.panel_rect.contains(x, y) {
        return SidebarAction::None;
    }
    if let Some(button) = view.pause_button.as_ref() {
        // ... unchanged ...
```

(`right_click` is still used by the remaining branches' actions; if clippy warns it is now unused,
keep it for signature stability and add `let _ = right_click;` — but pause/producer/dev actions
do not branch on it, so verify the warning and prefix with `_right_click` only if clippy flags it.
The existing `hit_test_item` is now `pub(crate)` and reached only from the driver.)

**Step 2: Verify the A2 slice.** Run in the worktree:
- `cargo test -p vera20k` → ALL tests green (gadget behavior tests, sidebar `hit_test_item` SW
  tests `mod.rs:440-524`, the 87-test shell net, everything).
- `cargo clippy -p vera20k` → no new warnings from the edited files.
- **Manual A2 checklist** (in-game): build cameo left-click queues; right-click cancels a queued;
  SW cameo arms/clears; ready building cameo arms placement; disabled cameo does nothing; cameo
  hover shows the tip immediately; tab switch swaps the cameo set; scroll pages the cameos —
  every outcome identical to the pre-A2 build.

**Step 3: Commit.** `ui: A2 T4 — retire the cameo branch from sidebar::hit_test (cameos now substrate-owned); A2 slice gate`

---

### Task 5 (A3 T1): `ClickRegion` ctor

**Why:** The `ClickRegion` variant + `base_action` route already landed in A2 T1 (for an
exhaustive match). A3 adds the `click_region` ctor (invisible, sticky, custom mask).

**Files:**
- Modify: `src/ui/gadget/list.rs`

**Step 1: add the ctor.** After `GadgetSpec::cameo`:

```rust
    /// Invisible Action-only click region ctor (A3): sticky, custom event mask
    /// (tactical 0x7F / minimap 0x9F), `ClickRegion` behavior, no id. The
    /// sticky byte makes the dispatcher acquire capture on a press so a drag
    /// stays bound to the region across the sidebar boundary (G17).
    pub fn click_region(rect: GadgetRect, flags: u16) -> Self {
        let mut spec = Self::new(rect, flags, true); // sticky=true → Flags |= 5 (already in 0x7F/0x9F)
        spec.behavior = GadgetBehavior::ClickRegion;
        spec
    }
```

**Step 2: tests.** In `list.rs` tests:

```rust
    #[test]
    fn click_region_ctor_sticky_invisible_a3() {
        let r = GadgetRect::new(0, 0, 800, 600);
        let spec = GadgetSpec::click_region(r, 0x7F);
        assert!(spec.sticky, "click region is sticky");
        assert_eq!(spec.flags, 0x7F, "0x7F already includes the sticky |5 bits");
        assert!(matches!(spec.behavior, GadgetBehavior::ClickRegion));
        assert_eq!(spec.id, 0, "no id");
    }
```

**Step 3: Verify.** `cargo test -p vera20k ui::gadget::list`.

**Step 4: Commit.** `ui: A3 T1 — GadgetSpec::click_region ctor (invisible, sticky, no id)`

---

### Task 6 (A3 T2): tactical + minimap region gadgets in the driver

**Why:** The two A8 click regions must be on the retained list (sticky) so tactical/minimap clicks
resolve through the gadget walk and a drag stays bound to its region.

**Files:**
- Modify: `src/ui/gadget/mod.rs`
- Modify: `src/app_gadget_input.rs`

**Step 1: region mask constants.** In `src/ui/gadget/mod.rs`:

```rust
/// Full-tactical catcher mask (study tactical lane §3): all mouse bits except
/// RIGHTUP and KEYBOARD = 0x7F.
pub const TACTICAL_REGION_FLAGS: u16 =
    FLAG_LEFT_PRESS | FLAG_LEFT_HELD | FLAG_LEFT_RELEASE | FLAG_LEFT_UP
        | FLAG_RIGHT_PRESS | FLAG_RIGHT_HELD | FLAG_RIGHT_RELEASE; // 0x7F
/// Minimap/radar region mask (study minimap lane §1b): left press/held/release/
/// up + right-press + right-up; NOT right-held, NOT right-release = 0x9F.
pub const MINIMAP_REGION_FLAGS: u16 =
    FLAG_LEFT_PRESS | FLAG_LEFT_HELD | FLAG_LEFT_RELEASE | FLAG_LEFT_UP
        | FLAG_RIGHT_PRESS | FLAG_RIGHT_UP; // 0x9F
```

**Step 2: region handle fields.** Extend `InGameGadgets`:

```rust
    /// Full-tactical catcher (A3): invisible sticky region over the play area.
    pub tactical: Option<GadgetHandle>,
    /// Minimap/radar click region (A3): invisible sticky region over the radar
    /// minimap; None-rect (disabled) when the radar is offline / minimap absent.
    pub minimap: Option<GadgetHandle>,
```

Init both `None` in `new()`.

**Step 3: build + sync the regions.** In `sync_gadgets`, build the regions once (after the chrome
build block, before the cameo sync) and sync their rects each frame. The tactical rect is the play
area left of the sidebar; the minimap rect is the live minimap screen rect.

Add to the one-time build (inside the `if gadgets.handles.is_none()` block is the wrong place —
regions are independent of `handles`; build them in their own lazy guard):

```rust
    // Build the A3 regions once (independent of the chrome `handles` guard).
    if state.in_game_gadgets.tactical.is_none() {
        let zero = GadgetRect::new(0, 0, 0, 0);
        let tac = state
            .in_game_gadgets
            .list
            .add_tail(GadgetSpec::click_region(zero, TACTICAL_REGION_FLAGS));
        let mini = state
            .in_game_gadgets
            .list
            .add_tail(GadgetSpec::click_region(zero, MINIMAP_REGION_FLAGS));
        state.in_game_gadgets.tactical = Some(tac);
        state.in_game_gadgets.minimap = Some(mini);
    }
```

Place this build BEFORE `sync_cameos(state, view);` so the order is chrome → regions → cameos.
(Cameos appended last; disjoint rects make this irrelevant — documented in `sync_cameos`.)

Then sync the region rects (add near the end of `sync_gadgets`, after the chrome sync, before
`sync_cameos`):

```rust
    // Tactical catcher rect = the play area left of the sidebar (gamemd's
    // g_RadarViewport*). Disabled in the map editor (gamemd registers it only
    // when !g_IsMapEditor — we have no in-game editor, so always enabled here;
    // documented for parity).
    let panel_left = view.panel_rect.x.round() as i32;
    let play_rect = GadgetRect::new(0, 0, panel_left.max(0), state.render_height() as i32);
    if let (Some(tac), Some(g)) = (
        state.in_game_gadgets.tactical,
        state
            .in_game_gadgets
            .tactical
            .and_then(|h| state.in_game_gadgets.list.get_mut(h)),
    ) {
        let _ = tac;
        g.rect = play_rect;
        g.is_disabled = false;
    }
    // Minimap region rect = the live minimap screen rect; disabled when absent.
    let mini_rect = state.active_minimap_screen_rect.map(rect_px);
    if let Some(mh) = state.in_game_gadgets.minimap
        && let Some(g) = state.in_game_gadgets.list.get_mut(mh)
    {
        match mini_rect {
            Some(r) => {
                g.rect = r;
                g.is_disabled = false;
            }
            None => g.is_disabled = true,
        }
    }
```

> NOTE for the implementer: verify the exact type/name of `state.active_minimap_screen_rect`
> (rust lane §4 cites it in `app_sidebar_render`). If it is not an `AppState` field, source the
> minimap rect the same way `is_cursor_over_minimap` / `update_camera_from_minimap_cursor` does
> (`app_sidebar_render.rs:201-237/345-366`) — e.g. via a small `pub(crate)` accessor — rather than
> recomputing geometry. The rect MUST match the existing minimap hit region exactly.

**Step 4: Verify (build only this task).** `cargo test -p vera20k ui::gadget` (substrate unchanged)
+ `cargo check -p vera20k` (driver compiles with the new fields/regions). Routing lands in T7;
until then the regions are in the list but `handle_mouse_input` still uses the bool path — confirm
no behavior change yet (the regions consume nothing the old path needed because the old path runs
after the gadget `bool`; a region consuming a tactical press would currently SWALLOW it — so DO
NOT ship T6 alone to a playable build; T6+T7 land together for the gate). Mark T6 as build-verified
only; the playable gate is T7.

**Step 5: Commit.** `ui: A3 T2 — tactical + minimap ClickRegion gadgets on the retained list (rects synced per frame)`

---

### Task 7 (A3 T3): `GadgetConsume` routing in `handle_mouse_input`

**Why:** With the regions on the list, a tactical/minimap press is now consumed by the gadget walk.
The master mouse handler must route a region-consumed edge to the existing tactical/minimap bodies
(not short-circuit). This is the slice's load-bearing edit.

**Files:**
- Modify: `src/app_gadget_input.rs` (return a `GadgetConsume` class)
- Modify: `src/app_input.rs` (dispatch on the class; extract `tactical_mouse`/`minimap_mouse`)

**Step 1: define the class + classify.** In `src/app_gadget_input.rs`:

```rust
/// What the in-game gadget walk did with a mouse edge (A3 routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GadgetConsume {
    /// Nothing on the list consumed it — fall through to the legacy path.
    NotConsumed,
    /// A chrome button or cameo handled it — the caller returns.
    Consumed,
    /// The full-tactical catcher consumed it — run the tactical body.
    Tactical,
    /// The minimap region consumed it — run the minimap body.
    Minimap,
}
```

Change `handle_mouse_button_event` to return `GadgetConsume`:

```rust
pub(crate) fn handle_mouse_button_event(
    state: &mut AppState,
    button: MouseButton,
    pressed: bool,
) -> GadgetConsume {
    match button {
        MouseButton::Left => state.in_game_gadgets.left_held = pressed,
        MouseButton::Right => state.in_game_gadgets.right_held = pressed,
        _ => return GadgetConsume::NotConsumed,
    }
    let Some(view) = current_sidebar_view(state) else {
        return GadgetConsume::NotConsumed;
    };
    sync_gadgets(state, &view);
    let key = match (button, pressed) {
        (MouseButton::Left, true) => KEY_LMB_DOWN,
        (MouseButton::Left, false) => KEY_LMB_UP,
        (MouseButton::Right, true) => KEY_RMB_DOWN,
        (MouseButton::Right, false) => KEY_RMB_UP,
        _ => return GadgetConsume::NotConsumed,
    };
    run_tick(state, &view, key)
}
```

Change `run_tick` to return `GadgetConsume`. The key change: capture the pre-tick sticky holder
(the sticky tier does not set `out.consumed_by`), then classify the routed handle:

```rust
fn run_tick(state: &mut AppState, view: &SidebarView, key: u16) -> GadgetConsume {
    let cx = state.cursor_x.round() as i32;
    let cy = state.cursor_y.round() as i32;
    let input = GadgetInput {
        queued_key: key,
        event_x: cx,
        event_y: cy,
        mouse_x: cx,
        mouse_y: cy,
        left_held: state.in_game_gadgets.left_held,
        right_held: state.in_game_gadgets.right_held,
        shift: crate::app_input::is_shift_held(state),
        ctrl: crate::app_input::is_ctrl_held(state),
        alt: crate::app_input::is_alt_held(state),
    };
    let prev_sticky = state.in_game_gadgets.focus.sticky;
    let gadgets = &mut state.in_game_gadgets;
    let result = tick(&mut gadgets.list, &mut gadgets.focus, &input, &mut gadgets.out);
    // Broadcast-tier consumer, else the pre-tick sticky holder (the sticky tier
    // dispatches the holder but does not set consumed_by).
    let routed = state.in_game_gadgets.out.consumed_by.or(prev_sticky);
    let fired = (result & RESULT_BUTTON) != 0;
    if fired {
        apply_gadget_result(state, view, result);
    }
    publish_pressed_visuals(state);
    apply_cameo_hover_tooltip(state);
    classify(state, routed, fired)
}

/// Map the consuming/holding handle to a routing class.
fn classify(state: &AppState, routed: Option<GadgetHandle>, fired: bool) -> GadgetConsume {
    let g = &state.in_game_gadgets;
    match routed {
        Some(h) if Some(h) == g.tactical => GadgetConsume::Tactical,
        Some(h) if Some(h) == g.minimap => GadgetConsume::Minimap,
        Some(_) => GadgetConsume::Consumed, // chrome button or cameo
        None => {
            // No gadget consumed and no capture. A fired control with no
            // consumed_by cannot happen (a fire sets consumed_by), so this is a
            // genuine miss.
            if fired { GadgetConsume::Consumed } else { GadgetConsume::NotConsumed }
        }
    }
}
```

(Cameo fires set `out.consumed_by` to the cameo handle → `routed` is the cameo → `Consumed`. Chrome
button presses capture (sticky) → on later edges `routed = prev_sticky` = the chrome handle →
`Consumed`. Region presses → `Tactical`/`Minimap`; region-captured drags/releases → `prev_sticky`
= the region → same class. )

`idle_tick` keeps calling `run_tick(state, &view, 0)` but ignores the returned class (idle ticks
drive visuals/hover only):

```rust
pub(crate) fn idle_tick(state: &mut AppState) {
    let Some(view) = current_sidebar_view(state) else {
        return;
    };
    sync_gadgets(state, &view);
    let _ = run_tick(state, &view, 0);
}
```

**Step 2: extract the tactical/minimap bodies.** In `src/app_input.rs`, split the current
`handle_mouse_input` (`:34-169`) into the dispatcher + two extracted bodies. The bodies are the
existing code, relocated verbatim (no logic change):

```rust
/// Handle mouse button press/release for selection and move commands.
pub(crate) fn handle_mouse_input(
    state: &mut AppState,
    button: MouseButton,
    btn_state: ElementState,
) {
    use crate::app_gadget_input::GadgetConsume;
    let pressed = btn_state.is_pressed();
    // Gadget substrate first (study G22/A8): chrome buttons + cameos fire/consume
    // here; the tactical/minimap regions decide WHICH body runs.
    match crate::app_gadget_input::handle_mouse_button_event(state, button, pressed) {
        GadgetConsume::Consumed => return,
        GadgetConsume::Tactical => {
            tactical_mouse(state, button, btn_state);
            return;
        }
        GadgetConsume::Minimap => {
            minimap_mouse(state, button, btn_state);
            return;
        }
        GadgetConsume::NotConsumed => {}
    }
    // Legacy press path for the surfaces still off the substrate (pause/producer,
    // dev buttons). Empty-sidebar / off-window clicks fall through to nothing.
    if pressed && handle_sidebar_mouse_input(state, button) {
        return;
    }
}

/// Tactical-viewport mouse body (extracted from the legacy handler; routed here
/// when the full-tactical ClickRegion consumes the edge). Logic unchanged.
pub(crate) fn tactical_mouse(state: &mut AppState, button: MouseButton, btn_state: ElementState) {
    match button {
        MouseButton::Left => {
            if btn_state.is_pressed() {
                if state.targeting_mode.is_some() {
                    return; // suppress selection drag while a targeting mode is armed
                }
                state.selection_state.begin_drag(state.cursor_x, state.cursor_y);
            } else {
                if let Some(section) = state.armed_super_weapon_type().map(str::to_owned) {
                    crate::app_commands::launch_super_weapon_at_cursor(state, &section);
                    return;
                }
                if let Some(type_id) = state.armed_building_type().map(str::to_owned) {
                    place_ready_building_at_cursor(state, &type_id);
                    return;
                }
                let action: SelectAction =
                    state.selection_state.end_drag(state.cursor_x, state.cursor_y);
                let shift = is_shift_held(state);
                if let SelectAction::Click(_, _) = action {
                    let commanded: bool = try_queue_context_order_at_screen_point(
                        state,
                        state.cursor_x,
                        state.cursor_y,
                        true,
                    );
                    if commanded {
                        return;
                    }
                }
                let mut queued_selection: Option<Vec<u64>> = None;
                if let Some(sim) = &state.simulation {
                    match action {
                        SelectAction::Click(sx, sy) => {
                            let world_x: f32 = sx / state.zoom_level + state.camera_x;
                            let world_y: f32 = sy / state.zoom_level + state.camera_y;
                            let fog_ref = if state.sandbox_full_visibility { None } else { Some(&sim.fog) };
                            queued_selection = compute_click_selection_snapshot(
                                sim.entities(),
                                fog_ref,
                                preferred_local_owner_name(state).as_deref(),
                                world_x,
                                world_y,
                                CLICK_SELECT_RADIUS,
                                shift,
                                state.rules.as_ref(),
                                &state.height_map,
                                Some(&state.tactical_bridge_inverse_map),
                                Some(&sim.interner),
                            );
                        }
                        SelectAction::BoxSelect(min_x, min_y, max_x, max_y) => {
                            let fog_ref = if state.sandbox_full_visibility { None } else { Some(&sim.fog) };
                            let z = state.zoom_level;
                            queued_selection = compute_box_selection_snapshot(
                                sim.entities(),
                                fog_ref,
                                preferred_local_owner_name(state).as_deref(),
                                min_x / z + state.camera_x,
                                min_y / z + state.camera_y,
                                max_x / z + state.camera_x,
                                max_y / z + state.camera_y,
                                shift,
                                Some(&sim.interner),
                            );
                        }
                        SelectAction::None => {}
                    }
                }
                if let Some(snapshot) = queued_selection {
                    emit_selection_voice(state, &snapshot);
                    queue_selection_snapshot_command(state, snapshot, shift);
                }
            }
        }
        MouseButton::Middle => {
            if btn_state.is_pressed() {
                state.middle_mouse_panning = true;
                state.middle_mouse_anchor_x = state.cursor_x;
                state.middle_mouse_anchor_y = state.cursor_y;
            } else {
                state.middle_mouse_panning = false;
            }
        }
        MouseButton::Right if btn_state.is_pressed() => {
            if state.targeting_mode.is_some() {
                state.targeting_mode = None;
                state.building_placement_preview = None;
                return;
            }
            queue_selection_snapshot_command(state, Vec::new(), false);
        }
        _ => {}
    }
}

/// Minimap mouse body (extracted; routed here when the minimap ClickRegion
/// consumes the edge). Preserves the current continuous-follow drag gesture —
/// the gamemd press-edge-jump difference is a DEFERRED parity item, not flipped
/// here.
pub(crate) fn minimap_mouse(state: &mut AppState, button: MouseButton, btn_state: ElementState) {
    match button {
        MouseButton::Left => {
            if btn_state.is_pressed() {
                crate::app_sidebar_render::try_begin_minimap_drag(state);
            } else if state.minimap_dragging {
                state.minimap_dragging = false;
            }
        }
        // Right/middle on the minimap: no current behavior; preserved as no-op.
        _ => {}
    }
}
```

> IMPLEMENTER NOTES (preserve behavior exactly):
> - The `Middle` and `Right` arms move into `tactical_mouse` because middle-pan and right-cancel
>   currently live in the master handler and apply to the play area. A middle/right press over the
>   tactical catcher routes here. A middle/right press over the sidebar (no region) → `NotConsumed`
>   → legacy (no-op today) — matching the current behavior where a right-press over the sidebar
>   that hits no item falls through to the right arm... **CHECK THIS:** today a right-press anywhere
>   in-game that isn't consumed reaches the right arm (deselect). With A3, a right-press over empty
>   sidebar is `NotConsumed` and the new dispatcher does NOT run the right-cancel. To preserve "right
>   anywhere deselects", either (a) keep a right-press fallthrough in the dispatcher's `NotConsumed`
>   arm, or (b) accept that right-cancel now requires the click be over the tactical area (gamemd:
>   right is handled by the tactical catcher, which only covers the viewport — so right over the
>   sidebar does NOT deselect in gamemd). **Prefer (b)** (gamemd-faithful: the catcher owns
>   right-press; sidebar right does nothing) — but VERIFY against the current Rust expectation and
>   note it in the commit. If the user relies on right-over-sidebar deselect, add (a).
> - `try_begin_minimap_drag` already contains the "units selected → minimap move-order, else begin
>   drag" split (`app_sidebar_render.rs:224-237`); do not duplicate it.
> - The leading `try_begin_minimap_drag` call at the OLD `:54` is removed (minimap entry now comes
>   via the `Minimap` class). The leading `minimap_dragging` end-check at `:64` moves into
>   `minimap_mouse`'s left-release arm.

**Step 3: Verify.** `cargo test -p vera20k` → all green. Then the **manual A3 checklist** (in-game):
- Left-click empty ground deselects/none; left-click own unit selects; box-select drags a band;
  left-drag from tactical onto the sidebar then release → still completes on tactical (sticky).
- Move/attack orders to ground/enemy; SW launch at cursor; building placement on ready cameo.
- Right-click deselects (over tactical); targeting-mode right-cancel.
- Minimap left-click jumps/move-orders; minimap drag scrolls the camera as before.
- A sidebar button/cameo click still works and does NOT trigger tactical selection.

**Step 4: Commit.** `app: A3 T3 — route tactical/minimap clicks through the gadget walk (GadgetConsume); extract tactical_mouse/minimap_mouse`

---

### Task 8 (A3 T4): headless acceptance tests + A3 slice gate

**Why:** Lock the substrate guarantees A3 relies on (study §8 A3 acceptance: walk ordering,
sticky bypass, catcher-area independence).

**Files:**
- Modify: `src/ui/gadget/tick.rs` (test module)

**Step 1: add tests.**

```rust
    #[test]
    fn a3_smaller_or_earlier_wins_overlapping_region_and_button() {
        // A small button overlapping a large catcher: the button consumes the
        // press first (earlier in the walk) — the catcher is the fallback only.
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let button = l.add_tail(btn(GadgetRect::new(0, 0, 20, 20), 0x65));
        let _catcher = l.add_tail(GadgetSpec::click_region(GadgetRect::new(0, 0, 800, 600), 0x7F));
        tick(&mut l, &mut f, &idle(500, 500), &mut out); // prime
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(out.consumed_by, Some(button), "button (earlier) consumes, not the catcher");
    }

    #[test]
    fn a3_catcher_dispatches_above_hit_seed_area() {
        // A region larger than HIT_SEED_AREA still consumes a contained press in
        // the broadcast walk (dispatch uses rect.contains, not the hover seed).
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let catcher = l.add_tail(GadgetSpec::click_region(GadgetRect::new(0, 0, 1920, 1080), 0x7F));
        assert!(1920 * 1080 > crate::ui::gadget::HIT_SEED_AREA);
        tick(&mut l, &mut f, &idle(5000, 5000), &mut out); // prime, hover off
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 100, 100, true), &mut out);
        assert_eq!(out.consumed_by, Some(catcher), "rect.contains dispatch, seed-independent");
        assert_eq!(f.sticky, Some(catcher), "press acquires sticky capture");
    }

    #[test]
    fn a3_sticky_region_keeps_drag_across_boundary() {
        // Press on the catcher captures; an idle masked-0 tick re-dispatches the
        // HOLDER even with the cursor over a (later) button rect (G15 bypass).
        let mut f = FocusState::new();
        let mut out = TickOutput::default();
        let mut l = GadgetList::new(ListId(1));
        let catcher = l.add_tail(GadgetSpec::click_region(GadgetRect::new(0, 0, 100, 100), 0x7F));
        let _button = l.add_tail(btn(GadgetRect::new(200, 0, 20, 20), 0x65));
        tick(&mut l, &mut f, &idle(500, 500), &mut out);
        tick(&mut l, &mut f, &event(crate::ui::gadget::KEY_LMB_DOWN, 5, 5, true), &mut out);
        assert_eq!(f.sticky, Some(catcher));
        // Drag onto the button rect: held idle tick goes to the catcher (sticky
        // tier exclusive), the button is never dispatched.
        let mut held = idle(205, 5);
        held.left_held = true;
        tick(&mut l, &mut f, &held, &mut out);
        assert_eq!(out.dispatches.len(), 1);
        assert_eq!(out.dispatches[0].handle, catcher, "held drag stays with the catcher");
        // Release over the button still releases the catcher's capture.
        let up = event(crate::ui::gadget::KEY_LMB_UP, 205, 5, false);
        tick(&mut l, &mut f, &up, &mut out);
        assert_eq!(f.sticky, None, "release frees capture");
    }
```

**Step 2: A3 slice gate.** In the worktree:
- `cargo test -p vera20k` → ALL green (new A3 tests + everything).
- `cargo clippy -p vera20k` → no new warnings from edited files.
- Re-run the full **manual A3 checklist** from T7.

**Step 3: Commit.** `ui: A3 T4 — A3 acceptance tests (walk ordering, sticky bypass, seed-independent dispatch); A3 slice gate`

---

## Post-implementation

- **Update the study slice ledger**: mark A2 + A3 done in
  `GADGET_DIALOG_CONTROL_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §8 (local-only doc) — leaves A6
  (command bar + dev/pause/producer + SBGadget body catcher + radar mode buttons) + B-track.
- **File the deferred items** (do NOT implement without sign-off): minimap continuous-follow vs
  press-edge-jump drift; SBGadget body catcher (A6); radar mode buttons (A6); cameo/tab/scroll
  CSF tooltip text mapping (A4 follow-up).
- **Do NOT commit** anything under `docs/` (gitignored, local-only) as part of the worktree
  branch — the worknotes and this plan live in the main repo's local `docs/` only.
