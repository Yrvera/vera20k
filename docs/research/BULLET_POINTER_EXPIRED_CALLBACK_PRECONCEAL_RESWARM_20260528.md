# Bullet Pointer-Expired Callback Pre-Conceal Cleanup - Re-swarm Research Report

**Address(es):** `BulletClass` pointer-expired callback body `0x004684E0..0x004685C6`, `BulletClass::UpdateTarget @ 0x00468430`, `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::UnInit @ 0x005F65F0`, `MapClass::Get_CellClass @ 0x005657A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the active YR `BulletClass` vtable `+0x28` pointer-expired callback reached by `ObjectClass::UnInit -> Detach_From_All_Lists` before target conceal/alive clear, including target invalidation, cell fallback, map-editor and high-flying gates, compared/written Bullet fields, and Rust projectile-target cleanup handoff.
**Non-Scope:** full `Detach_From_All_Lists` listener roster, full `BulletClass::AI` homing math, warhead detonation damage, projectile spawn implementation, all non-destruction limbo paths, and broad `g_MapEditorMode` side-effect inventory.
**Confidence:** High for branch order, fields, callback dispatch, and active YR path; Medium for exact semantic labels of `BulletClass+0xB0/+0xAC/+0x130/+0x154` because this slice proves pointer-expiry behavior, not all consumers.
**Active in YR:** Yes. Standard object death calls `ObjectClass::UnInit`, which calls `Detach_From_All_Lists` at `0x005F6616` before conceal and alive clear; the object-expiry roster dispatches listener vtable `+0x28`; `BulletClass` vtable base `0x007E46E4` has slot `+0x28` at `0x007E470C -> 0x004684E0` per current sibling reports and this slot's Ghidra assembly context.

## 1. Working Notes Gate

Target question: What exactly does the `BulletClass` vtable `+0x28` pointer-expired callback do when reached from pre-conceal `Detach_From_All_Lists`, and what must Rust preserve for projectile target cleanup?

Non-goals: Do not re-investigate the whole cleanup roster, full homing math, full projectile spawn, every listener body, every map-editor side effect, or non-destruction limbo transitions.

Evidence needed to mark COMPLETE: decompile plus assembly context for `Detach_From_All_Lists` and `ObjectClass::UnInit` ordering; vtable evidence for `BulletClass+0x28`; assembly context for `0x004684E0..0x004685C6`; decompile/assembly for the mirrored `BulletClass::UpdateTarget` and `MapClass::Get_CellClass`; focused Rust scan of projectile/target/death surfaces.

Stop conditions: stop after the callback's compared fields, write behavior, target-cell fallback conditions, path liveness, and Rust handoff are verified; record broader listener roster/runtime mutation behavior as out of scope.

## 2. Overview

The Bullet callback runs while the expiring target object still exists, before `ObjectClass::Conceal` and before `Object+0x90` is cleared. Active in YR: Yes. Evidence: `ObjectClass::UnInit` calls `Detach_From_All_Lists` at `0x005F6616`, virtual conceal at `0x005F661F`, and writes `Object+0x90 = 0` at `0x005F6625`.

For bullets, the callback first runs inherited pointer-expired cleanup, then checks Bullet-specific pointer fields against the expired object. If the expired object is the bullet target at `BulletClass+0x10C`, the callback either replaces that target pointer with a `CellClass*` for the target's last cell or clears it to null. Active in YR: Yes. Evidence: assembly context `0x004684E0..0x0046859C`; mirrored decompile of `BulletClass::UpdateTarget @ 0x00468430`.

## 3. Class Layout / Key Offsets

| Field / slot | Verified behavior in this slice | Evidence | Active in YR |
|---|---|---|---|
| Bullet vtable `+0x28` | pointer-expired callback body starts at `0x004684E0` | sibling vtable read `0x007E470C -> 0x004684E0`; `Detach_From_All_Lists` calls `[listener_vtbl+0x28]` | Yes |
| `BulletClass+0xB0` | cleared to null when it equals expired pointer | `0x004684F7..0x00468503` | Yes |
| `BulletClass+0x10C` | target pointer; object pointer becomes `CellClass*` or null when expired | `0x00468509..0x0046859C` | Yes |
| `BulletClass+0xAC` | cleared to null when it equals expired pointer | `0x004685A2..0x004685AA` | Conditional; type-pointer expiry is not normal skirmish target death |
| `BulletClass+0x130` | cleared to null when it equals expired pointer | `0x004685B0..0x004685B8` | Conditional |
| `BulletClass+0x154` | cleared to null when it equals expired pointer | `0x004685BE..0x004685C6` | Conditional |
| target vtable `+0x48` | coordinate reader used before target fallback decision | `0x00468517..0x0046851E`; mirrored `0x0046843A..0x00468443` | Yes |
| target vtable `+0x54` | high-flying predicate for the clear-vs-cell fallback branch | callback `0x0046855A..0x00468567`; `ObjectClass::IsHighFlying @ 0x005F6B90` | Yes |
| `g_MapEditorMode @ 0x00A8E7AC` | nonzero skips cell fallback and clears target | `0x00468551..0x00468558`; address map and architecture docs identify the global | Conditional; normal gameplay has it zero, editor/suppression contexts can set it |
| off-map sentinel `DAT_0089DDF0/2` | if target cell equals this sentinel, clear target instead of cell fallback | `0x00468569..0x00468583` | Yes |

## 4. Core Logic

### 4.1 Dispatch and pre-conceal ordering

`ObjectClass::UnInit` calls `Detach_From_All_Lists` before virtual conceal and alive clear. `Detach_From_All_Lists` obtains the expiring object's RTTI through vtable `+0x2C`, then for object-registered listeners iterates `DAT_00B0F724[0..DAT_00B0F730)` and calls each listener's vtable `+0x28(expired, removal_flag)`. Active in YR: Yes. Evidence: `0x005F6616..0x005F6625`, `0x007258D0..0x0072595F`.

For the ordinary object branch, `Detach_From_All_Lists` passes the expiring object as the first pushed callback argument and the caller's removal flag as the second pushed argument. Active in YR: Yes. Evidence: callback call context `0x00725947..0x00725954` pushes `EDI` then `ESI` before calling `[EDX+0x28]`; `0x004684E0` reads `[ESP+0x8]` as the removal flag and `[ESP+0x20]` as the expired pointer after its prologue.

### 4.2 Bullet callback branch order

Verified branch order for `0x004684E0..0x004685C6`:

1. Read the removal flag, preserve registers, load expired pointer into `EDI`, and call inherited pointer-expired cleanup `FUN_005F5230(expired, removal_flag)`.
2. If `Bullet+0xB0 == expired`, write `Bullet+0xB0 = 0`.
3. If `Bullet+0x10C == expired`, run target fallback/clear logic below.
4. If `Bullet+0xAC == expired`, write `Bullet+0xAC = 0`.
5. If `Bullet+0x130 == expired`, write `Bullet+0x130 = 0`.
6. If `Bullet+0x154 == expired`, write `Bullet+0x154 = 0`.

Active in YR: Yes for the callback and the `+0x10C` target branch on ordinary target destruction; Conditional for `+0xAC/+0x130/+0x154` because those are pointer roles that only matter if that referenced object/type/anim expires. Evidence: assembly contexts `0x004684E0..0x004685C6`.

### 4.3 Target fallback and clear decision

If `Bullet+0x10C` equals the expired pointer:

1. Call expired target vtable `+0x48` to read coordinates.
2. Convert returned lepton `x` and `y` to cell coordinates using signed divide-by-256 truncation shape: `CDQ; AND EDX,0xFF; ADD EAX,EDX; SAR EAX,8`.
3. Store those two shorts through `MapCoord_Set @ 0x0042D470`.
4. If `g_MapEditorMode != 0`, write `Bullet+0x10C = 0`.
5. Else call target vtable `+0x54`.
6. If `+0x54` returns nonzero, write `Bullet+0x10C = 0`.
7. Else compare the computed cell against `DAT_0089DDF0/2`.
8. If the cell equals the sentinel, write `Bullet+0x10C = 0`.
9. Otherwise call `MapClass::Get_CellClass @ 0x005657A0` and write the returned pointer to `Bullet+0x10C`.

Active in YR: Yes for normal in-game target destruction with `g_MapEditorMode == 0`; Conditional for map-editor/suppression contexts where the same branch clears the target. Evidence: callback assembly `0x00468509..0x0046859C`; mirrored `BulletClass::UpdateTarget` decompile `0x00468430`; `MapCoord_Set @ 0x0042D470`.

The `+0x54` predicate must not be named "is on map" from this slice. For object targets it resolves to `ObjectClass::IsHighFlying @ 0x005F6B90`, which returns true only when byte `Object+0x74` is nonzero and virtual `+0x1C8` height is at least `2 * DAT_00AC13C8`. Active in YR: Yes. Evidence: decompile/assembly `0x005F6B90..0x005F6BB1`.

### 4.4 Cell fallback safety

`MapClass::Get_CellClass` computes `index = y * 0x200 + x` from signed 16-bit cell coordinates. If `index < 0`, `index >= 0x40000`, or the cell pointer is null, it stores the requested cell coordinate in `DAT_00ABDC74` and returns the dummy cell at `DAT_00ABDC50`; otherwise it returns the real cell pointer. Active in YR: Yes. Evidence: decompile/assembly `0x005657A0..0x005657D5`.

This means the Bullet callback never writes a dangling expired object pointer back to `+0x10C`. It writes either null or a `CellClass`-compatible pointer returned by the map. Active in YR: Yes. Evidence: write sites `0x00468594` and `0x0046859C`.

### 4.5 Relation to `BulletClass::UpdateTarget`

`BulletClass::UpdateTarget @ 0x00468430` mirrors the `+0x10C` target branch: read `+0x10C` coordinates through vtable `+0x48`, check `g_MapEditorMode`, check vtable `+0x54`, compare against `DAT_0089DDF0/2`, and either write `MapClass::Get_CellClass` or null to `+0x10C`. Active in YR: Conditional. Evidence: decompile `0x00468430`; xref evidence shows one caller, `TeleportLocomotionClass::StateMachineTick @ 0x007193EE`.

It is not the normal target-death callback. Active in YR: Yes for the distinction. Evidence: `get_function_xrefs 0x00468430 -> 0x007193EE`; normal death path uses `ObjectClass::UnInit -> Detach_From_All_Lists -> vtable +0x28`.

## 5. INI Keys

No INI key directly gates the pointer-expired callback. Stock activation of the projectile path comes from existing weapon/projectile data: `[GGI] Secondary=MissileLauncher`, `[MissileLauncher] Projectile=AAHeatSeeker2`, `[AAHeatSeeker2] ROT=60`, `AG=yes`, `AA=yes`, `Arm=2`, `Image=DRAGON`. Active in YR: Yes for stock deployed Guardian GI missiles; the callback itself is engine lifecycle behavior, not an INI parser feature.

`g_MapEditorMode` is a runtime global (`0x00A8E7AC`) with many engine uses and is cleared for normal game initialization per architecture docs. Active in YR: Conditional; ordinary skirmish/gameplay target expiry uses the zero branch, while editor/silent-spawn/suppression contexts can set it.

## 6. Integration Points

| Integration | Verified role | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::UnInit` | enters pre-conceal cleanup | `0x005F6616` calls `0x007258D0` before `0x005F661F` and `0x005F6625` | Yes |
| `Detach_From_All_Lists` | dispatches listener vtable `+0x28` | decompile `0x007258D0`; assembly `0x00725947..0x00725954` | Yes |
| `BulletClass` vtable | binds `+0x28` to callback body | sibling vtable read `0x007E470C -> 0x004684E0` | Yes |
| inherited pointer-expired cleanup | callback first clears base Object pointer roles | `0x004684EE..0x004684F2`; `FUN_005F5230` decompile | Yes |
| target cell fallback | target object pointer becomes map cell pointer | `0x0046858F..0x00468594`; `MapClass::Get_CellClass @ 0x005657A0` | Yes/Conditional on gates |
| target clear | target pointer becomes null | `0x0046859C`; gates at `0x00468551..0x00468583` | Yes/Conditional on gates |

## 7. Current Rust Implementation Status

Focused scan only:

| Rust surface | Current behavior | Delta for this report |
|---|---|---|
| `src/sim/game_entity.rs` | `GameEntity` has `homing_state: Option<HomingState>` and ordinary entity target fields such as `attack_target` and `movement_target`. | No native pointer-expired listener stage or object-vs-cell target abstraction matching `Bullet+0x10C` object/CellClass/null. |
| `src/sim/movement/homing_movement.rs` | `HomingState` stores `target_id: Option<u64>` and `last_known_rx/ry`; each tick, missing target lookup sets `target_id = None` and continues to last-known cell. | Similar high-level fallback but wrong mechanism/order: fallback occurs by failed ID lookup during projectile tick, not pre-conceal callback while the target still exists, and it does not preserve a typed CellClass target. |
| `src/sim/combat/mod.rs` | `handle_entity_deaths` calls `clear_targets_on_dead_entity`, which only clears `attack_target` references to a dead entity; immediate non-SHP deaths remove occupancy/entity. | No per-listener callback; projectile homing target cleanup is not handled at the native pre-conceal point. |
| `src/sim/world/mod.rs` | `despawn_entity` removes occupancy, clears radio contacts, removes from `EntityStore`, unregisters live object. Homing movement runs later in `advance_tick`; detonation list is currently unused. | Removal is ID-store based and can make projectiles observe missing targets after removal, not the gamemd ordered callback and cell/null write. |
| `src/app_sim_tick.rs` | death-animation cleanup despawns finished animated entities later. | Animation linger/despawn timing is not the same as `UnInit` pre-conceal pointer-expiry plus pending delete. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::UnInit` pre-conceal dispatch order | verified | `0x005F6616..0x005F6625` | none |
| `Detach_From_All_Lists` `+0x28` dispatch shape | verified | `0x007258D0`, `0x00725947..0x00725954` | vector mutation behavior during callbacks is out of scope |
| Bullet vtable `+0x28` binding | verified | sibling raw vtable read `0x007E470C -> 0x004684E0`; callback assembly context | none for this slice |
| callback inherited cleanup call | verified | `0x004684EE..0x004684F2`; `FUN_005F5230` decompile | exact semantic labels of base fields outside scope |
| callback `+0xB0` clear | verified | `0x004684F7..0x00468503` | field consumer inventory outside scope |
| callback `+0x10C` cell/null logic | verified | `0x00468509..0x0046859C`; `0x00468430` mirror | none |
| callback `+0xAC/+0x130/+0x154` clears | verified | `0x004685A2..0x004685C6` | field consumer inventory outside scope |
| `MapClass::Get_CellClass` fallback | verified | `0x005657A0..0x005657D5` | none |
| `+0x54` predicate identity | verified | `ObjectClass::IsHighFlying @ 0x005F6B90` | non-Object target slot variants out of scope |
| `BulletClass::UpdateTarget` caller distinction | verified | decompile `0x00468430`; xref `0x007193EE` | chrono-specific behavior outside scope |
| Rust projectile target cleanup | touched-not-exhausted | focused `rg` scan and file reads listed in Sources | exact future patch design belongs to implementation work |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-001 - Is Bullet pointer-expired reached before target conceal/alive clear? -> Yes; `ObjectClass::UnInit` calls `Detach_From_All_Lists` before virtual `+0xD4` and `[+0x90]=0`.` (evidence: `0x005F6616..0x005F6625`; Active in YR: Yes)
- `[RESOLVED] OQ-002 - Which vtable slot is used for the callback? -> Listener vtable `+0x28`; Bullet slot binds to `0x004684E0`.` (evidence: `0x007258D0`; `0x007E470C -> 0x004684E0`; Active in YR: Yes)
- `[RESOLVED] OQ-003 - Which Bullet fields are compared/written? -> `+0xB0`, `+0x10C`, `+0xAC`, `+0x130`, `+0x154` are compared against expired pointer and selectively written; target has special cell/null logic.` (evidence: `0x004684F7..0x004685C6`; Active in YR: Yes/Conditional per field)
- `[RESOLVED] OQ-004 - Does normal target expiry always null `+0x10C`? -> No; ordinary non-editor, non-high-flying, non-sentinel cell writes `MapClass::Get_CellClass` to `+0x10C`.` (evidence: `0x00468551..0x00468594`; Active in YR: Yes)
- `[RESOLVED] OQ-005 - What clears target instead? -> map-editor mode nonzero, target `+0x54` true/high-flying, or computed cell equals `DAT_0089DDF0/2`.` (evidence: `0x00468551..0x0046859C`; Active in YR: Conditional)
- `[RESOLVED] OQ-006 - Is `+0x54` an on-map check? -> Not for Object targets; it is `ObjectClass::IsHighFlying`, checking `Object+0x74` and height >= `2 * DAT_00AC13C8`.` (evidence: `0x005F6B90`; Active in YR: Yes)
- `[RESOLVED] OQ-007 - Is `MapClass::Get_CellClass` safe for out-of-bounds cells? -> Yes; it returns dummy cell `DAT_00ABDC50` and records the requested cell at `DAT_00ABDC74`.` (evidence: `0x005657A0..0x005657D5`; Active in YR: Yes)
- `[RESOLVED] OQ-008 - Is `BulletClass::UpdateTarget` the normal death path? -> No; it mirrors the branch but has sole xref from `TeleportLocomotionClass::StateMachineTick`.` (evidence: `get_function_xrefs 0x00468430`; Active in YR: Conditional)
- `[RESOLVED] OQ-009 - Does current Rust model this callback stage? -> No central equivalent found; homing uses target-id lookup and last-known cell after missing target, while deaths clear attack targets or remove entities.` (evidence: `src/sim/movement/homing_movement.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`; Active in YR comparison: Yes)
- `[DEFERRED] OQ-010 - What exact runtime contents and mutation behavior does `DAT_00B0F724` have during callback iteration?` (category: out-of-scope; reason: parent/other slot owns roster census; next-step-if-pursued: runtime watchpoint or dedicated listener-mutation report)
- `[DEFERRED] OQ-011 - What are every consumer and canonical name for `Bullet+0xB0/+0xAC/+0x130/+0x154`?` (category: out-of-scope; reason: this slice only needs pointer-expired compare/write behavior; next-step-if-pursued: BulletClass field inventory)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bullet target invalidation runs as a pre-conceal listener callback, before target alive false/removal. Active in YR: Yes. | `0x005F6616..0x005F6625`; `0x00725947..0x00725954`; `0x007E470C -> 0x004684E0` | missing central pre-conceal projectile/listener stage | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/sim/movement/homing_movement.rs` | Add an ordered expiry notification phase that lets projectiles update targets before entity conceal/removal and before alive/dead state becomes externally final. | A homing projectile targeting a ground unit that dies in the same tick updates its target during death cleanup while the target is still present, then later projectile AI sees the updated target state. | `bullet_pointer_expired_runs_before_target_conceal_and_alive_clear` | Do not use `entities.get(target_id).is_none()` as the primary invalidation mechanism. |
| For `Bullet+0x10C`, expired non-high-flying, non-sentinel ground targets become a `CellClass*` fallback, not null. Active in YR: Yes. | callback assembly `0x00468509..0x00468594`; `MapClass::Get_CellClass @ 0x005657A0` | Rust `HomingState` only has `target_id: Option<u64>` plus `last_known_rx/ry`; no typed object/cell/null target state | `src/sim/game_entity.rs`, `src/sim/movement/homing_movement.rs` | Represent projectile target cleanup distinctly enough to preserve object target vs cell fallback vs null. A cell fallback should remain an explicit target source for homing/detonation, not just stale coordinates from a failed lookup. | Destroy a ground target at cell `(x,y)` while an AAHeatSeeker2-style projectile is in flight; projectile target becomes cell `(x,y)` in cleanup and later homing reads that cell target. | `bullet_target_expiry_retargets_ground_object_to_cell_fallback` | Do not null all projectile targets on death, and do not conflate cell fallback with a missing object lookup. |
| Map-editor/suppression mode, high-flying targets, and sentinel cells clear `Bullet+0x10C` instead of cell fallback. Active in YR: Conditional. | `g_MapEditorMode` read `0x00468551..0x00468558`; high-flying call `0x0046855A..0x00468567`; sentinel compare `0x00468569..0x00468583`; null write `0x0046859C` | Rust has no equivalent gate in homing target cleanup | future projectile target cleanup helper; possibly world lifecycle context for editor/silent-spawn mode | Implement branch-specific cleanup predicates: normal ground death uses cell fallback; high-flying/sentinel/editor cleanup clears to null. | Projectile targeting a high-flying aircraft that expires clears target rather than retargeting to its ground cell. | `bullet_target_expiry_high_flying_target_clears_instead_of_cell_fallback` | Do not use target category alone; native gate is the target's `+0x54` high-flying predicate and current coordinate sentinel result. |
| The callback also clears Bullet pointer fields `+0xB0`, `+0xAC`, `+0x130`, and `+0x154` when equal to the expired pointer. Active in YR: Yes/Conditional. | `0x004684F7..0x00468503`, `0x004685A2..0x004685C6` | unchecked; no Rust BulletClass-equivalent field inventory because projectile implementation is partial | future projectile entity/model surfaces | When these Bullet pointer roles are represented, wire them into the same pointer-expired stage with exact compare-then-null behavior. | Expiring a referenced owner/type/weapon/anim role clears only matching projectile pointer fields and leaves unrelated target state unchanged. | `bullet_pointer_expired_clears_only_matching_auxiliary_fields` | Do not clear unrelated projectile fields just because any referenced object expired. |

### Negative Facts / Do Not Do

- Do not treat `ObjectClass::Conceal` as the bullet invalidation trigger. Active in YR: Yes. Evidence: invalidation dispatch is at `0x005F6616`, before virtual conceal at `0x005F661F`.
- Do not clear target alive/entity storage before running projectile target cleanup. Active in YR: Yes. Evidence: alive clear is `0x005F6625`, after `Detach_From_All_Lists`.
- Do not always null in-flight projectile targets on normal ground target death. Active in YR: Yes. Evidence: `0x0046858F..0x00468594` writes `MapClass::Get_CellClass` to `Bullet+0x10C`.
- Do not call the `+0x54` gate "on-map" for this branch. Active in YR: Yes. Evidence: Object target slot resolves to `ObjectClass::IsHighFlying @ 0x005F6B90`.
- Do not implement `BulletClass::UpdateTarget` as the normal target-death path. Active in YR: Conditional. Evidence: xref only from `TeleportLocomotionClass::StateMachineTick @ 0x007193EE`; normal death uses vtable `+0x28`.

### Remaining Uncertainty

- Exact canonical names and consumers for auxiliary Bullet fields `+0xB0/+0xAC/+0x130/+0x154` remain outside this slice; only compare/write behavior is verified here.
- Runtime mutation behavior of `DAT_00B0F724` during callbacks is a roster/listener-system question, not part of this Bullet callback slice.
- Full Rust patch shape is intentionally not designed here because projectile spawn/detonation is still partial; the handoff identifies the required behavioral surface and tests.

### Stale Docs / Follow-up Docs

- `docs/research/BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`: replacement for references that cite `ObjectClass::UnInit @ 0x005F6620` as the detach call site: "`ObjectClass::UnInit` calls `Detach_From_All_Lists` at `0x005F6616`; virtual conceal follows at `0x005F661F`, and `Object+0x90` alive clear follows at `0x005F6625`."
- No new contradiction was found against the prior correction that the callback's `+0x54` predicate is high-flying, not an on-map predicate.

## Sources

- Ghidra read-only decompile/assembly context:
  - `ObjectClass::UnInit @ 0x005F65F0`, especially `0x005F6616..0x005F6625`
  - `Detach_From_All_Lists @ 0x007258D0`, especially `0x00725947..0x00725954`
  - `BulletClass` pointer-expired callback body `0x004684E0..0x004685C6`
  - `BulletClass::UpdateTarget @ 0x00468430`
  - `MapClass::Get_CellClass @ 0x005657A0`
  - `MapCoord_Set @ 0x0042D470`
  - inherited pointer-expired cleanup `FUN_005F5230`
  - `ObjectClass::IsHighFlying @ 0x005F6B90`
  - `get_function_xrefs 0x00468430 -> 0x007193EE`
- Prior/sibling docs cross-checked:
  - `docs/research/BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`
  - `docs/research/AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`
  - `docs/research/OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`
  - `docs/research/DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`
  - `docs/research/ADDRESS_MAP.md`
  - `docs/research/GAMEMD_ARCHITECTURE.md`
- Rust focused scan:
  - `src/sim/game_entity.rs`
  - `src/sim/movement/homing_movement.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/world/mod.rs`
  - `src/app_sim_tick.rs`
