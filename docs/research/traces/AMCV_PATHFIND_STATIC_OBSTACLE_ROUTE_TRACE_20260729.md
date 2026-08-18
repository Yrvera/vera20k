# AMCV Static-Obstacle Route-Construction Trace — 2026-07-29

**Scenario:** stock YR `[AMCV]`, normal owner, exact centre of flat clear Temperate cell `(50,50)`, ground Z, body facing east `0x40`, at rest; one intact enemy `[GAWALL]` overlay at `(55,50)`; one ordinary player Move to the exact centre of `(60,50)`. Scope is **route construction only** — passability inputs, the A* search, and both smoothing passes — up to the committed path queue. Drive execution (slot-1) and dynamic repathing (slot-3) are out of scope.

**Status:** **AMBER / MECHANISM DRIFT, OUTCOME AGREES.** The crux resolves cleanly and against the intuition that a Crusher can bulldoze walls: native `UnitClass::Can_Enter_Cell` returns hard-block code **7** for the intact `[GAWALL]`, because AMCV carries **no weapon at all** and the wall branch's first test is "do you have a weapon". Rust also rejects `(55,50)`, so the traversal result matches and the detour geometry is the same shape. Everything underneath it differs: the native wall decision is a `Crushable`/`CrusherAll`/weapon/warhead ladder producing codes 4/5/7, Rust is a static overlay bit; the native A* edge cost is a pure `costTable[can_enter_code]` lookup with no terrain or slope term, Rust multiplies in a land-type percentage and a cliff factor; and Rust's second smoothing pass is still provably dead.

**Verdict tally:** **PASS 10 · FAIL 8 · UNCHECKED 5 · NOT-IMPLEMENTED 2** (25 bounded rows). A PASS certifies only the named row.

---

## Scope, freshness, and evidence discipline

- Investigation only. No Rust, INI, or asset was edited; no cargo command was run; Ghidra access was strictly read-only (no rename, no comment, no `save_program`). This report is the sole written artifact.
- Tree read at `ce096b3f` (clean). Current source is authoritative over every prior trace cited below.
- Two prior traces cover adjacent ground: `AMCV_OBSTACLE_DETOUR_TRACE_20260527.md` (same mechanic, different coordinates) and `E2_STATIC_WALL_WALK_FEEL_RETRACE_20260728.md` (identical fixture geometry, Infantry mover). **Three of the 2026-05-27 findings are now stale and are corrected in this report** (§ Stale prior findings).
- The prompt located `path_smooth.rs` under `src/sim/movement/`. It actually lives at `src/sim/pathfinding/path_smooth.rs`; `src/sim/movement/movement_path.rs` is the caller.
- Native identities below are bound from vtable slot bytes, RTTI, and callsites — not from labels. Label drift found: `UnitClass__Can_Enter_Cell` is correctly named, but the Infantry counterpart at `0x0051BF90` currently carries the decompiler-generated name `FUN_0051bf90` while prior docs cite it as `InfantryClass::Can_Enter_Cell`.
- No native literal route, cost series, or smoothed output was executed against an oracle. Every such row is `UNCHECKED`; nothing was promoted from static plausibility.

### Native identity binding for the crux function

- `read_memory 0x007f5c6c` → `68 cc 80 00`: RTTI Complete Object Locator pointer at vtable−4, so the vtable base is **`0x007f5c70`**.
- `read_memory 0x0080cc68` → TypeDescriptor pointer `0x00842d80`; `read_memory 0x00842d88` → `.?AVUnitClass@@`. The vtable is **UnitClass**.
- `read_memory 0x007f5e18` → slot at `0x007f5e1c` holds `0x0073f0a0`; `0x007f5e1c − 0x007f5c70 = 0x1AC`. So **`UnitClass::Can_Enter_Cell @ 0x0073f0a0` occupies vtable slot `+0x1AC`**.
- `get_xrefs_to 0x0073f0a0` returns exactly one reference, `007f5e1c [DATA]` — the function is reached only through that slot.
- `decompile_function 0x00429A90` (`AStar_main_loop`) calls `(**(code **)(*param_4 + 0x1ac))(cell, dir, path_height, current_cell, flags)` inside the neighbour loop. **The A* passability call and the Unit slot are the same slot**, so this is the function that decides the fixture.

This is the Unit variant, not the Infantry one; the two bodies differ materially (§ Stage 2).

---

## Retail inputs

| Input | Source | Value |
|---|---|---|
| `[AMCV]` | `ini/rulesmd.ini:6969-7010` | `Speed=4`, `ROT=5`, `Crusher=yes`, `Weight=3.5`, `MovementZone=Normal`, `OmniCrushResistant=yes`, Drive CLSID `{4A582741-…}`, `Size=6`, **no `SpeedType=`**, **no `Primary=`/`Secondary=`** |
| `[GAWALL]` rules | `ini/rulesmd.ini:12022-12044` | `Wall=yes`, `Armor=concrete`, `Strength=300`, `Insignificant=yes`, **no `Crushable=`** |
| `[GAWALL]` art | `ini/artmd.ini:4122-4127` | `Foundation=1x1`, `ToOverlay=GAWALL`, `DamageLevels=3` |
| `[Clear]` ground | `ini/rulesmd.ini:30191-30199` | `Foot=100% Track=100% Wheel=100% Float=0% Hover=50% Amphibious=80% FloatBeach=0% Buildable=yes` |

Twelve land-type sections exist (`ini/rulesmd.ini:30191-30322`): Clear, Rough, Road, Water, Rock, Wall, Tiberium, Weeds, Beach, Ice, Railroad, Tunnel.

**AMCV has no weapon.** `[AMCV]` declares neither `Primary=` nor `Secondary=`. This is the single load-bearing fact for Stage 2.

---

## Stage 1 — Passability inputs for AMCV specifically

### 1a. The real `SpeedType` default is derived from `Crusher=`, not a constant

`[AMCV]` has no `SpeedType=` key, so the value comes from the constructor/ReadINI chain, not the INI.

`disassemble_bytes 0x007470d0` shows `UnitTypeClass::Constructor` pushing `EDI = -1` as the second argument to `TechnoTypeClass::Constructor @ 0x00710af0`; `disassemble_bytes 0x00711090` shows that constructor storing that stack argument into `[ESI+0x67c]`. So the raw constructor default is **`-1`**.

`disassemble_bytes 0x007121b8` binds the field: the `TechnoTypeClass::ReadINI` call at `0x007121e0` pushes string pointer `0x844504`, and `read_memory 0x844504` = `"SpeedType"`. **`TechnoTypeClass +0x67C = SpeedType`.**

The sentinel is resolved in `UnitTypeClass::ReadINI` before the INI read (`disassemble_bytes 0x007476c0`):

```asm
007476d3  MOV  EAX, [EDI+0x67c]      ; SpeedType
007476d9  CMP  EAX, -0x1
007476dc  JNZ  007476f1
007476de  MOV  DL,  [EDI+0xd28]      ; Crusher
007476e4  NEG  DL                    ; CF = (Crusher != 0)
007476e6  SBB  EDX, EDX              ; EDX = Crusher ? -1 : 0
007476e8  ADD  EDX, 0x2              ; EDX = Crusher ? 1 : 2
007476eb  MOV  [EDI+0x67c], EDX
007476f1  ...  CCINIClass__ReadSpeedType(section, "SpeedType", EAX)
```

`disassemble_bytes 0x00714cb0` + `read_memory 0x81bb58` = `"Crusher"` binds **`TechnoTypeClass +0xD28 = Crusher`**.

So: **a UnitType with no `SpeedType=` gets `Track` if `Crusher=yes`, `Wheel` if not.** AMCV is `Crusher=yes` → **SpeedType = 1 = Track**.

The enum is 8-wide (`decompile_function 0x0048e030`: `if (param_1 < 8) return g_SpeedTypeNameTable[param_1]`, table at `0x0081da58` per `disassemble_bytes 0x0048e030`). `decompile_function 0x00674000` (`RulesClass::ReadSpeedTypeLandTypeTable`) walks 12 land-type rows of 9 floats each (`pfVar4 += 9`, `while (pfVar4 < 0x89ebf4)`, base `0x0089ea44`) writing, in row order: `Foot` (`read_memory 0x0081dbd4` = `"Foot"`), `Track`, `Wheel`, `Hover`, a hardcoded `1.0` (Winged), `Float`, `Amphibious`, `FloatBeach`, then a `Buildable` bool byte in slot 8. **SpeedType = {0 Foot, 1 Track, 2 Wheel, 3 Hover, 4 Winged, 5 Float, 6 Amphibious, 7 FloatBeach}.**

Rust: `src/rules/object_type.rs:1071-1074` reads `SpeedType` and falls back to `SpeedType::default()`, which is `Track` (`src/rules/locomotor_type.rs:150-151`). `src/sim/movement/locomotor.rs:294` copies `obj.speed_type` onto the locomotor; `src/sim/world/world_commands.rs:79` reads it back with a second fallback to `Track`. **Outcome for AMCV is correct (`Track`), mechanism is not** — Rust hardcodes `Track` where gamemd computes it from `Crusher=`. Any stock `Crusher=no` vehicle omitting `SpeedType=` gets `Track` in Rust and `Wheel` in gamemd, which changes its `[Rough]`/`[Beach]`/`[Railroad]` percentages.

### 1b. Does the native path scorer consult crushability at all?

Yes, in **three** places, none of them a single "is crusher" predicate:

1. **`OverlayTypeClass +0x22D = Crushable`** in the wall branch of `Can_Enter_Cell` (§ Stage 2). Bound via `search_instructions ObjectTypeClass__ReadINI 0x22d` → `0x005f940a` pushes `0x832bd8`; `read_memory 0x832bd8` = `"Crushable"`. Default is **false**: `disassemble_bytes 0x005f7090` shows `XOR EBX, EBX` at `0x005f70a1` and `disassemble_bytes 0x005f7160` shows `MOV byte ptr [EBP+0x22d], BL` at `0x005f717c`. `OverlayTypeClass::Constructor` does not touch `+0x22d` (`search_instructions OverlayTypeClass__Constructor 0x22d` → 0 matches).
2. **`TechnoTypeClass +0xD28 = Crusher`**, both as the `SpeedType` default source (§ 1a) and as one disjunct of the wall-branch guard.
3. **`TechnoClass::CanCrushCheck`** in the cell-occupant loop of `Can_Enter_Cell` (`decompile_function 0x0073f0a0`, two callsites) — the dynamic-unit path, not exercised by a static wall.

Rust's ground A* consults crushability once: `options.mover_is_crusher` exempts a cell from the entity-block cost multiplier (`src/sim/pathfinding/core.rs:1280`). It plays no part in terrain or overlay passability. `mover_is_crusher = e.regular_crusher || e.omni_crusher` (`src/sim/world/world_commands.rs:108`), and `regular_crusher` now does come from the `Crusher=` key (`src/rules/object_type.rs:1103`, covered by `object_type_parses_regular_crusher_for_amcv_fixture` at `:2335`).

### 1c. `MovementZone` binding

`disassemble_bytes 0x00716040` shows `TechnoTypeClass::ReadINI` pushing `0x8431c8`; `read_memory 0x8431c8` = `"MovementZone"`. **`TechnoTypeClass +0x5B4 = MovementZone`.** `decompile_function 0x00474e40` (`CCINIClass__ReadMovementZone`) scans `g_MovementZone_NameTable` while `ptr < 0x81babc`; `read_memory 0x0081ba70` plus `read_memory 0x0081bad0` resolve the 13 entries:

`0 Normal · 1 Crusher · 2 Destroyer · 3 AmphibiousDestroyer · 4 AmphibiousCrusher · 5 Amphibious · 6 Subterannean · 7 (Infantry) · 8 InfantryDestroyer · 9 Fly · 10 Water · 11 WaterBeach · 12 CrusherAll`

Rust's `MovementZone` enumerates the same 13 names in the same order (`src/rules/locomotor_type.rs`, list at `:330-345`). **`MovementZone=Normal` = 0**, and index 6 (`Subterannean`) is the TS-legacy dormant zone ENGINE.md flags — note that `TechnoTypeClass::ReadINI` caches `MovementZone == 6` into a derived bool at `+0xD2C` (`disassemble_bytes 0x00716040`, `CMP EAX,0x6 / SETZ DL / MOV [EBP+0xd2c],DL`), i.e. the Subterranean flag is computed on every type load but its consumers are TS legacy. Active in YR: **no**.

---

## Stage 2 — The wall cell result (the crux)

`decompile_function 0x0073f0a0`, overlay slice, reduced:

```text
if (cell.OverlayIndex != -1):
    ot = OverlayTypeClass_Array[cell.OverlayIndex]
    if ot.Crate && !IsPlayerControl() && GameMode == 0: return 7
    if ot.Wall:                                                  # ot+0x2A8
        if  (ot.Crushable == 0 || (type.Crusher == 0 && !HasWeaponAbility()))   # ot+0x22D, type+0xD28
        and (ot.Wall == 0 || type.MovementZone != 12):                          # 12 = CrusherAll
            if !vt[0x2AC]():   return 7          # <-- "do you have any weapon at all"
            w = vt[0x3F8]()                      # GetWeapon(0)
            if !w.warhead.Wall and (!w.warhead.Wood or ot.Armor != 6): return 7
            if !Is_Ally(): code = max(code, 5); goto occupants
        else:
            if !Is_Ally(): goto occupants        # crusher/CrusherAll bypass: wall costs nothing
        code = max(code, 4)                      # allied wall
```

Bindings: `decompile_function 0x005fe770` shows `OverlayTypeClass::ReadINI` writing `+0x2A8` from `CCINIClass__ReadBool(section, DAT_0081ac58, …)`, and `read_memory 0x0081ac58` = `"Wall"`. `+0x2A0 = DamageLevels` (same decompile, `s_DamageLevels` read from the art section at `+0x1F8`). `+0x9C = Armor`.

### Walking the fixture through it

1. `[GAWALL]` has no `Crushable=` key, and the `ObjectTypeClass` constructor default is **false** (§ 1b). So `ot.Crushable == 0` → the **first conjunct is true regardless of `Crusher=yes`**.
2. `ot.Wall != 0` and AMCV's `MovementZone = Normal = 0 ≠ 12` → the **second conjunct is true**.
3. Both conjuncts true → the weapon ladder runs. First rung is vtable `+0x2AC`. `read_memory 0x007f5f1c` → `0x00701120`; `decompile_function 0x00701120`:

```c
piVar1 = (int *)(**(code **)(*param_1 + 0x3f4))();
if ((piVar1 != (int *)0x0) && (*piVar1 != 0)) return 1;
return 0;
```

It fetches a weapon slot and demands both a non-null `WeaponStruct` **and** a non-null `WeaponTypeClass` inside it. `read_memory 0x007f6068` → slot `+0x3F8` = `0x0070e140`, `decompile_function 0x0070e140` = `TechnoClass::GetWeapon`, which returns `NULL` for index `-1` and otherwise returns whatever the type's weapon array holds.

**AMCV declares no `Primary=` and no `Secondary=`, so `+0x2AC` returns 0 and `Can_Enter_Cell` returns 7 — Impassable — before the warhead is ever consulted.**

### What this means

- **An MCV cannot path through a wall, and `Crusher=yes` is irrelevant to that.** The only two native wall bypasses for a vehicle are (a) the overlay itself being `Crushable=yes` while the mover is a Crusher or has the weapon ability, and (b) `MovementZone=CrusherAll`. Stock `[GAWALL]` satisfies neither, and no stock unit uses `MovementZone=CrusherAll`.
- The result code is **7**, not 5. Code 5 (`EnemyBlock`) is only reachable for a mover that *has* a weapon whose warhead sets `Wall=` (or `Wood=` against an `Armor=6` overlay). AMCV never gets that far. This distinction matters downstream: 5 is a *soft* block that A* expands at 20× cost, 7 is a hard reject.
- **The Unit variant has no wall damage-state gate.** `search_instructions UnitClass__Can_Enter_Cell 0x2a0` and `… 0x11e` both return **0 matches** across 953 scanned instructions. The Infantry variant at `0x0051BF90` *does* gate on `(cell[0x11e] >> 4) != ot.DamageLevels` (`decompile_function 0x0051BF90`). So for vehicles the wall branch fires at every damage level including a fully-damaged wall; for infantry it stops firing once the wall's damage state reaches `DamageLevels`. This is a genuine class asymmetry, not a decompiler artifact.

### Rust side

`(55,50)` is rejected as a static grid cell. `src/map/resolved_terrain.rs:1531,1551-1553` sets `overlay_blocks = true` for any `Wall=yes` overlay lacking `Land=Road`, and `:596-604` folds that into the cell's non-walkable state; the A* neighbour gate reads that bit. There is no weapon, warhead, `Crushable`, `CrusherAll`, ownership, or result-code mechanism anywhere in the path.

Separately, `src/sim/pathfinding/passability.rs:115-122` carries a native-shaped 13×8 `MOVEMENT_ZONE_PASSABILITY` matrix whose `Wall` column reads `Normal=2, Crusher=2, Destroyer=1`. That matrix's `Destroyer` row would let a `MovementZone=Destroyer` vehicle enter a wall; the verified native wall bypass keys on `CrusherAll` (12), not `Destroyer` (2). On this fixture the matrix is not consulted — `src/sim/pathfinding/core.rs:1391-1403` routes non-water movers to the shared `PathGrid` walkable bit — so the discrepancy is recorded but not decisive here.

**Bounded traversal result agrees (both hard-block). The mechanism does not.**

---

## Stage 3 — A* neighbour order, tie-breaking, and cost

### Native neighbour table and tie row — exact match

`read_memory 0x007e3774` gives `g_CellNeighborOffsets_8Dir` as eight signed cell-array deltas with row stride `0x200`:

| idx | offset | direction |
|---:|---:|---|
| 0 | −512 | N |
| 1 | −511 | NE |
| 2 | +1 | E |
| 3 | +513 | SE |
| 4 | +512 | S |
| 5 | +511 | SW |
| 6 | −1 | W |
| 7 | −513 | NW |

`read_memory 0x0081870c` (64 bytes) yields both tables the loop uses:

- **Cost table `0x0081870c[0..7]`** = `1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0` — indexed by the `Can_Enter_Cell` result code.
- **Direction tie row `0x0081872c[0..7]`** = `0.001, 0.005, 0.002, 0.006, 0.003, 0.007, 0.004, 0.008`.

Rust's `NEIGHBORS` (`src/sim/pathfinding/core.rs:388-397`) is `N, NE, E, SE, S, SW, W, NW` and `DIR_TIEBREAK` (`:371-380`) is `[1,5,2,6,3,7,4,8]`. **Both the direction order and the tie row match exactly, 1:1, at a 1000× scale.** The comment at `:367` naming `0x0081872c` is correct.

`AStar_main_loop` expands **nine** edges (`do { … } while (iStack_44 < 9)`), index 8 being the `TubeClass` jump priced at Chebyshev cell distance. Rust mirrors this at `core.rs:1338-1369` with `TUBE_DIR_TIEBREAK = 9`. Not exercised on this fixture.

### Native edge cost — no terrain, no slope, no diagonal upcharge

From `AStar_main_loop`:

```c
fStack_28 = AStar_compute_edge_cost(cur, nbr, is_alt_layer, can_enter_code, mover)
          * *(float*)(pathfinder + 4)
          + tie[dir];
```

and `decompile_function 0x00429830`:

```c
base = costTable[can_enter_code];                        // 0x0081870c
if (can_enter_code == 2) { 10-hop blocker walk; base = 4.0; if (urgency==2) base = 1000.0; }
if (dest.flags & 0x40000) base *= 4.0;                   // temporary search marker
if (is_alt_layer && pathfinder[1]) { bridge-flank check → ×1.0 / ×2.0 / ×10.0 }
return base;
```

Three consequences for a flat clear ground fixture:

- **Diagonals cost exactly what cardinals cost.** `AStar_compute_edge_cost` takes no direction argument; the only per-direction term is the 0.001–0.008 tie row. Rust's `STEP_COST` is likewise uniform (`core.rs:1264-1267`). **PASS.**
- **There is no land-type or slope term in the native edge cost.** The `g_SpeedType_LandType_Table` value is used inside `Can_Enter_Cell` only as a *binary* gate (`== 0.0 → return 7`), never as a weight. Rust divides by a land-type percentage (`core.rs:1268-1272`) and multiplies by `CLIFF_COST_MULTIPLIER = 4` on any height change (`:1274-1277`). Rust's own `src/sim/pathfinding/terrain_speed.rs:1-6` states the original applies terrain speed as a runtime modifier "not an A* cost weight" — the A* code contradicts its own module header.
- **The soft-block codes are applied differently.** Native *replaces* the base with `costTable[code]` (60/20/8 for codes 4/5/6). Rust *multiplies* `STEP_COST` by `CODE5_MULT_ENEMY = 20` / `CODE6_MULT_STATIONARY_ALLY = 8` (`core.rs:1283-1295`) and additionally exempts the goal cell and any crusher — two gates with no native counterpart. Code 4 (friendly wall, native 60.0) has no Rust multiplier at all.

### Node arithmetic

`decompile_function 0x0042a460` reads as if `g` and `f` were truncated to `int`, but that is Ghidra typing the node as `int*`. `disassemble_bytes 0x0042a523` shows the real stores:

```asm
0042a529  FLD   float ptr [ESP+0x20]   ; edge cost incl. tiebreak
0042a52d  FADD  float ptr [EBX+0x4]    ; parent g
0042a530  FSTP  float ptr [ESI+0x4]    ; g  (float32)
...
0042a58d  FSTP  double ptr [ESP]       ; dx*dx + dy*dy
0042a590  CALL  0x004cac40             ; sqrt
0042a595  FADD  float ptr [ESI+0x4]    ; + g  →  f  (float32)
```

So **native `g`/`f` are float32 and the heuristic is plain Euclidean distance in cells**, `sqrt(dx² + dy²)`, added to `g`. Rust uses `euclidean_heuristic` scaled by `STEP_COST` (`core.rs:2084-2090`) with integer `g` — the required deterministic substitution. Two residuals: the pathfinder scale at `pathfinder+4` was not read, so the native h:g ratio is `UNCHECKED`; and float32 accumulation can reorder 0.001-magnitude ties on long paths in a way integer math cannot. Neither is decisive over ten steps.

### Exact literal Rust route for this fixture (source reduction)

Inputs: ground layer; `terrain_cost = 100` (`[Clear] Track=100%` → `cost_for_speed_type(Track) = Some(100)`, `src/rules/terrain_rules.rs:49-59`, asserted at `:378`) so `step_cost = STEP_COST = 1000`; equal heights so no cliff factor; no entity-block entries (the wall is static terrain, not an entity); no marker overlay.

```text
(50,50)
  E  (51,50)   g =  1002
  E  (52,50)   g =  2004
  E  (53,50)   g =  3006
  E  (54,50)   g =  4008
  NE (55,49)   g =  5013
  SE (56,50)   g =  6019
  E  (57,50)   g =  7021
  E  (58,50)   g =  8023
  E  (59,50)   g =  9025
  E  (60,50)   g = 10027
```

Direction IDs `[2,2,2,2,1,3,2,2,2,2]`; accumulated scaled `g = 10027`.

**Why north.** The two detours are exactly symmetric in geometry and both reach `(56,50)` at `g = 6019`. They are separated only by the tie row: the first detour frontier node is `(55,49)` at `5013` (NE adds 5) versus `(55,51)` at `5014` (SE adds 6). The north node pops first, writes `ground_from[(56,50)]`, and the southern candidate's `tentative_g < g_array[n_idx]` test fails on equality (`core.rs:1311`). **The detour side is decided entirely by `DIR_TIEBREAK[NE] < DIR_TIEBREAK[SE]`.** Since that row is bit-identical to `0x0081872c` and the native step cost is likewise direction-uniform, gamemd is expected to detour north as well — but that expectation was not executed, so the **native literal route is `UNCHECKED`**.

---

## Stage 4 — Both smoothing passes

`AStar_main_loop` calls, unconditionally on success: `AStar_reconstruct_path` → `Path_smooth_corners` → `Path_optimize_straight_segments` (`decompile_function 0x00429A90`, tail at `LAB_0042a3de`). Both prior-doc addresses are confirmed.

### Pass 1 — `Path_smooth_corners @ 0x0042B210`

`decompile_function 0x0042B210`. The trigger is `uVar6 = (cur − prev) & 7; if ((uVar6 == 2 || uVar6 == 6) && prev != -1 && prev != 8 && cur != 8)` — a 90° turn between two adjacent steps. Critically, in the non-matching branch:

```c
iVar8 = 1; uVar7 = uVar2;
if ((uVar2 & 1) == 0) uVar7 = 0xffffffff;   // cardinal → reset prev_dir to -1
```

Direction indices are even for cardinals, so **after any cardinal step the previous direction is cleared and cannot anchor a smoothing pair**. Only diagonal→diagonal pairs collapse. Rust reproduces exactly this with `if !is_diagonal_dir(d0) { i += 1; continue; }` (`src/sim/pathfinding/path_smooth.rs:110-113`, `:196-199`), and its comment at `:106-109` names the mechanism correctly. **PASS on the trigger condition.**

### Pass 1's validator — `Path_smooth_single_segment @ 0x0042B420`

`decompile_function 0x0042B420`. Per candidate replacement cell it evaluates:

```c
iVar5 = (**(code **)(*param_1 + 0x1ac))(cell, mid_dir, height, 0, 1);   // Can_Enter_Cell
if ( iVar5 != 0                                    // must be EXACTLY 0
  || (cell.flags & 0x40000) != 0                   // temporary A* search marker
  || MapClass__Get_Slope_Cost_At_Cell(&coord, house)
       * FootClass__Get_Slope_Speed_Factor() >= 1.0 )   // slope gate
    reject;
```

with bridge-deck height carry-forward (`if (height − cellHeight != 4 || !(cell.flags & 0x100)) height = cellHeight;`).

Three things Rust does not have. First, native demands `Can_Enter_Cell == 0` — **strictly stronger than the A* gate**, which accepts 0–6. A cell A* was happy to route through at code 5 or 6 can still be refused as a smoothing shortcut. Second, the `0x40000` search-marker bit. Third, a real slope cost × slope speed-factor product. Rust's validator is the closure at `src/sim/movement/movement_path.rs:261-278`: static walkability, plus "is there an entity-block entry", plus "is it a marker cell and not the goal". That is an approximation of the first and third checks and a re-implementation of the second at a different granularity. **FAIL on mechanism.**

### Pass 1 outcome on this fixture

Raw directions are `[…, NE(1), SE(3), …]`. `dir_diff(1,3) = 2` and `d0` is diagonal, so both engines enter the smoothing branch. `midpoint_dir(1,3) = 2 = E`; the replacement cell is `(54,50) + E = (55,50)` — **the wall**. Rust's `walkable(55,50)` is false, native's `Can_Enter_Cell` returns 7 ≠ 0. Both reject; the `NE,SE` pair survives. **The path is unchanged by pass 1 in both engines.** Rust's final route equals the raw row above.

### Pass 2 — `Path_optimize_straight_segments @ 0x0042B7F0`

`decompile_function 0x0042B7F0` confirms an active mechanism: a 20-step window (`if (0x13 < iVar13) break;`), running signed drift accumulators compared against a running max, `Path_Find_Split_Anchor @ 0x0042BCA0` to choose the split index, `Path_Reroute_Straight_Line @ 0x0042BE20` to rewrite the segment, a tail pass for the final open segment, and a final compaction that strips `0xFFFFFFFE` deletion sentinels and rewrites the path length.

**Rust's pass 2 is still dead — re-verified against current source on 2026-07-29.** `find_drift_segment` (`src/sim/pathfinding/path_smooth.rs:352-398`) accumulates `cum_dx/cum_dy` as `path[i+1] − path[i]` summed from `start`, which telescopes to exactly `path[i+1] − path[start]` — the *same vector* it then assigns to `ideal_dx/ideal_dy` at `:375-376`. The cross product at `:388` is therefore `ideal × ideal ≡ 0`, `drift_sq = 0`, and the `drift_sq > dist_sq * 1` test at `:392` can never fire. The function always returns `None`, so `optimize_path` breaks on its first iteration (`:288-290`) and `reroute_segment` (`:402`) is unreachable. **The 2026-07-28 claim holds unchanged.**

`MAX_OPTIMIZE_STEPS = 20` (`:245`) does match the native 20-step window — the constant is right, the predicate that would use it is not.

---

## Stage 5 — Terrain cost model, and whether this fixture exercises it

Commit `2e73ac1e` ("sim+rules: native cliff/slope speed model") replaced an invented `SlopeClimb`/`SlopeDescend` pair with four verified `[General]` coefficients (`TrackedUphill`/`TrackedDownhill`/`WheeledUphill`/`WheeledDownhill`), traced from `DriveLocomotionClass::Process_Movement 0x004B2630`. That model lives in `src/sim/pathfinding/terrain_speed.rs` and is a **runtime per-tick speed multiplier**, selected by `SpeedType == Track` and by the sign of the destination-cell height delta.

**Path cost and movement cost do not use the same model, and neither matches gamemd's split.**

- Native: land type and slope appear in *movement* (the `Process_Movement` coefficients) and in the smoothing *validator* (`Get_Slope_Cost_At_Cell × Get_Slope_Speed_Factor`). They appear in *A\* cost* **not at all**; the only terrain input to A* is the binary `g_SpeedType_LandType_Table[...] == 0.0 → 7` gate inside `Can_Enter_Cell`.
- Rust: `core.rs:1268-1277` puts both a land-type percentage divisor **and** a `×4` height-change factor into the A* edge cost, and the ordinary player Move supplies the cost grid (`src/sim/world/world_commands.rs:256` passes `self.terrain_costs.get(&info.speed_type)`).

**On this fixture neither term fires.** Flat clear Temperate gives `terrain_cost = 100` for `Track`, so `step_cost` stays at `STEP_COST`, and all heights are equal, so `CLIFF_COST_MULTIPLIER` is skipped. The drift is real and recorded, but it does not move a single cell here. It would move cells the moment a route crosses `[Rough]`, `[Road]`, `[Railroad]`, or any ramp — i.e. on most retail maps, for any route long enough to have a choice.

Rust also clamps the combined runtime modifier to `[0.3, 1.2]` (`terrain_speed.rs:34-37`). Those clamps are execution-domain (slot-1's row) and were not adjudicated here, but `COMBINED_MIN = 0.3` has no cited native source in the module and reads as a VERA-internal floor.

---

## Stage 6 — Slope/cliff and zone gates; Rust-invented gates

**Does the native search use `MovementZone` to reject cells?** Only in one narrow place. `UnitClass::Can_Enter_Cell` references `MovementZone` (`type+0x5B4`) exactly once — the `!= 12` (`CrusherAll`) test in the wall branch. Everything else is decided by:

- `g_SpeedType_LandType_Table[SpeedType + LandType*9] == 0.0 → return 7` (the ground-passability gate; suppressed on the alt/bridge list),
- the `MovementRestrictedTo` gate, and
- the locomotor hook `FootClass__LocomotorPassabilityCheck` (returns 7 to hard-block).

**`MovementRestrictedTo` is a live but dormant gate.** `disassemble_bytes 0x00747820` shows `UnitTypeClass::ReadINI` reading key pointer `0x845d64`; `read_memory 0x845d64` = `"MovementRestrictedTo"`, stored to `+0xDFC`. `disassemble_bytes 0x007470d0` shows the constructor defaulting `+0xDFC` to `-1`, and `Can_Enter_Cell`'s first block is `if (type[0xdfc] != -1) { … reject cells whose LandType differs … }` with a special case for LandType 10 that inspects the isometric tile's ramp fields. Six occurrences exist in `ini/rulesmd.ini`; `[AMCV]` is not one of them, so the gate is **off for this fixture**. Rust has no equivalent — **NOT-IMPLEMENTED**, dormant for AMCV, live for the six types that set it.

**Rust gates without a native counterpart, in `sim/`, unlabelled:**

| Rust gate | Location | Native counterpart |
|---|---|---|
| Land-type percentage divides the A* edge cost | `core.rs:1268-1272` | none — native uses it only as a `== 0` reject |
| `CLIFF_COST_MULTIPLIER = 4` on any height change | `core.rs:1274-1277` | none in `AStar_compute_edge_cost` |
| Goal cell exempt from entity-block cost | `core.rs:1280` | none |
| Crusher exempt from entity-block cost | `core.rs:1280` | none (native crush handling is inside `Can_Enter_Cell`) |
| `MOVEMENT_ZONE_PASSABILITY[Destroyer][Wall] = passable` | `passability.rs:115-122` | native wall bypass keys on `CrusherAll` (12), not `Destroyer` (2) |
| `find_drift_segment` degenerate predicate | `path_smooth.rs:352-398` | native pass 2 is an active split/reroute |

Of these, only the last two are not simply extra cost terms; the first four all bias route selection whenever the terrain is non-uniform.

**Zone precheck / retry.** The 2026-05-27 trace recorded a FAIL because the player move path passed `zone_grid: None`. Current source passes `self.zone_grid.as_ref()` (`src/sim/world/world_commands.rs:268`), so the plumbing is present. Whether the grid is `Some` at runtime, and whether Rust reproduces the native five-attempt hierarchy retry in `AStar_pathfind_search @ 0x0042C900`, was not executed this session — **UNCHECKED**.

---

## Stage verdicts

| # | Bounded row | Verdict | Result |
|---:|---|---|---|
| 1 | Retail AMCV / GAWALL / `[Clear]` input bindings | PASS | Stock keys, wall flag, damage levels, and ground percentages all read correctly. |
| 2 | `SpeedType` resolution with no INI key | FAIL | Native derives `Crusher ? Track : Wheel`; Rust hardcodes `Track`. Same value for AMCV, wrong for any non-crusher vehicle omitting the key. |
| 3 | `SpeedType` enum ordering (8 entries, Track = 1) | PASS | `SpeedType__ToName` bound `<8`; ground-table row order Foot/Track/Wheel/Hover/Winged/Float/Amphibious/FloatBeach reproduced in Rust. |
| 4 | `MovementZone` enum ordering (13 entries, Normal = 0, CrusherAll = 12) | PASS | Native name table and Rust enum agree entry-for-entry. |
| 5 | Crushability consulted by the native path scorer | FAIL | Native reads `OverlayType.Crushable`, `TechnoType.Crusher`, and `CanCrushCheck`; Rust's A* uses one `mover_is_crusher` cost exemption and nothing in passability. |
| 6 | A* passability vtable slot binding (`+0x1AC` → `UnitClass::Can_Enter_Cell`) | PASS | RTTI + slot bytes + sole data xref + `AStar_main_loop` callsite all agree. |
| 7 | Intact-GAWALL traversal result for weaponless AMCV | PASS | Native returns 7; Rust hard-blocks `(55,50)`. |
| 8 | Wall classification mechanism and result code | FAIL | Rust static overlay bit omits `Crushable`, `Crusher`, `CrusherAll`, `GetWeapon`, warhead `Wall`/`Wood`, ownership, and codes 4/5/7. |
| 9 | Vehicle wall branch independent of wall damage state | PASS | Native Unit variant has no `DamageLevels` gate (0 matches for `0x2a0`/`0x11e`); Rust likewise blocks at every state. |
| 10 | 8-direction neighbour order and offsets | PASS | `0x007e3774` = N, NE, E, SE, S, SW, W, NW; identical to Rust `NEIGHBORS`. |
| 11 | Direction tie-break row | PASS | `0x0081872c` = 0.001…0.008 in the same slot order as Rust `[1,5,2,6,3,7,4,8]`. |
| 12 | Ninth edge (tube, index 8) present and priced separately | PASS | Both implement it; not exercised here. |
| 13 | Uniform cardinal/diagonal step cost | PASS | `AStar_compute_edge_cost` takes no direction argument; Rust `STEP_COST` is uniform. |
| 14 | Edge-cost model (`costTable[code]` vs multiplicative) | FAIL | Native replaces base with 1/1000/1/1/60/20/8/10000; Rust multiplies `STEP_COST` and has no code-4 term. |
| 15 | Land-type percentage in A* cost | FAIL | Rust-only weight; native uses land type only as a `== 0` reject. Not exercised on flat Clear. |
| 16 | Height-change cost factor in A* | FAIL | Rust-only `×4`; no native counterpart in the edge-cost function. Not exercised (flat). |
| 17 | Ground diagonal corner legality | PASS | Both test only the diagonal destination; flank checks are bridge-only in both. |
| 18 | `g`/`f` numeric domain and heuristic scale | UNCHECKED | Native `g`/`f` are float32 with a raw-cell Euclidean `h`; Rust is scaled integer. Pathfinder scale at `pathfinder+4` not read. |
| 19 | Literal native detour side and cell series | UNCHECKED | Rust route and cost are an exact source reduction; gamemd was not executed on this fixture. |
| 20 | Smoothing pass 1 trigger (diagonal anchor only) | PASS | Native clears `prev_dir` after every cardinal; Rust gates on `is_diagonal_dir(d0)`. |
| 21 | Smoothing pass 1 validator | FAIL | Native requires `Can_Enter_Cell == 0`, no `0x40000` marker, and slope-cost × slope-factor `< 1.0`; Rust uses a walkability closure. |
| 22 | Smoothing pass 1 outcome for this fixture | PASS | Both reject the E midpoint `(55,50)`; `NE,SE` survives; path unchanged. |
| 23 | Straight-segment optimization (pass 2) live | FAIL | Rust `find_drift_segment` cross product is identically zero — re-verified 2026-07-29; pass 2 never fires. |
| 24 | Native pass-2 split-anchor + reroute mechanism | NOT-IMPLEMENTED | `Path_Find_Split_Anchor` / `Path_Reroute_Straight_Line` / deletion-sentinel compaction have no Rust counterpart. |
| 25 | `MovementRestrictedTo` land-type gate | NOT-IMPLEMENTED | Native gate at `type+0xDFC`, default `-1`; six stock types set it; AMCV does not. |

---

## Top root findings

1. **The MCV-through-walls question is settled, and the reason is "no weapon", not "not a crusher".** Native `Can_Enter_Cell`'s wall branch tests `vtable+0x2AC` — "do you own any weapon at all" — before it looks at the warhead. `[AMCV]` declares no `Primary=`, so the answer is no and the function returns 7. `Crusher=yes` never enters the decision, because the crusher bypass additionally requires the *overlay* to be `Crushable=yes`, and `[GAWALL]` is not (`ObjectTypeClass` default false, verified from `XOR EBX,EBX` in the constructor). Rust reaches the same block from a static overlay bit; the outcome is right and the mechanism is absent.
2. **The detour side is decided purely by the direction tie row, and that row is bit-identical.** Native `0x0081872c` = `0.001…0.008` maps 1:1 onto Rust's `[1,5,2,6,3,7,4,8]`, and the neighbour offsets at `0x007e3774` are in the same N/NE/E/SE/S/SW/W/NW order. Since the native step cost is direction-uniform, the north detour Rust computes is the expected native answer — but it remains UNCHECKED until executed.
3. **Rust prices terrain into A\*; gamemd does not.** `AStar_compute_edge_cost` is a pure `costTable[can_enter_code]` lookup (plus the code-2 blocker walk, the `0x40000` marker `×4`, and a bridge-only flank multiplier). Rust adds a land-type divisor and a `×4` height-change factor on the ordinary player Move path. Flat clear Temperate hides this completely; `[Rough]`, `[Road]`, `[Railroad]`, and every ramp expose it. Rust's own `terrain_speed.rs` header already states the correct native rule and the A* code contradicts it.
4. **The native smoothing validator is strictly stricter than the A\* gate.** A* admits `Can_Enter_Cell` codes 0–6; `Path_smooth_single_segment` admits only code 0, and additionally rejects search-marked cells and cells whose slope cost × slope speed factor reaches 1.0. Rust's closure is a walkability test with two ad-hoc filters. Same answer at a wall, different answer wherever a shortcut crosses a soft-blocked or sloped cell.
5. **Rust's second smoothing pass is still a no-op, one year of commits later.** `find_drift_segment` compares a vector against itself, so the cross product is always zero and `reroute_segment` is dead code. Native pass 2 is a live 20-step split-and-reroute with sentinel compaction. Every avoidable bend the native optimizer would straighten is retained.

**Frequency clause.** The route-construction path adjudicated here runs on *every* Move order whose direct line is not clear — many times per minute once an army exists, and on the player's very first MCV drive on most retail maps, where the blocker is usually terrain (cliff edge, tree cluster, ore, water inlet) rather than a wall. The specific "intact enemy wall in the straight line" variant is rare in the opening minutes and common mid-game when a second MCV or a redeploy crosses a walled perimeter — call it a handful of times in a 30–60 minute match. But the wall branch, the tie row, the edge-cost model, and both smoothing passes are the *same code* for every static blocker, so rows 14–16, 21, and 23 are high-frequency drift wearing a low-frequency fixture. Rows 2, 5, 8, 24, and 25 are the ones that only bite outside this fixture.

---

## Stale prior findings, corrected

Three findings in `AMCV_OBSTACLE_DETOUR_TRACE_20260527.md` no longer hold against current source:

- **"AMCV speed is multiplied by 3 before movement."** `resolve_move_info` (`src/sim/world/world_commands.rs:86-89`) has no deployable multiplier; base speed is `ra2_speed_to_leptons_per_second(o.speed)` times the locomotor multiplier only.
- **"`mover_is_crusher` is not derived from `Crusher=yes`."** It now is — `src/rules/object_type.rs:1103` parses the key and `world_commands.rs:108` ORs it with `omni_crusher`.
- **"The player move path passes `zone_grid: None`."** It now passes `self.zone_grid.as_ref()` (`world_commands.rs:268`). Whether that grid is populated at runtime, and whether the five-attempt retry exists, remains UNCHECKED.

---

## Smallest decisive follow-up

**One executable check settles the largest open row.** Emulate `AStar_compute_edge_cost @ 0x00429830` over the eight `Can_Enter_Cell` result codes with `pathfinder+0x3c = 0` and a plain ground destination cell, and read `pathfinder+4` from the same structure, then assert the resulting cost series against Rust's `STEP_COST × code-multiplier` table in a `#[test]` beside `core_tests.rs`. That converts rows 14 and 18 from FAIL/UNCHECKED into a named machine-derived check, and it is the prerequisite for deciding whether the land-type divisor and `CLIFF_COST_MULTIPLIER` should be deleted from `core.rs` or moved to the runtime speed path where `terrain_speed.rs` already says they belong.

## Fresh read-only Ghidra calls made this session

`decompile_function` — `0x0042B210`, `0x0042B7F0`, `0x0042B420`, `0x0073F0A0`, `0x0051BF90`, `0x0070E140`, `0x00701120`, `0x00429A90`, `0x00429830`, `0x0042A460`, `0x00474E40`, `0x00476FC0`, `0x0048E030`, `0x005FE770`, `0x00674000`, `0x007470D0`.
`disassemble_bytes` — `0x0042A523`, `0x0048E030`, `0x005F7090`, `0x005F7120`, `0x005F7160`, `0x005F93F0`, `0x00710AF0`, `0x00711090`, `0x007110C8`, `0x007121B8`, `0x00714CB0`, `0x00716040`, `0x007470D0`, `0x007476C0`, `0x00747820`.
`read_memory` — `0x007E3774`, `0x007F5C6C`, `0x007F5E18`, `0x007F5F1C`, `0x007F6068`, `0x0080CC68`, `0x0081870C`, `0x0081AC58`, `0x0081BA70`, `0x0081BAD0`, `0x0081DA58`, `0x0081DBD4`, `0x00832BD8`, `0x008431C8`, `0x00842D88`, `0x00844504`, `0x00845D64`.
`search_instructions` — scoped to `TechnoTypeClass__ReadINI`, `UnitTypeClass__ReadINI`, `UnitTypeClass__Constructor`, `TechnoTypeClass__Constructor`, `ObjectTypeClass__ReadINI`, `ObjectTypeClass__Constructor`, `OverlayTypeClass__Constructor`, `OverlayTypeClass__ReadINI`, `UnitClass__Can_Enter_Cell`.
`get_xrefs_to` — `0x0073F0A0`. `search_functions` — `Can_Enter_Cell`, `Path_`, `ReadINI`, `SpeedType`, `UnitTypeClass`, `TechnoTypeClass__C`, `ObjectTypeClass__C`, `OverlayTypeClass`, `AStar_compute_edge_cost`.

No Ghidra mutation was performed. Findings that clear the VERIFIED bar (UnitClass vtable `+0x1AC`, `TechnoType +0x5B4`/`+0x67C`/`+0xD28`/`+0xDFC`, `OverlayType +0x22D`/`+0x2A0`/`+0x2A8`, the `Crusher`-derived SpeedType default, the two constant tables at `0x0081870C`/`0x0081872C`, and the neighbour table at `0x007E3774`) are **not** written back this session because four sibling agents hold concurrent read sessions and mutations are single-writer. They should be labelled by whoever next owns write access.
