# Detach Post-Listener Cleanup Helpers - Re-swarm 2026-05-28

**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `PlacementImageVector::RemoveAndClearOwners? @ 0x00439150`, `SpawnRetreat__Remove @ 0x0054E590`, `DiskLaserClass::DetachFromObject @ 0x004A7900`, no-op hook `0x00413490`, `FUN_00733160 @ 0x00733160`, Tactical pointer-expired body `0x006DA560`, tag/scenario helper `FUN_0055B880 @ 0x0055B880`, `ObjectClass::Conceal @ 0x005F4D30`, `FUN_0055BAE0 @ 0x0055BAE0`
**Investigation Mode:** focused swarm subagent
**Claimed Scope:** Helper sequence after the main `DAT_00B0F724` listener roster dispatch inside `Detach_From_All_Lists`: current/UI pointer cleanup, spawn retreat, disk-laser cleanup, object-to-cell fallback, special vectors, and ordering relative to broad roster dispatch and later LogicClass unregister.
**Non-Scope:** Re-decoding settled `ObjectClass::UnInit` pre-Conceal order; full listener roster census; all `+0x28` listener bodies; Rust implementation; Ghidra label edits.
**Confidence:** High for helper order and bodies that have function boundaries; High for Tactical `0x006DA560` first-body semantics from raw disassembly but Medium for canonical name because Ghidra has no function boundary there; Medium for semantic names of `0x00439150` and `0x00733160`.
**Active in YR:** Yes for standard object removal reaching the object-registered branch; Conditional for helpers whose global/vector/list is empty or subsystem-specific; No for the `0x00413490` hook body because it returns immediately.

## 0. Working Notes

**Target question:** After the main object listener roster dispatch in `Detach_From_All_Lists`, what exact helper sequence runs, what does each helper mutate, which paths are active in standard YR, and how should Rust phase post-listener cleanup relative to `LogicClass` unregister?

**Non-goals:** Do not re-prove `ObjectClass::UnInit` ordering except as caller context; do not re-census `DAT_00B0F724`; do not investigate every RTTI-specific listener body; do not edit Rust, INI, sibling docs, or `.swarm-claims.md`; do not use mutating Ghidra operations.

**Evidence needed to mark COMPLETE:** Decompile and assembly/disassembly range for `0x007258D0`; decompile plus assembly/disassembly ranges for post-listener helpers where boundaries exist; raw disassembly for Tactical `0x006DA560`; evidence that `FUN_0055BAE0` runs later through Conceal rather than in the post-listener helper sequence; read-only Rust scan for likely affected surfaces; active-YR label for every material finding.

**Stop conditions:** Stop after the fixed post-listener sequence, object-to-cell fallback, Tactical/current UI cleanup, special-vector placement, LogicClass unregister relation, and Rust handoff are covered. Leave exact runtime vector contents and Tactical full class naming as uncertainty if read-only Ghidra cannot provide a better boundary.

## 1. Executive Summary

For object-registered removals, `Detach_From_All_Lists` clears two current/UI globals before the broad listener loop, then forward-iterates `DAT_00B0F724[0..DAT_00B0F730)` and calls each listener's primary vtable `+0x28(target, removal_flag)`. Only after that loop does it run the fixed helper tail:

1. `0x00439150` on global `0x87F5D8`, removing the target from a DynamicVector-like list and clearing `entry+0x24` backrefs in remaining entries.
2. `SpawnRetreat__Remove @ 0x0054E590` on global `0xABC5F8`, which either removes rows whose first pointer is the target or converts rows whose second pointer is the target into a CellClass fallback for the first object.
3. Reverse disk-laser array loop over `g_DiskLaserClass_Array_Count`, calling `DiskLaserClass::DetachFromObject @ 0x004A7900` on each laser.
4. RTTI `0x0F`, `0x01`, or `0x02` calls `0x00413490`, but that function is an immediate return in this binary.
5. `FUN_00733160 @ 0x00733160`, which reverse-finds and stable-removes the target from a global vector at `DAT_00B0FE6C` if present.
6. `g_Tactical->vtable+0x28(target, 1)` if `g_Tactical` exists; raw body `0x006DA560` clears matching entries in a 12-byte-step Tactical object-slot table and clears current focus globals `0x00880978/7C/80` if they refer to the target.
7. `FUN_0055B880(target, removal_flag)`, which only does work for RTTI `0x2C`: it removes matching entries from the vector at `DAT_008B40CC/DAT_008B40D8`.

`LogicClass` active-vector unregister is not one of these post-listener helpers. It occurs later when `ObjectClass::UnInit` calls virtual `+0xD4`, which reaches `ObjectClass::Conceal`; Conceal conditionally calls `FUN_0055BAE0` under type/mode/UniqueID gates before setting `InLimbo`.

## 2. Verified Fixed Order

| Order | Binary evidence | Operation | Active in YR |
|---:|---|---|---|
| 0 | `0x007258D9..0x00725909` decompile/assembly | Read RTTI through vtable `+0x2C`; clear `DAT_0088098C` if equal to target; clear `g_UIModeLock @ 0x00880990` and call `0x004A8BF0(0)` if equal to target. | Yes; object removal always executes these pre-loop checks, writes only when globals match. |
| 1 | `0x0072593E..0x0072595F` decompile/assembly | For targets with abstract flag bit `+0x14 bit 1`, forward-loop `DAT_00B0F724` using current `DAT_00B0F730`, calling listener `+0x28(target, removal_flag)`. | Yes for object-registered standard YR objects. |
| 2 | `0x00725961..0x00725967`; helper `0x00439150..0x004391B5` | Call `0x00439150` with global `ECX=0x87F5D8` and target. | Conditional on entries in that global vector; call itself is active. |
| 3 | `0x0072596C..0x00725972`; helper `0x0054E590..0x0054E684` | Call `SpawnRetreat__Remove` with global `ECX=0xABC5F8` and target. | Conditional on spawn-retreat rows; call itself is active. |
| 4 | `0x00725977..0x00725993`; helper `0x004A7900..0x004A79C9` | Reverse-loop disk lasers and detach matching source/target object. | Conditional: active for stock Floating Disc disk-laser effects when present; no-op if array empty. |
| 5 | `0x00725995..0x007259AA`; body `0x00413490` | For RTTI `0x0F`, `0x01`, or `0x02`, call no-op hook. | No material effect in standard YR; body immediately returns. |
| 6 | `0x007259AF..0x007259B1`; helper `0x00733160..0x007331A9` | Remove target from global vector at `DAT_00B0FE6C` by reverse search and stable compaction. | Conditional on global vector existence and membership; call itself is active. |
| 7 | `0x007259B6..0x007259C5`; raw body `0x006DA560..0x006DA5AF` | If `g_Tactical @ 0x00887324` is non-null, call Tactical `+0x28(target, 1)`. | Conditional on Tactical singleton; active in normal gameplay UI. |
| 8 | `0x007259C8..0x007259CF`; helper `0x0055B880..0x0055B8DE` | Call `FUN_0055B880(target, removal_flag)`, which removes from `DAT_008B40CC` only for RTTI `0x2C`. | Conditional; active only for RTTI `0x2C` targets. |

## 3. Current/UI/Tactical Pointer Cleanup

Active in YR: Yes for branch execution; writes are conditional on the target being the current pointer.

Before the broad roster loop, `Detach_From_All_Lists` clears `DAT_0088098C` if it equals the target and clears `g_UIModeLock @ 0x00880990` if it equals the target. The `g_UIModeLock` clear also calls `0x004A8BF0` on singleton `0x87F7E8` with argument `0`, and `0x004A8BF0` clears placement image state at `this+0x117C` after dirtying the previous placement footprint if one exists (`0x004A8BF0..0x004A8D1F`; clear path at `0x004A8C47..0x004A8D1F`).

After disk-laser and `FUN_00733160`, Tactical `+0x28` runs if `g_Tactical` is non-null. Ghidra has no function boundary at `0x006DA560`, but raw disassembly verifies the relevant body:

- `0x006DA560..0x006DA57B`: iterate addresses `0x00B0CEC8` through `< 0x00B0E638` in 12-byte strides; if slot first dword equals target, write zero.
- `0x006DA57D..0x006DA5AF`: if target equals `DAT_00880978`, call `0x004A8D50(0)` on singleton `0x87F7E8`, then clear `DAT_00880978 = 0`, `DAT_0088097C = 0`, and `DAT_00880980 = -1`.

Rust implication: selected/current/tactical focus cleanup is not only `GameEntity.selected = false`. Rust currently has app-layer selection and placement preview state (`src/sim/game_entity.rs:161`, `src/app_sim_tick.rs:908`, `src/render/selection_overlay.rs`) but no native-order Tactical pointer-expiry phase tied to object removal.

## 4. Helper Bodies

### 4.1 `0x00439150`: remove from vector and clear remaining row backrefs

Active in YR: Conditional. The call is active for every object-registered detach; mutation occurs only if the target exists in global vector `0x87F5D8` or if remaining rows have `+0x24 == target`.

Evidence: `Detach_From_All_Lists` call site `0x00725961..0x00725967`; helper assembly/decompile `0x00439150..0x004391B5`.

The helper uses `ECX` as a vector-like object and the stack argument as the target. It calls vector vtable `+0x10` to find the target pointer, removes one found entry with count decrement and left compaction, then iterates remaining entries and clears `entry+0x24` if it equals the target. This is a post-roster cleanup, not part of the main listener dispatch.

### 4.2 `SpawnRetreat__Remove @ 0x0054E590`: row removal and object-to-cell fallback

Active in YR: Conditional. The call is active for every object-registered detach; effects require rows in the spawn-retreat global at `0xABC5F8`.

Evidence: call site `0x0072596C..0x00725972`; helper assembly/decompile `0x0054E590..0x0054E684`.

The helper scans rows from the end. If the target equals row first pointer `[row+0]`, it frees the row object, decrements the count, and left-compacts the vector. If the target equals row second pointer `[row+4]`, it does not delete the row. Instead, it calls the target's vtable `+0x48` to get coordinates, calls `MapClass::Get_CellClass @ 0x005657A0`, calls the first object's vtable `+0x3C8(cell)`, then calls first object's vtable `+0x1E8(1, 0)`, and stores the returned CellClass pointer into `[row+4]`.

This is the requested object-to-cell fallback: a retreat row that referenced an object as its second endpoint is downgraded to a cell target at the expired object's current location.

### 4.3 `DiskLaserClass::DetachFromObject @ 0x004A7900`

Active in YR: Conditional. Active for standard Floating Disc disk-laser effect instances when source or target matches; no-op when the disk-laser global array is empty or the object is unrelated.

Evidence: reverse-loop call site `0x00725977..0x00725993`; helper decompile and assembly `0x004A7900..0x004A79C9`; existing disk-laser reports confirm stock Floating Disc `DiskLaser=yes`.

The object branch snapshots `g_DiskLaserClass_Array_Count`, starts at `count - 1`, and decrements to zero. For each DiskLaser object, if target equals `DiskLaser+0x24` or `DiskLaser+0x28`, the helper writes `DiskLaser+0x30 = -1` and appends the DiskLaser object to the pending-delete vector at `DAT_00B0F69C/DAT_00B0F6A8`, growing if allowed. If both source and target match the same expired object, the body can execute both append sites; no deduplication was observed in this helper.

### 4.4 `0x00413490`: no-op RTTI hook

Active in YR: No material effect. The branch is reached for RTTI `0x0F`, `0x01`, or `0x02`, but `0x00413490` is an immediate return in this binary.

Evidence: branch/call `0x00725995..0x007259AA`; decompile of `0x00413490` returns immediately.

### 4.5 `FUN_00733160 @ 0x00733160`

Active in YR: Conditional. The call is active in the object branch; mutation occurs only when `DAT_00B0FE6C` exists and contains the target.

Evidence: call site `0x007259AF..0x007259B1`; helper decompile and assembly `0x00733160..0x007331A9`.

The helper reads global vector pointer `DAT_00B0FE6C`, returns if null or count is zero, scans backward for the target, decrements count on match, and shifts later entries left. It does not call listener vtables and is not the `LogicClass` active-vector remover.

### 4.6 `FUN_0055B880 @ 0x0055B880`

Active in YR: Conditional. The helper is always called at the end of the object branch and some special-vector branches, but it only mutates when the target RTTI is `0x2C`.

Evidence: object-branch call site `0x007259C8..0x007259CF`; helper decompile/assembly `0x0055B880..0x0055B8DE`.

For RTTI `0x2C`, it loops over `DAT_008B40CC[0..DAT_008B40D8)`, removes any matching target with stable left compaction, decrements the loop index after removal, and continues scanning. For all other RTTI values, it returns without mutation.

## 5. Special Vectors Outside The Object Branch

Active in YR: Conditional by removed object's RTTI/class. These are in `Detach_From_All_Lists`, but they are not part of the standard object-registered post-loop helper tail unless control reaches their branch.

| RTTI branch | Binary evidence | Action | Active in YR |
|---|---|---|---|
| `0x18` | `0x0072590E..0x00725925` | Clear singleton `DAT_00A8ED78` if it equals target, then continue to object branch test. | Conditional. |
| `0x0D` | `0x007259DA..0x00725A15` | Iterate `g_HouseClass_RemoveListeners`, call listener `+0x28`, then call `FUN_0055B880` and return. | Conditional for House-like removals; not object-tail order. |
| `0x04` | `0x00725A16..0x00725A47` | Iterate `g_AnimClass_RemoveListeners`, call listener `+0x28`, and return. | Conditional; active for Anim expiry, including SQDG/Temporal listener paths in prior reports. |
| abstract type cast | `0x00725A4D..` decompile | If dynamic cast to AbstractType succeeds, iterate `DAT_00B0F674` and call `FUN_00678850`, then return. | Conditional for type-class removals. |
| `0x0C` | switch decompile | Iterate `g_FactoryClass_RemoveListeners`, then `MapClass__UnregisterBridgeRepairHut(target, 1)`. | Conditional. |
| `0x22` | switch decompile | Iterate `g_TeamClass_RemoveListeners`. | Conditional. |
| `0x26` | switch decompile | Iterate `g_TriggerClass_RemoveListeners`. | Conditional. |
| `0x2C` | switch decompile | Iterate `g_TagClass_RemoveListeners`, unregister bridge repair hut, then `FUN_0055B880`. | Conditional. |
| `0x2F/0x30` | switch decompile | Iterate `g_TriggerTypeClass_Array`. | Conditional. |
| `0x33` | switch decompile | Iterate `DAT_00B0F5F4`. | Conditional. |
| `0x3C` | switch decompile | Iterate `g_NeuronClass_RemoveListeners`. | Conditional. |

Do not merge these special RTTI vectors into the object-registered `DAT_00B0F724` post-tail. Some branches return before the object branch and therefore skip spawn-retreat, disk-laser, Tactical, and other object-tail helpers.

## 6. Ordering Relative To LogicClass Unregister

Active in YR: Yes, conditional on normal object conceal gates.

`Detach_From_All_Lists` does not call `FUN_0055BAE0`. In the settled object-death path, `ObjectClass::UnInit @ 0x005F65F0` calls `Detach_From_All_Lists` first, then calls virtual `+0xD4`. `ObjectClass::Conceal @ 0x005F4D30` is reached through that virtual path and conditionally calls `FUN_0055BAE0` only when type `+0x234` is set and the game-mode/UniqueID gates permit it. `Conceal` calls `FUN_0055BAE0` before setting `InLimbo +0x81`.

Evidence:

- `ObjectClass::UnInit` decompile `0x005F65F0`: `Detach_From_All_Lists`; virtual `+0xD4`; `Object+0x90 = 0`; pending-delete append.
- `ObjectClass::Conceal` decompile and call-site range `0x005F4DA6..0x005F4DD3`: type/mode/UniqueID gates then `FUN_0055BAE0`.
- `FUN_0055BAE0` decompile/assembly `0x0055BAE0..0x0055BB2F`: active vector membership-byte gate and stable compaction.

Rust phase implication: post-listener helpers must run before any Rust equivalent of Conceal/unregister/limbo/alive-clear. Do not place active-list unregister ahead of disk-laser/spawn/Tactical pointer-expired cleanup for the UnInit path.

## 7. Current Rust Shape

Focused read-only scan:

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs:612` | `register_live_object` pushes stable ID if not already in `live_object_order`. | Native active membership is object-byte gated and not equivalent to storage membership. |
| `src/sim/world/mod.rs:618` | `unregister_live_object` uses `retain`. | Native unregister is later Conceal work, not in `Detach_From_All_Lists`, and stable-compacts one found entry under `Object+0x98`. |
| `src/sim/world/mod.rs:675` | `despawn_entity` gathers data, clears contacts, removes occupancy/storage, then unregisters. | Native UnInit runs pointer-expiry listeners and post-listener helpers while the object is still alive/unconcealed. |
| `src/app_sim_tick.rs:298` | Death-animation completion removes occupancy and calls `despawn_entity`. | Native helper tail is not delayed until visual death completion. |
| `src/sim/combat/mod.rs:975` | Combat death clears targets on the dead entity and then marks dying/clears fields. | Missing central `Detach_From_All_Lists` phase that notifies non-entity listeners and post-tail helpers. |
| `src/sim/game_entity.rs:161` | Selection is app-layer state on entities; death clears it in combat. | Native current/Tactical pointer cleanup uses global pointers and Tactical object-slot cleanup, not just entity `selected = false`. |
| focused disk-laser/spawn scan | No direct `DiskLaserClass`/disk-laser object lifecycle or spawn-retreat structure found in `src/sim`/`src/render`. | Future disk-laser and spawn-retreat surfaces need native expiry hooks before Conceal/unregister. |

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| Object UnInit runs fixed post-listener helpers after `DAT_00B0F724` and before Conceal/LogicClass unregister. | `0x0072593E..0x007259CF`; `0x005F65F0`; `0x005F4DA6..0x005F4DD3`; Active in YR: Yes | Rust despawn/death cleanup has no native ordered pre-conceal helper phase. | future lifecycle stage around `src/sim/world/mod.rs::despawn_entity`, `src/sim/combat/mod.rs::handle_entity_deaths`, app death completion | Destroy an object referenced by a listener, spawn-retreat row, disk laser, and Tactical focus; listener observes pre-helper state, helpers then clean, Conceal/unregister happens after. | `detach_post_listener_helpers_run_before_conceal_unregister` | High: wrong phase order changes targeting, visuals, selection/focus, and active-vector iteration. |
| `SpawnRetreat__Remove` converts second-object endpoint to the expired object's current CellClass instead of deleting the row. | `0x0054E590..0x0054E684`; Active in YR: Conditional | No spawn-retreat row model found. | future spawn manager/retreat subsystem | Expire a retreat destination object referenced as row second pointer; row remains and stores the destination cell while first object receives `+0x3C8(cell)` and `+0x1E8(1,0)`. | `spawn_retreat_target_object_expiry_falls_back_to_cell` | Medium/high for aircraft/spawn return behavior once represented. |
| Tactical pointer-expiry clears matching Tactical object slots and current focus globals after disk-laser/vector cleanup. | call `0x007259B6..0x007259C5`; raw body `0x006DA560..0x006DA5AF`; Active in YR: Conditional on Tactical singleton | Rust selection/placement/current state is app-layer and not wired to native object expiry ordering. | app tactical state, selection overlay, building placement preview/current target state | Expire the object currently stored in Tactical focus/current UI state; slot refs clear only after spawn/disk-laser cleanup and before final type cleanup. | `tactical_pointer_expiry_clears_current_refs_after_post_listener_helpers` | Medium: player-visible stale selection/focus/cursor artifacts. |

## 9. Negative Facts / Do Not Do

- Do not put `FUN_0055BAE0` or Rust active-list unregister inside `Detach_From_All_Lists` post-listener helpers. Evidence: `0x007258D0` calls helper tail through `FUN_0055B880`; `FUN_0055BAE0` is reached later from `ObjectClass::Conceal @ 0x005F4D30`. Active in YR: Yes.
- Do not fold disk-laser cleanup into the `DAT_00B0F724` listener loop. Evidence: disk-laser reverse loop is after the forward listener loop at `0x00725977..0x00725993`. Active in YR: Conditional.
- Do not delete every spawn-retreat row that references the expired object. Evidence: `SpawnRetreat__Remove @ 0x0054E590` deletes when row first pointer matches but converts row second pointer to a CellClass fallback. Active in YR: Conditional.
- Do not implement Tactical expiry as only clearing `GameEntity.selected`. Evidence: Tactical `0x006DA560..0x006DA5AF` clears a 12-byte slot table and current globals `0x00880978/7C/80`; pre-loop `0x007258E0..0x00725909` clears separate current/UI globals. Active in YR: Conditional/Yes.
- Do not treat RTTI-specific special vectors as if they all fall through to the object-tail helpers. Evidence: RTTI `0x0D` and `0x04` branches return before the object branch; switch branches return after their own vector handling. Active in YR: Conditional.

## 10. Remaining Uncertainty

- Canonical class/name for global vector `0x87F5D8` used by `0x00439150` was not proven; behavior is verified, semantic name remains tentative.
- Canonical class/name for global vector `DAT_00B0FE6C` used by `0x00733160` was not proven; behavior is verified.
- Tactical `+0x28` body at `0x006DA560` lacks a Ghidra function boundary in read-only mode; raw disassembly proves the local cleanup but a future mutating-approved session could create/label the function for fuller xrefs.
- Runtime mutation safety if helpers/listeners mutate the same vectors during dispatch was not measured with a debugger.
- Stock frequency of spawn-retreat rows during standard Carrier/Hornet/ASW/Floating Disc interactions was not sampled at runtime.

## 11. Stale Docs / Suggested Wording

- `docs/research/DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`: replace "SpawnRetreat remove body/function boundary not resolved" with "SpawnRetreat__Remove @ 0x0054E590 scans rows backward; when the expired object is row first pointer it frees/removes the row, and when it is row second pointer it converts the endpoint to the expired object's current CellClass, calls the first object's `+0x3C8(cell)` and `+0x1E8(1,0)`, and stores the CellClass in the row."
- `docs/research/OBJECT_LOGIC_LIFECYCLE_ACTIVE_MEMBERSHIP_SYSTEM_MODEL_SYNTHESIS.md`: replace "Exact bodies and side effects for Tactical `+0x28`, SpawnRetreat, and any listener callback still represented only by a broad roster name" with "SpawnRetreat body and Tactical first pointer-expiry body are now partially decoded in `DETACH_POST_LISTENER_CLEANUP_HELPERS_RESWARM_20260528.md`; remaining uncertainty is canonical naming/full xrefs, not the verified object-to-cell fallback or Tactical current-ref clears."

## 12. Evidence Log

- Read-only Ghidra decompile/disassembly: `Detach_From_All_Lists @ 0x007258D0`; key assembly ranges `0x007258D9..0x00725909`, `0x0072593E..0x0072595F`, `0x00725961..0x007259CF`.
- Read-only Ghidra decompile/assembly: `0x00439150..0x004391B5`, `SpawnRetreat__Remove @ 0x0054E590..0x0054E684`, `DiskLaserClass::DetachFromObject @ 0x004A7900..0x004A79C9`, `0x00413490`, `FUN_00733160 @ 0x00733160..0x007331A9`, `FUN_0055B880 @ 0x0055B880..0x0055B8DE`.
- Read-only Ghidra raw disassembly: Tactical pointer-expired body `0x006DA560..0x006DA5AF`.
- Read-only Ghidra caller context: `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass::Conceal @ 0x005F4D30`, `FUN_0055BAE0 @ 0x0055BAE0`.
- Existing corroborating docs read/scanned: `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`, `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`, `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`, `TEMPORAL_SQDG_REMOVELISTENER_LIFECYCLE_GHIDRA_REPORT.md`, `DISK_LASER_CLASS_GHIDRA_REPORT.md`.
- Focused Rust scan: `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, `src/sim/game_entity.rs`, `src/sim/combat/*`, `src/app_sim_tick.rs`, `src/render/*`, `src/sidebar/*`, `src/ui/*`.

## 13. Status

COMPLETE for the requested helper sequence, active-YR labels, ordering relative to the broad listener loop and LogicClass unregister, Rust handoff, negative facts, and remaining uncertainty. No Rust, INI, sibling docs, claims file, or Ghidra state were modified.
