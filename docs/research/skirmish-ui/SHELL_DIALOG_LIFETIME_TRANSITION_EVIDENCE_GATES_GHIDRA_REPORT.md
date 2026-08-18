# Shell Dialog Lifetime and Transition Evidence Gates — Ghidra Report

**Date:** 2026-07-18  
**Target:** active Yuri's Revenge `gamemd.exe`  
**Mode:** exhaustive slice  
**Scope:** seven evidence gates required before the approved shell-dialog transition implementation may begin

## Status

Working investigation document. No implementation is authorized by this report until every load-bearing question below is resolved, explicitly classified as `UNKNOWN`, or converted into a bounded implementation blocker.

## Scope boundary

This investigation is limited to:

1. Choose Map (`0x6B`) child reveal and Z-order behavior.
2. Static-child classification, first-paint defaults, and repaint participation.
3. BITFONT Path A text rasterization and tint conversion.
4. Optional shell chrome animation and its runtime predicates.
5. Close-start sound, close wait-loop, timeout, and failure behavior.
6. Non-animated shell composition order and underdraw ownership.
7. Escape/Cancel routing and action ownership for dialogs `0x6B`, `0xE2`, `0x100`, and `0x102`.

No Rust code, INI data, asset files, or unrelated shell behaviors are in scope.

## Open Questions Log

- [RESOLVED] Q01 — `0x6B` schedule assignment is not callback/Z-order-derived. The geometry walk assigns Use Map `0x6C5` value `1` and Create Random Map `0x583` value `2`; the count callbacks retain no IDs or HWND order.
- [RESOLVED] Q02 — Cancel `0x5C0` is the special Group-A-framed control and reads value `N_A=2`, so it shares Create Random Map's value `2`.
- [RESOLVED] Q03 — The kind-1 classifier and exact four-dialog membership are pinned in Gate 2.
- [RESOLVED] Q04 — Every qualifying child starts `(running=0,count=1,interval=30,step=1,range=8,sound=-1)`.
- [RESOLVED] Q05 — Kind-1 statics are not slide-count inputs. Membership is `0xE2:{0x694}`, `0x100:{0x694}`, `0x102:{0x694,0x6EC,0x5A8}`, `0x6B:{0x694}`.
- [RESOLVED] Q06 — `0x4EE` starts a stopped reveal; timer messages invalidate only; each eligible paint draws and then advances the count. Transition suppression prevents that paint path.
- [OPEN] Q07 — What are the exact input/output encodings of BITFONT Path A at `0x00434CD0`, including UTF-16 termination and unsupported glyph handling?
- [OPEN] Q08 — What exact arithmetic converts/tints BITFONT pixels, including integer widths, signedness, shifts, rounding, and clamps?
- [OPEN] Q09 — Which globals select RGB555 versus RGB565, and what are their active YR default values?
- [OPEN] Q10 — What verified golden vectors can be derived directly from retail/binary behavior for BITFONT Path A?
- [RESOLVED] Q11 — Node flags `+0xD9/+0xDA/+0xDB` independently select SDWRNTMP, SDMPBTN, and radar branches; `+0xDC` is not consumed by the compositor. Gate 4 gives every reachable frame and rectangle.
- [PARTIAL] Q12 — Within the blocking compositor, optional D9/DA chrome overlays the surface present at entry and precedes ordinary SDBTNANM draws. Static evidence alone does not order the pre-existing background/movie/text/cursor pixels; Gate 6 owns that capture boundary.
- [RESOLVED] Q13 — Close start loads `RulesClass` data-root `+0x19C`, parsed from `[AudioVisual] GUIMoveOutSound`; stock binds `MenuSlideOut` (`uslide2`, volume `60`).
- [RESOLVED] Q14 — The close helper uses wrapping 32-bit `GetTickCount` subtraction followed by signed `JG 5000`; it pumps messages/network or the guarded main tick before testing pending-byte completion and elapsed time.
- [RESOLVED] Q15 — Guard failures are side-effect free; audio failure is ignored; timeout restores child paint flags and the parent's prior enabled state but does not clear the deferred-slide byte. A post-disable record-table failure returns without cleanup.
- [RESOLVED] Q16 — The helper owns transition attempt and input/paint suppression only. Each caller unconditionally owns its later hide/destroy/navigation action and ignores the helper result.
- [OPEN] Q17 — What is the complete non-animated shell composition order, including background, underdraw, chrome, children, and cursor/focus artifacts?
- [RESOLVED] Q18 — The referenced `+0x50` is an immediate 80-pixel x displacement, not a record field. It is confined to the DB radar group when screen width is at least 800 and does not affect SDTP, SDMPBTN, SDWRNTMP, or ordinary controls.
- [OPEN] Q19 — For each target dialog, does Escape synthesize a Cancel command, invoke a dialog-specific branch, or bypass transition teardown?
- [OPEN] Q20 — For Choose Map, does Cancel restore selection/state before or after the close transition, and which routine owns the final action?
- [OPEN] Q21 — Are any investigated branches dormant TS legacy or gated off by stock YR defaults?
- [RESOLVED] Q22 — Special Back/Cancel controls use Group-A frame arithmetic and share the last ordinary schedule value; the loop executes exactly `N_A+9` ticks (`0..N_A+8`) with no extra terminal tick.
- [OPEN] Q23 — Which retail assets and metadata drive optional chrome and non-animated composition; what are their exact dimensions and lookup precedence?
- [OPEN] Q24 — Which current Rust paths consume or approximate each behavior, and what observations remain regression-only rather than parity evidence?

## Entry-point and caller map

Record offsets in this report use the established **data-root convention** unless
explicitly called a hash-node offset: the owner-draw hash node stores the HWND at
`+0x00`, data begins at node `+0x04`, and the chain link is at node `+0x204`.
Thus data `+0xB0/+0xBD/+0xBE` correspond to node `+0xB4/+0xC1/+0xC2`.

| Address | Verified role in this slice | Active edge |
|---|---|---|
| `0x00610CA0` | shared subclass procedure; stages a dialog's first-paint state and calls the direct SHOW wrapper at `0x00612690` | first eligible parent paint |
| `0x00608260` | synchronous SHOW wrapper: checks gates, plays `Rules+0x1A0` (`GUIMoveInSound`), suppresses eligible child paints, calls the core with `DL=1`, restores, invalidates | entry |
| `0x00608070` | deferred CLOSE wrapper: checks the same gates, plays `Rules+0x19C` (`GUIMoveOutSound`), sets data `+0xBE`, invalidates, pumps until completion or timeout | close/pre-modal |
| `0x00607FD0` | deferred-paint consumer: if data `+0xBE` is set, optionally sends `0x4E2` to child `0x71A`, calls the core with `DL=0`, then clears `+0xBE` | CLOSE paint |
| `0x006071E0` | blocking 30 ms-per-iteration transition compositor; `DL=1` is SHOW and ends with completion sound plus `0x4EC`; `DL=0` is CLOSE and ends with `0x4ED` only | both |
| `0x00622B50` | common dialog procedure; parent paint/composition and deferred-close consumption | all shell dialogs |
| `0x00622720` | generic teardown: pre-close helper, close transition attempt, `DestroyWindow`, stack compaction, focus restoration | close/destroy |
| `0x007757E0` | modal-stack pop: close transition attempt, destroy target and dialogs above it, restore focus/result | modal close |
| `0x006ACEE0` | Skirmish command handler; control `0x5AA` calls close transition on the still-visible `0x102` HWND, hides it, runs modal `0x6B`, then restores it | Choose Map open/return |

The July correction that caller `ECX` is valid is confirmed directly. At
`0x006AD92F`, `MOV ECX,EBP` precedes the call at `0x006AD931`; the function
prologue at `0x006ACEEA` saved the setup HWND from incoming `ECX` into `EBP`.
Likewise `0x0062272F` loads the saved HWND into `ECX` before the generic teardown
call. In `0x007757E0`, the selected HWND stays in `ECX` from stack lookup through
the call at `0x00775831`. The older “uninitialized/garbage ECX, therefore no-op”
conclusion is stale.

## Gate findings

### Gate 1 — Choose Map reveal and Z-order

**Verdict: CLOSED.** The `0x6B` control wave is fixed by count plus geometry,
not by the enumeration order assumed in the task wording.

`FUN_0060A180 @ 0x0060A180..0x0060A227` and
`FUN_0060A250 @ 0x0060A250..0x0060A2F7` count visible qualifying controls but
store no HWND, control ID, or enumeration-order vector. The schedule constructor
at `0x0060766B..0x006076A4` builds `{1..N_A, 0, N_A+3, 0}`. The later geometry
walk at `0x006079CF..0x00607B23` binds those ordinary values to the controls:
Use Map `0x6C5` is value `1`, and Create Random Map `0x583` is value `2`.
Cancel `0x5C0` uses the special block at `0x00607C63..0x00607C83` and reads
`schedule[N_A-1]`, also value `2`. Create Random Map remains visible and counted
even when disabled; `0x005E6CAF..0x005E6CCE` and
`0x005E6F6F..0x005E6F8E` call `EnableWindow` but never hide it.

For both directions, `delta = tick - entry_value`. A negative delta selects the
held-before frame; deltas `0..5` select the six ramp frames; later deltas select
the held-after frame. The literal `0x6B` table is:

| Control | Predicate/class | Value | SHOW pre / ramp / post | CLOSE pre / ramp / post |
|---|---|---:|---|---|
| Use Map `0x6C5` | visible allow-listed owner-draw Button, ordinary Group A | 1 | `10 / 10,9,8,7,6,5 / 1` | `1 / 5,6,7,8,9,10 / 10` |
| Create Random Map `0x583` | visible allow-listed owner-draw Button, ordinary Group A; enabled state irrelevant | 2 | `10 / 10,9,8,7,6,5 / 1` | `1 / 5,6,7,8,9,10 / 10` |
| Cancel `0x5C0` | visible special control; Group-A frame family | 2 | `10 / 10,9,8,7,6,5 / 1` | `1 / 5,6,7,8,9,10 / 10` |

With `N_A=2`, the maximum schedule value is `5`, the loop bound is
`max+6=11`, and the only rendered ticks are `0..10`:

| Tick | Use SHOW | Create/Cancel SHOW | Use CLOSE | Create/Cancel CLOSE |
|---:|---:|---:|---:|---:|
| 0 | 10 | 10 | 1 | 1 |
| 1 | 10 | 10 | 5 | 1 |
| 2 | 9 | 10 | 6 | 5 |
| 3 | 8 | 9 | 7 | 6 |
| 4 | 7 | 8 | 8 | 7 |
| 5 | 6 | 7 | 9 | 8 |
| 6 | 5 | 6 | 10 | 9 |
| 7 | 1 | 5 | 10 | 10 |
| 8 | 1 | 1 | 10 | 10 |
| 9 | 1 | 1 | 10 | 10 |
| 10 | 1 | 1 | 10 | 10 |

The exact runtime `EnumChildWindows` Z-order remains unnecessary to this result:
neither count callback preserves order, and the geometry walk is the binding
authority.

### Gate 2 — Static classification and repaint

**Verdict: CLOSED pending only offset notation cross-check.** Fresh classification
of the four parent dialogs produces this literal membership and initialization:

| Parent | Kind-1 child IDs | Initial state for every listed child |
|---|---|---|
| Main Menu `0xE2` | `0x694` | `running=0`, `count=1`, `interval=30 ms`, `step=1`, `range=8`, `sound=-1` |
| Single Player `0x100` | `0x694` | same |
| Skirmish `0x102` | `0x694`, `0x6EC`, `0x5A8` | same |
| Choose Map `0x6B` | `0x694` | same |

`0x71C` is not kind 1. The classifier is
`FUN_00602490 @ 0x00602490..0x00602AD1`; initialization is in
`FUN_0060A5B0 @ 0x0060A5B0..0x0060AA4F` with value helpers
`0x00600CA0`, `0x006015E0`, and `0x00601D20`. These statics never contribute
to `N_A`: the slide counter separately requires the owner-draw Button class-type
zero path.

`OwnerDraw_Static_006153E0 @ 0x006153E0..0x00616300` handles the reveal:

1. message `0x4EE` starts only a stopped kind-1 record, installs
   `running=1,count=1`, starts timer ID `0` at 30 ms, and invalidates;
2. another `0x4EE` while running is a no-op;
3. `WM_TIMER` invalidates but does not increment;
4. an unsuppressed `WM_PAINT` draws with the current count and then adds `step`;
5. timer ID `0` is killed once the count reaches the wide-text length plus
   `range+1`; `running` remains set after completion; and
6. before the first `0x4EE`, kind-1 text is not drawn.

The target records use `sound=-1`, so these starts emit no sound. The transition
callback `LAB_00606800` selects, for parent `0x6B`, exactly
`{0x694,0x695,0x468,0x6C5,0x583,0x5C0}` and writes `(lParam != 0)` to
data-root `+0xBC`; `0x6EB` and `0x553` are excluded. Reads at
`0x0061374B..0x00613753`, inside `OwnerDraw_Static_006153E0`, and at
`0x006AE45D..0x006AE47B` make this a paint/input-suppression byte, not a child
enabled-state or reveal-order field. The wrappers broadcast `1` around the
transition and `0` afterward; parent `EnableWindow` is separate.

### Gate 3 — BITFONT Path A

Pending.

### Gate 4 — Optional chrome animation

**Verdict: CLOSED for every implementation-facing branch.** The flag offsets
below are stated both ways to avoid the historical four-byte convention error:

| Dialog | data `+0xD5` / node `+0xD9` (D9) | data `+0xD6` / node `+0xDA` (DA) | data `+0xD7` / node `+0xDB` (DB) | data `+0xD8` / node `+0xDC` |
|---|---:|---:|---:|---:|
| `0xE2` | 0 | 0 | 0 | 0 |
| `0x100` | 0 | 0 | 0 | 0 |
| `0x102` | 1 | 1 | 0 | 0 |
| `0x6B` | 1 | 0 | 0 | 0 |

The values are produced by `FUN_0060CAF0`, `FUN_0060C930`,
`FUN_0060CCC0`, and `FUN_0060CDB0`. They are independent predicates;
node `+0xDC` is never consumed by `FUN_006071E0` and is not a direction bit.

For Group-A count `N`, schedule construction at
`0x00607646..0x006076AD` gives DA anchor `0`, D9 anchor `N+3`, DB anchor
`0`, and ticks `0..N+8`. Thus `0x102` and `0x6B` both execute 11 ticks and
D9 starts at tick `5`; `0xE2` and `0x100` execute 14 and 12 ticks respectively,
but enable neither D9 nor DA.

DA/SDMPBTN, enabled only for `0x102`, draws exactly once per tick:

| Direction | ticks `0..5` | ticks `6..10` |
|---|---|---|
| SHOW | frames `6,5,4,3,2,1` | frame `0` held |
| CLOSE | frames `1,2,3,4,5,6` | frame `6` held |

D9/SDWRNTMP, enabled for `0x102` and `0x6B`, is:

| Direction | ticks `0..4` | ticks `5..10` |
|---|---|---|
| SHOW | SDTP frame `0` only | SDTP frame `1`, then SDWRNTMP `5,4,3,2,1,0` |
| CLOSE | SDTP frame `1` only | SDTP frame `1`, then SDWRNTMP `0,1,2,3,4,5` |

The loop terminates after D9 delta `5`; its apparent post-phase is unreachable.
At CLOSE tick `6`, held SDMPBTN frame `6` moves to the early DA call, so it
precedes D9 thereafter. These branches are at `0x006076F4..0x006079CA`.

`RightPanel__ComputeLayoutRects @ 0x0072EC70` yields:

- SDMPBTN origin `(effective_right-156, y_margin+157)`, canvas metadata
  `156x84`;
- SDWRNTMP origin `(effective_right-168, y_margin)`, rect metadata `168x177`;
- SDTP underdraw at the SDWRNTMP origin, using its full `168x199` canvas, not
  clipped to 177 pixels; and
- at 800x600, origins `(644,157)` and `(632,0)` respectively.

`FUN_0072A9C0` copies x/y without coordinate conversion. The literal `+0x50`
x displacement is confined to the DB radar-background group for widths
`>=800`; it never applies to SDTP, SDMPBTN, SDWRNTMP, or SDBTNANM, and DB is
off for all four dialogs.

The explicit in-core shape order per tick is: early DA terminal/base call; D9
SDTP or SDTP-underlay plus SDWRNTMP; DA ramp/SHOW-terminal SDMPBTN; ordinary
Group-A/Group-B/special SDBTNANM controls; DB radar group; full-surface copy,
service, and 30 ms wait. For `0x102` SHOW this is therefore D9, then SDMPBTN,
then ordinary controls. No background, text, movie, hover, pressed-state, or
cursor paint is issued inside this core; Gate 6 owns the surrounding composition.

Retail frames were inspected: SDMPBTN contains 7 frames on a `156x84` canvas,
SDWRNTMP 6 on `168x177`, and SDTP 2 on `168x199`. SDMPBTN frame 6 has a
shorter `156x44` encoded subframe; the renderer must honor SHP frame metadata
rather than hard-code a crop.

### Gate 5 — Close-start sound and timeout/error behavior

**Verdict: CLOSED.** The close helper is a blocking transition attempt followed
by caller-owned action dispatch. Its return value does not decide whether the
caller proceeds.

#### Exact start sound

`FUN_00608070` loads the Rules singleton at `0x0060810C`, then loads data-root
`+0x19C` at `0x0060811E`, passes `EDX=0x2000`, stack arguments `0` and
`1.0f`, and calls `VocClass__PlayAtPos @ 0x00750920` at `0x00608124`.
There is no result test; an absent/failed sound does not alter the transition.

The mapping is not inferred from spelling. The only string
`GUIMoveOutSound` is at `0x0083AB10`; its data xref is
`RulesClass__ReadAudioVisual @ 0x006694C7`. That parse block reads the old
data-root `+0x19C` value at `0x006694BB` and stores the parsed/fallback value at
`0x006694F1`. Retail data binds:

| Source | Literal |
|---|---|
| `ini/rules.ini:494` | `GUIMoveOutSound=MenuSlideOut` |
| `ini/rulesmd.ini:648` | `GUIMoveOutSound=MenuSlideOut` |
| `ini/sound.ini:3194..3196` | `[MenuSlideOut]`, `Sounds=uslide2`, `Volume=60` |
| `ini/soundmd.ini:2954..2956` | same YR patch value |

This sound is audible at CLOSE start in stock YR. It is separate from
`ShellButtonSlideSound` at Rules data-root `+0x750`: that stock-empty field is
loaded only on the `DL=1` SHOW completion branch at `0x00607F4A..0x00607F5F`.

#### Gates and ordered active path

`FUN_00608070 @ 0x00608070..0x0060825F` performs this exact order:

1. Call `FUN_0069BBE0`; any nonzero result returns `0`.
2. Require the owner-draw table to exist and find the HWND record.
3. Require data `+0xBD != 0` and data `+0xB0 == 1`.
4. Call `IsWindowVisible`; a false result returns `0`.
5. Play `Rules+0x19C` as above.
6. Save `IsWindowEnabled(hwnd) != 0`, then call `EnableWindow(hwnd, FALSE)`.
7. `EnumChildWindows(hwnd, 0x00606800, 1)` sets the transition paint-suppress
   byte on eligible child records; it is not a blanket child `EnableWindow` call.
8. Re-find the parent record, set data `+0xBE = 1`, and call
   `InvalidateRect(hwnd, NULL, FALSE)`.
9. Save `GetTickCount()` and run the inline pump until completion/abort/timeout.
10. `EnumChildWindows(..., 0x00606800, 0)` clears eligible child paint-suppress
    bytes and `EnableWindow` restores the parent's saved boolean state.
11. Return `1`. There is no final invalidate in this close wrapper.

Disabling an already-disabled but visible parent is allowed; cleanup restores it
disabled. Hidden, unregistered, non-mode-1, or non-slide-gated parents return
`0` before sound or state mutation.

#### Pump, completion, and exact timeout comparison

Each iteration first calls `Process_NetworkMessages @ 0x005D4D50`. When
`g_GameMode` is `0` or `5`, or either front-end blocker global is set, it then
calls `Network_ServiceLoop @ 0x0048D080`; otherwise it uses
`FUN_0055CBF0` as the guard before `Main_Tick @ 0x0055D360`. A nonzero
`Main_Tick` result exits through normal cleanup.

After the pump work, the helper snapshots whether data `+0xBE` is clear, calls
`GetTickCount`, subtracts the start tick in 32-bit arithmetic, compares the
result to `0x1388`, and branches with signed `JG` at `0x00608230`. Timeout is
therefore **signed `(i32)(now.wrapping_sub(start)) > 5000`**, not `>= 5000`.
If time has not expired, a cleared `+0xBE` exits; otherwise it loops. The return
is `1` for normal completion, main-tick abort, and timeout alike.

The timeout is a wait-for-paint guard, not a pre-emptive animation watchdog.
Once a `WM_PAINT` consumer enters the blocking core, the outer loop cannot
recheck elapsed time until the core returns.

#### Completion and caller-owned action

On a successful CLOSE paint, `FUN_00607FD0` calls the core with `DL=0` at
`0x00608057..0x0060805B`. The core sends parent message `0x4ED` at
`0x00607FA8..0x00607FB4`, with no completion sound and no `0x4EC -> 0x4EE`
static reveal. It returns, then `0x00608060` clears data `+0xBE`; the outer
helper sees that clear and restores input/paint state.

The following actions are unconditional after the helper returns:

- `FUN_00622720`: `DestroyWindow`, compact dialog LIFO state, restore focus.
- `FUN_007757E0`: destroy the selected modal and every stack entry above it,
  update top/result, restore focus.
- `FUN_006ACEE0` control `0x5AA`: hide `0x102`, run `0x6B`, then restore
  `0x102`; its saved setup transaction is not owned by the close helper.

Thus a gate return `0` or a timeout does **not** cancel the pending navigation or
destruction. The helper never owns the semantic action.

#### Failure-state ledger

| Condition | Helper result | State/action consequence |
|---|---:|---|
| predicate nonzero, table absent, HWND missing, mode/gate false, invisible | `0` | no sound, suppression, dirty byte, or enabled-state change; caller still proceeds |
| sound lookup/playback absent or fails | ignored | transition continues |
| parent initially disabled | `1` after active path | remains disabled after restoration |
| data `+0xBE` clears normally | `1` | child suppression cleared, parent enabled state restored, caller proceeds |
| `Main_Tick` returns nonzero before completion | `1` | cleanup runs; `+0xBE` is not explicitly cleared here; caller proceeds |
| elapsed signed delta becomes `>5000` before paint completion | `1` | cleanup runs, `+0xBE` remains set, no guaranteed `0x4ED`; caller proceeds |
| table disappears or parent relookup fails after disable/suppress | `0` | exceptional early return skips cleanup: parent stays disabled and eligible child paint-suppress bytes stay set; caller proceeds |

No stock single-threaded trigger that removes the parent record during the
synchronous child enumeration was found. The exceptional post-disable branch is
still part of the literal mechanism and must not be rewritten in the report as
successful cleanup.

### Gate 6 — Non-animated composition

Pending.

### Gate 7 — Escape/Cancel routing

Pending.

## UI composition ledger

Pending.

## Retail INI and asset evidence

Pending.

## Current Rust touchpoints

Current Rust has an entry-only wave, not a dialog lifetime controller:

| Rust surface | Current behavior relevant to the gates |
|---|---|
| `src/ui/shell/slide.rs` | count-only `ShellSlideSpec`; stale regular counts `0xE2=6`, `0x100=4`, `0x102=3`; no `0x6B` rendered spec; no caller-owned close phase |
| `src/app_shell_transition.rs` | detects screen edges, starts SHOW only, cancels on render fallback, clears the wave before a separate action model; only `0x102` starts statics |
| `src/app.rs` | parses/plays `gui_move_in_sound`; route handlers mutate screens or launch state immediately; Choose Map is embedded state; no close-start sound hook or timeout/action ownership model |
| `src/rules/ruleset.rs` | parses `GUIMoveInSound`; neither `GUIMoveOutSound` nor `ShellButtonSlideSound` is exposed for this lifecycle |
| `src/ui/skirmish_shell/static_reveal.rs` | timer advances Rust scalar-character count; inactive means full text; gradient is deferred |
| `src/render/bit_font.rs` and `src/render/shell_text.rs` | Rust-scalar cutoff and uniform per-glyph tint; not exact wide-unit Path A |
| `src/app_skirmish_shell_render.rs` | parent `0x102` is suppressed while chooser is open, but chooser has no independent transition lifetime; optional chrome is largely static/currently approximate |

These are implementation deltas, not parity evidence. Existing Rust unit tests
and Rust-vs-Rust frame behavior are regression ratchets only.

## Adversarial checks

Pending.

## Implementation handoff

Pending.

## Remaining unknowns and blockers

Pending.

## Evidence call ledger

Binary claims above were rechecked against the current `gamemd.exe` program with
read-only Ghidra operations:

- `decompile_function(0x00608070)` and `disassemble_function(0x00608070)` —
  complete close gate/sound/suppression/pump/timeout body `0x00608070..0x0060825F`.
- `decompile_function(0x00607FD0)` and `disassemble_function(0x00607FD0)` —
  deferred CLOSE paint consumer `0x00607FD0..0x0060806A`.
- `decompile_function(0x00608260)` and `disassemble_function(0x00608260)` —
  direct SHOW wrapper and `DL=1` call `0x00608260..0x00608370`.
- `get_assembly_context(0x00607F39,0x00607F48,0x00607F59,0x00607FA8,0x00607FAE)` —
  direction-dependent completion sound/message tail.
- `search_strings("GUIMoveOutSound")`, `get_xrefs_to(0x0083AB10)`, and
  `get_assembly_context(0x006694C7)` — exact Rules parser mapping to data `+0x19C`.
- `get_xrefs_to(0x00608070)` and assembly contexts at `0x00622731`,
  `0x00775831`, `0x005DE951`, `0x005E6A03`, `0x006AD931` — complete direct-call
  set and valid `ECX` flow.
- `decompile_function(0x00622720)`, `decompile_function(0x007757E0)`, and
  `decompile_function(0x006ACEE0)` — action ownership after the helper.
- `get_assembly_context(0x00606800,0x00607150,0x006071AD)` — child callback
  classification tail and data `+0xBC` paint-suppress write.

Additional gate-specific calls are appended below when their lanes reconcile.
