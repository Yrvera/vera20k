# DisplayClass Remainders — Ghidra Research Report (2026-04-24)

Closes two DisplayClass follow-ups from the discovery report:

1. `BandBox_MouseMove` / `BandBox_LeftUp` state machine
2. **Correction:** DisplayClass does NOT have multiple inheritance.
   The "secondary vtable fragments" I flagged earlier are adjacent
   unrelated vtables (BufferStraw etc.) in `.rdata`.

**Confidence:** HIGH for both corrections. Action-code enumeration
in LeftUp is MEDIUM — I listed every code I saw in the switch but
did not decompile each action's handler in depth.

**Active in YR:** Yes — bandbox drives every band selection and
click-to-order in tactical play.

---

## 1. Correction: DisplayClass is single-inheritance

The discovery report (`DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md`, §3)
said DisplayClass's vtable ends "roughly at slot 49" with the
remainder being "secondary-inheritance vtable fragments" because
slots 50 and 54 contained pointers into `.rdata`. That was wrong.

**What's actually at 0x7E61DC onwards:**

```
0x7E61D8 (slot 49): 0x004AAD30       ← last real DisplayClass vtable slot
0x7E61DC:           0x007FFD08       ← COL pointer for NEXT vtable
0x7E61E0:           <BufferStraw vtable slot 0>
0x7E61E4:           <BufferStraw vtable slot 1>
...
```

`0x7E61E0` is referenced from 11+ independent sites — `Straw__
Constructor`, `CDFileClass__Constructor`, `ReadMapOverlayPacks`, etc
— as the vtable of an I/O streaming class, **not** as part of
DisplayClass's vtable. The COL at `0x7FFD08` identifies it as
`BufferStraw` (type descriptor string `.?AVBufferStraw@@`).

So the `.rdata` layout around DisplayClass is:

```
0x7E6114  ┌──────────────────────────┐
          │ DisplayClass vtable      │  50 slots (inc. inherited
          │ 50 × 4 = 200 bytes       │           MapClass methods)
0x7E61DC  ├──────────────────────────┤
          │ BufferStraw COL-4        │
0x7E61E0  ├──────────────────────────┤
          │ BufferStraw vtable       │  ~4 slots
          │ ...                      │
0x7E61EC  ├──────────────────────────┤
          │ Another COL-4 marker     │
          │ Another class vtable     │  different I/O class
          │ ...                      │
          └──────────────────────────┘
```

These classes are **unrelated to DisplayClass** — they just happen
to sit next to it because the linker groups vtables by module.

### Verifying single inheritance

Xrefs to `0x7E6114` (the start of the DisplayClass vtable): only
**2 hits**, both in `DisplayClass__constructor` (one at `0x4A8830`)
and `FUN_006AC861` (the mislabeled second "Constructor"). If
DisplayClass had secondary base classes, we'd expect additional
`mov [ecx+N], &vtable_secondary` patterns inside the constructor,
writing different vtable pointers into adjusted `this` positions.
There are none.

The DisplayClass constructor ends with:
```
*param_1 = &vtable_DisplayClass;   // single primary vtable write
```

That's the only vtable write. Confirmed single-inheritance from
MapClass.

### Vtable size: 50 slots

- Slots 0–29: inherited from MapClass (with 6 DisplayClass overrides)
- Slots 30–49: DisplayClass-specific additions

The overrides in the inherited range and all 20 new slots were
already enumerated in `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` §3
and §4. That part stands.

**Action:** `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` §3's claim
that "the primary DisplayClass vtable is roughly slots 0–49" should
be replaced with "the DisplayClass vtable is exactly 50 slots, with
no secondary-inheritance fragments. Slots after 0x7E61D8 in memory
belong to unrelated I/O streaming classes (BufferStraw and
neighbors)."

---

## 2. BandBox state machine

Two vtable methods + a shared set of state fields on DisplayClass.

### State fields on DisplayClass

| Offset | Field | Init | Set by |
|--------|-------|------|--------|
| `+0x11CF` | `bandbox_active` (byte) | 0 | set to 1 on drag-threshold-crossed in MouseMove; cleared in LeftUp |
| `+0x11D0` | `drag_pending` (byte) | 0 | set by LeftDown (not decompiled here); cleared in MouseMove when drag starts |
| `+0x11B3` | `some_mode_flag_a` (byte) | 0 | suspends bandbox when set |
| `+0x11B4` | `some_mode_flag_b` (byte) | 0 | suspends bandbox when set |
| `+0x11D4` | `drag_start_x` (int) | 0 | cursor X where mouse-down happened |
| `+0x11D8` | `drag_start_y` (int) | 0 | cursor Y where mouse-down happened |
| `+0x11DC` | `last_end_x` (int) | 0 | most recent clamped cursor X |
| `+0x11E0` | `last_end_y` (int) | 0 | most recent clamped cursor Y |
| `+0x474`  | `selection_active` (byte) | 0 | cleared at end of LeftUp paths |

Extra actors outside DisplayClass:
- `g_Tactical + 0xD7D` — single dirty-bit flag, flipped on any
  band-rect change to drive redraw
- `g_RadarViewportWidth` / `g_RadarViewportHeight` — cursor clamp
  bounds
- `Tactical::InitBandRect()`, `Tactical::UpdateBandRectEnd()` —
  draw-state update sites for the band rectangle
- `Tactical::ProcessBandBoxSelection(callback)` — the selection
  resolver that calls into each object in the rect

### `BandBox_MouseMove(0x4AC380)` — vtable slot 47

Branches on bandbox_active vs drag_pending:

```
if bandbox_active or mode_flag_a or mode_flag_b:
    # Drag is active — update the band rect
    clamp cursor_x to [0, g_RadarViewportWidth - 1]
    clamp cursor_y to [0, g_RadarViewportHeight - 1]
    if (clamped_x, clamped_y) != (last_end_x, last_end_y):
        Tactical.dirty_flag_0xD7D = 1
        Tactical::UpdateBandRectEnd()
    return

elif drag_pending:
    # Mouse is down but haven't crossed threshold yet
    dx = cursor_x - drag_start_x
    dy = cursor_y - drag_start_y
    if sqrt(dx² + dy²) > 4:
        bandbox_active = 1
        drag_pending = 0
        if not (mode_flag_a or mode_flag_b):
            Tactical.dirty_flag_0xD7D = 1
            Tactical::InitBandRect()
            FUN_005BDC80(0, 0)   # screen save/restore routine
```

**Key constant:** **4-pixel drag threshold**. Below this, a
mouse-down stays in "pending" state and releases as a click. At or
above, transitions to an active bandbox drag.

### `BandBox_LeftUp(0x4AB9B0)` — vtable slot 48

Large (~270 lines), two main branches gated by
`param_1[0x469] != 0`:

#### Branch A — No building placement in progress (`[0x469] == 0`)

```
if bandbox_active:
    Tactical.dirty_flag_0xD7D = 1
    if not FUN_0054F5C0(0x10):  # some multiplayer check
        if not Tactical::AnyObjectInBandRect():
            bVar2 = true         # "empty bandbox"
        else:
            Desync_Handler()
    Tactical::ProcessBandBoxSelection(Selection__PickCallback)
    ActionLines::StartTimer()
    bandbox_active = 0
    vtable+0x48(this, 0, param_6)   # dispatch "selection changed"
    selection_active = 0
    DAT_00a8ed9d = 1
    if not bVar2:
        return    # selection consumed the click

# Otherwise (or if bandbox ended empty), dispatch by action code param_5
switch param_5:
    case 8:   # special: convert to 7 if no valid command + transform ordinary click into move
        if param_4 != NULL and current objects selected:
            resolve building target / unit target
            if humanPlayer:
                dispatch action 8
            else: goto action_7_fallback
        else: param_5 = 7; goto case 7
    case 7, 0: move / default click
    case 0x3D: set waypoint / beacon
    case 0xA: attack-move
    case 0xC: repair / guard (→ command code 0x16 or 0x17)
    case 0xD: guard cell
    case 0x21: enter / board (code 1 if building, 2 if unit)
    case 0x3C: place beacon (via RadarClass::PlaceBeacon)
    case 0x10..0x48 ranges: many other action codes (see §3)

    (each dispatched through Selection::DispatchMultiUnitOrder +
     FUN_004C65E0 / 004C6650 / 004C6AE0 / 004C6B60 packet builders)

    → each pushes a command into g_CommandBuffer at
      g_CommandQueue_WriteIndex, 0x6F bytes per command, wrapping
      at 0x80 slots, with a timeGetTime() timestamp stored at
      g_CommandTimestamps[write_index * 4]
```

#### Branch B — Building placement in progress (`[0x469] != 0`)

```
if placement_target (param_1[0x46A]).abstract_type == 7:
    # compute world coord from cursor cell + offset_x/offset_y
    target_coord = (cursor_x + offset_x, cursor_y + offset_y)
    valid = FUN_004A8EB0(placement_target, placement_house, placement_ord, &target_coord)
    self.place_valid_flag[+0x460] = valid

if placement_target is BuildingClass of type == 6
   and param_4.abstract_type == 6
   and BuildingClass::CanAcceptUpgrade(placement_target, g_PlayerPtr):
    self.place_valid_flag[+0x460] = 1    # force-allow upgrade placement

if not place_valid_flag or not some_mode_flag[+0x1181]:
    VoxClass::PlayEVA(0xFFFFFFFF)   # "cannot place here" warning
    FUN_00731CF0()
    return

# Build and queue a "place building" command (code 0xB)
cmd = FUN_004C6AE0(PlayerID, 0x0B, unit_ptr, build_id, facing, target_coord)
push cmd into g_CommandBuffer
cleanup placement state fields
```

### Command queue (shared with all DispatchMultiUnitOrder codepaths)

Internal structure:
```
g_CommandBuffer           — ring buffer of commands
g_CommandQueue_WriteIndex — next write index (masked with 0x7F)
g_CommandQueue_Count      — current depth (cap 0x80 = 128 slots)
g_CommandTimestamps[i]    — timeGetTime() recorded per command
```

Command size: **0x6F bytes = 111 bytes** per command. The write
loop copies:
- 27 dwords (`0x1B` × 4 bytes = 108 bytes) via unrolled `mov`
- then 2 more bytes + 1 byte = 111 total

Commands represent buffered player orders. In multiplayer, these
get sent to the network layer; in single-player, they execute the
following tick. A key bit of determinism: the same command struct
layout is used for **all** action types — only the "opcode" field
inside it varies.

### Action code summary (from the LeftUp switch)

Every numeric `param_5` I saw in the switch, with best-guess
meaning. Not exhaustive — many branches in the giant `else if`
chain with nothing decompiled beyond the `!= N` check.

| Code | Hex | Guessed action |
|------|-----|---------------|
| 0 | 0x00 | default (click → select + optional move) |
| 7 | 0x07 | move / select (primary) |
| 8 | 0x08 | attack target (unit / structure) |
| 10 | 0x0A | attack-move |
| 12 | 0x0C | guard / repair (→ cmd 0x16 or 0x17) |
| 13 | 0x0D | guard with target (→ cmd 0x16) |
| 14 | 0x0E | — |
| 15 | 0x0F | — |
| 20 | 0x14 | — |
| 33 | 0x21 | enter / board (cmd 1 or 2 depending on cell override flag) |
| 34 | 0x22 | — |
| 37 | 0x25 | — |
| 38 | 0x26 | — |
| 39 | 0x27 | — |
| 40 | 0x28 | — |
| 41 | 0x29 | — |
| 58 | 0x3A | — |
| 60 | 0x3C | place beacon (via `RadarClass::PlaceBeacon`) |
| 61 | 0x3D | set rally / waypoint (via `FUN_00430F70`) |
| 65 | 0x41 | — |
| 66 | 0x42 | — |
| 67 | 0x43 | — |
| 68 | 0x44 | — |
| 69 | 0x45 | — |
| 70 | 0x46 | — |
| 72 | 0x48 | — |

The un-labeled ones are follow-up work (each is a specific
player-issued order like Deploy, Chrono-Vortex, Iron-Curtain, etc.
in a dispatch table elsewhere in the binary).

### Why `Desync_Handler` in band-box selection?

One notable detail: when `AnyObjectInBandRect` returns non-zero
during a multiplayer-gated band-box selection,
`Desync_Handler()` is called. This is the same function used
throughout gamemd to mark "potential network desync detected,
log for debugging". The band-box selection shouldn't mutate
world state, so why desync concern?

Best guess: `ProcessBandBoxSelection` internally calls methods
on selected objects that MIGHT mutate state (e.g., setting a
"selected" flag on each object for rendering). If the set of
selected objects differs across peers (timing-sensitive cursor
position at a boundary), the mutation diverges. The Desync marker
is a defensive canary, not an actual error path — the game
continues normally but logs the incident.

### Rust parity

Rust bandbox logic in `src/app_entity_pick.rs` exists but wasn't
audited in this pass. Things to check:
- **4-pixel drag threshold**: does Rust match this exactly?
- **Clamp to radar viewport** during drag: does Rust use the same
  bounds, or the full window size?
- **Command size / queue depth**: Rust presumably uses a different
  queue shape, but the 0x80 depth limit and 0x7F-masked ring index
  are a deterministic multiplayer contract — if Rust's queue is
  deeper or different, multiplayer peer divergence is possible
  under heavy order spam.
- **Action-code enum**: Rust needs the SAME action codes to
  dispatch to the SAME packet types. If Rust uses its own enum,
  the numeric codes must match for replay / network compat.

---

## 3. Cross-reference: field offset audit

The DisplayClass constructor initializes fields through `+0x11E0`
(per my discovery report). The BandBox analysis extends the
field map with:

| Offset | Field | Init | Written in ctor? |
|--------|-------|------|-------------------|
| +0x11B3 | mode_flag_a | 0 | yes (zeroed) |
| +0x11B4 | mode_flag_b | 0 | yes |
| +0x11CF | bandbox_active | 0 | yes |
| +0x11D0 | drag_pending | 0 | yes |
| +0x11D4 | drag_start_x | 0 | yes |
| +0x11D8 | drag_start_y | 0 | yes |
| +0x11DC | last_end_x | 0 | yes (via param_1[0x477] = 0) |
| +0x11E0 | last_end_y | 0 | yes (via param_1[0x478] = 0) |
| +0x474 | selection_active | 0 | yes |
| +0x469 | placement_target_ptr | 0 | yes |
| +0x46A | placement_secondary_ptr | 0 | yes |
| +0x46B | placement_ordinal | 0xFFFFFFFF | yes (-1) |

Matches the constructor layout cleanly — no new fields beyond
`+0x11E0` surfaced during bandbox analysis.

---

## 4. Still-open follow-ups

1. **Enumerate action codes 0x0E, 0x0F, 0x14, 0x22, 0x25, 0x26,
   0x27-0x29, 0x3A, 0x41-0x48.** Each corresponds to a
   player-issuable order. Need to find the dispatch-to-command
   mapping (probably a static table in `.data` or a switch in
   `DetermineAction` at `0x692610`).

2. **Full Selection::DispatchMultiUnitOrder trace** — the single
   choke point for multi-unit orders. Understanding its contract
   is required before any Rust-side multi-unit order handling.

3. **`FUN_006AC840`** — the "second DisplayClass Constructor"
   labeled in Ghidra. It also writes to vtable_DisplayClass. Is
   it a post-init hook? A re-init called on map change? Worth a
   short decomp.

4. **BandBox_LeftDown** — not in Ghidra's labels. Should be a
   vtable slot for mouse-down. It initializes `drag_start_*` and
   sets `drag_pending = 1`. Finding it completes the state
   machine.

5. **Command packet layouts** — the 0x6F-byte command struct has
   opcode + unit/cell/house fields + padding. Enumerating each
   opcode's layout would be a sizable but well-scoped project.

---

## Sources

### Newly decompiled / re-read
- `0x4AC380` DisplayClass::BandBox_MouseMove
- `0x4AB9B0` DisplayClass::BandBox_LeftUp (full, including building
  placement branch)

### Raw memory
- `0x7E61E0, 32 bytes` — adjacent BufferStraw vtable (for MI
  correction)
- `0x7FFD08, 24 bytes` — BufferStraw COL (confirms adjacent-class
  identity)
- `0x00820148, 48 bytes` — type descriptor at `.?AVBufferStraw@@`

### Xref scans
- `get_xrefs_to(0x7E61E0)` → 11+ I/O class constructors, zero
  DisplayClass-adjacent code
- `get_xrefs_to(0x7E6114)` → exactly 2 hits, both DisplayClass
  constructor code
- `get_xrefs_to(0x4AAD30)` → 5 vtable slots across the display
  hierarchy, confirms it's a real inherited DisplayClass method

### Referenced docs
- `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` — previous survey;
  this report corrects §3 (MI claim) and fills in §4 for two
  methods
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` — inheritance
  hierarchy context
