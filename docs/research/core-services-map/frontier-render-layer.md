# Core Service Profile — Z-ordered draw list (LayerClass / DisplayClass)

**Slug:** `frontier-render-layer`
**Layer:** ui-render (render-only sorted draw-list membership service; render-side, but its CHURN is driven by `sim/` object lifecycle — see Tick/render position)
**Tick/render position:** Render pass (walked by `Tactical_ObjectRenderingLoop 0x006D8DB0`); membership churn is driven from the **per-tick spine Rung T** (`ObjectClass::AI` fan-out, `0x005F3E70` base) and from object reveal/conceal lifecycle. The list itself holds no deterministic sim state.
**Primary doc:** `docs/research/LAYER_CLASS_GHIDRA_REPORT.md` (HIGH-confidence, Ghidra-verified, 24 functions decompiled). Supporting: `DISPLAYCLASS_GHIDRA_REPORT.md`, `LAYER_SYSTEM_GHIDRA_REPORT.md`, `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md`.
**Provenance:** This profile is **doc-sourced** from the four reports above plus the catalog stub (`core-services-map/_frontier.md` §A2) and the spine spec (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung T). **Ghidra was OFFLINE this session** (no UDS instance; TCP 127.0.0.1:8089 refused), so the representative addresses below could not be LIVE-re-verified this session — they are **corroborated** against the primary report (which decompiled them directly) and cross-confirmed across the three supporting reports. Flagged where a live re-verify is still owed.

---

## Purpose

The render-only **sorted draw list**: five `LayerClass` z-depth buckets that together form the object draw order the tactical pass walks each frame. Objects **submit / remove / re-submit** themselves into a per-layer bucket as their layer identity or position changes; the renderer then iterates the buckets in index order (0→4) and trusts they are already correctly ordered.

This is the **render-side twin** of the LogicClass sim vector: the same `ObjectClass*` pointers live in BOTH a LogicClass-owned active vector (drives per-tick AI, Rung T) and these DisplayClass-owned layer vectors (drive draw order). They are **distinct lists with distinct ordering contracts** — the sim vector is registration-order (FIFO via `FUN_0055BAA0`), the layer vectors are z-band + (for Ground) Y-sorted.

The contract is **observable draw ORDER**, not the internal vector mechanics: which sprite paints over which. Only **Layer 2 (Ground)** maintains sorted order (by lepton render `X + Y`, Z excluded, via incremental insertion sort kept current across ticks). Layers 0/1/3/4 (Underground / Surface / Air / Top) append unsorted and render in submission (FIFO) order.

---

## Owns (state / globals / structs)

- **`g_DisplayLayers 0x008A0360`** — the five `LayerClass<ObjectClass*>` instances, contiguous, **0x18 stride** (`5 × 0x18 = 0x78`, spanning `0x008A0360 → 0x008A03D8`). This is the draw list. Confirmed identical address in this profile's primary doc and in `drawing-helpers.md`.
  - `[0] 0x008A0360` Underground — unsorted
  - `[1] 0x008A0378` Surface — unsorted
  - `[2] 0x008A0390` **Ground — Y-sorted** (lepton X+Y)
  - `[3] 0x008A03A8` Air — unsorted (airborne aircraft / projectiles)
  - `[4] 0x008A03C0` Top — unsorted (`Layer=top` effects)
  - End sentinel `0x008A03D8` (count-based walks); `0x008A03E8` capacity-based.
- **`LayerClass` struct** (0x18 bytes, a `DynamicVectorClass<ObjectClass*>`): `+0x00 vtable` (→ `vtable__LayerClass 0x007E6060`), `+0x04 Items`, `+0x08 Capacity`, `+0x0C IsInitialized`, `+0x0D IsAllocated` (heap-ownership flag — RTTI_LABEL_DRIFT corrected 2026-05-29: 0x0D is the real free-guard, not 0x0C), `+0x10 Count`, `+0x14 CapacityIncrement = 0xA (10)`.
- **`vtable__LayerClass 0x007E6060`** — LayerClass overrides only TWO virtuals vs its `DynamicVectorClass` base: slot 0 (`~LayerClass 0x004AEB50`) and slot +0x1C (`DynamicVector__Insert` wrapper `0x005519B0`, which forks sorted-vs-unsorted). All other slots inherited.
- **Per-object layer bookkeeping** (lives ON each `ObjectClass`, not in this service): `+0x94 LayerIndex` (0..4, or `-1` = not submitted), `+0x99 wasDrawn` (cleared/set by the render loop). The `-1` sentinel is the "not in any layer" state (freshly constructed / hidden / destroyed).
- **`g_LayerNameTable 0x0081DA78`** — five string pointers (Underground/Surface/Ground/Air/Top), resolved via `Layer_To_Name 0x0048E090`.

---

## Key functions & globals (addresses)

| Symbol | Address | Role | Status |
|---|---|---|---|
| `g_DisplayLayers` | `0x008A0360` | the 5 LayerClass draw-list vectors (0x18 stride) | corroborated (primary + drawing-helpers) |
| `LayerClass__Constructor` | `0x004A862A` | one-time 5-instance init at module load; sets CapacityIncrement=10, Count=0, Items=NULL | **stub representative** — corroborated by primary doc §2 |
| `DisplayClass::Submit_Object` | `0x004A9720` | unified add/move path: if already in a layer → Remove first, then `InWhichLayer` (vtable+0x78) → insert (sorted iff layer==2) | **stub representative** — corroborated, 13 call sites enumerated |
| `DisplayClass::Remove_From_Layer` | `0x004A9770` | strip object from its cached layer (`LayerIndex`); 14 independent call sites | **stub representative** — corroborated |
| `DisplayClass::Init_Clear` | `0x004A88C0` | once per scenario boot: clears all 5 layers (vtable+0xC) + resets display fields | corroborated (primary §6) |
| `vtable__LayerClass` | `0x007E6060` | LayerClass vtable (overrides slot 0 dtor + slot 0x1C Insert) | corroborated (primary §5a, read from memory) |
| `~LayerClass` | `0x004AEB50` | destructor (vtable slot 0); frees Items if IsAllocated | corroborated |
| `DynamicVector__Insert` (LayerClass slot 0x1C) | `0x005519B0` | sorted/unsorted dispatch wrapper | corroborated |
| `DynamicVector__SortedInsert` | `0x00551A90` | the Layer-2 insertion-sort routine (NOT in vtable; called by the 0x1C wrapper when sorted) | corroborated |
| `DisplayClass::Save` / `Load` | `0x004AE720` / `0x004AE6F0` | persist all 5 layers (via `VectorClass__Save 0x00551B20` / `Load 0x00551B90`); called from `RadarClass::Save/Load` | corroborated (primary §5b) |
| `Tactical_ObjectRenderingLoop` | `0x006D8DB0` | the CONSUMER that walks the 5 layers in z-order (owned by `drawing-helpers`, not this service) | corroborated |
| `ObjectClass::InWhichLayer` | vtable+0x78 | returns which layer an object belongs to (drives Submit's bucket choice + AI re-layer trigger) | corroborated |
| `ObjectClass::GetYSort` | `0x005F6BD0` (vtable+0xB8) | sort key = render `X+Y` (Z excluded); consumed by SortedInsert for layer 2 | corroborated (cross-doc PERCLASS_VTABLE_B8 census) |
| `ObjectClass::Reveal` | `0x005F4EC0` | submits object on hidden→visible (call site `0x005F4FE2`) | corroborated (LIMBO/LINE_TRAIL/ACTIVE_OBJECT docs) |
| `ObjectClass::Conceal` | (call site `0x005F4D79`) | removes object on visible→hidden | corroborated |
| `g_LayerNameTable` | `0x0081DA78` | layer-index → name strings | corroborated |

---

## Tick / render position

- **Render pass for the WALK; sim lifecycle for the CHURN.** The list is iterated once per frame by `Tactical_ObjectRenderingLoop 0x006D8DB0` (inside `TacticalClass::Draw 0x006D3D10`, render side, downstream of the tick). The walk itself is a `drawing-helpers` responsibility; this service owns the **membership state** it walks.
- **Membership churn ties to spine Rung T** (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` Rung T, MAIN object vector tick, driver `0x005F3E70`): inside each object's `ObjectClass::AI`, if `vtable+0x78` (InWhichLayer) returns a different layer than before AI ran, the object is re-submitted (call site `0x005F400E`). In-layer **position** changes do NOT go through AI — the **locomotor** does the Remove → Set-coords → Submit each move tick (e.g. `FlyLocomotionClass::Process 0x004CD792`), which is also driven from Rung T (locomotor piggyback). This keeps Layer 2 incrementally sorted across ticks rather than rebuilt per frame.
- **Lifecycle churn ties to reveal/conceal** (out of the rung ladder, but sim-side): `ObjectClass::Reveal 0x005F4EC0` submits, `ObjectClass::Conceal` removes, `DropIn`/`BulletClass::Fire`/anim-attach also submit/remove. So the list mutates from sim-lifecycle events, NOT from the render thread.
- **No deterministic state / not in the state hash.** The layer vectors carry render ordering only; the lockstep sim state is the LogicClass vector + object fields, not these. (Layer ORDER is, however, persisted to save games — primary §5b — so a loaded game does not re-sort Layer 2.)
- **Active in YR: YES** — core per-frame object draw dispatch, not gated behind any flag. No TS-legacy gating found. (The Underground layer [0] is populated by tunnel/subterranean locomotion, which is TS-legacy/absent in stock YR — so that one BUCKET is effectively empty in stock skirmish, but the layer array and the service are fully live.)

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `abstract-object` | `ObjectClass::InWhichLayer` (vtable+0x78) picks the bucket; `ObjectClass::GetYSort 0x005F6BD0` (vtable+0xB8) provides the Layer-2 sort key; per-object `+0x94 LayerIndex` / `+0x99 wasDrawn` are ObjectClass fields this service reads/writes; `Reveal 0x005F4EC0` / `Conceal` are the lifecycle hooks that submit/remove. | The draw list is keyed entirely on base ObjectClass virtuals + fields; Submit/Remove dispatch through them. LAYER_CLASS §4/§5/§9; primary §2 (per-object state). |
| `cell-map` | DisplayClass IS a base of MapClass→Display→Radar→Tactical; the layer vectors are owned within that DisplayClass hierarchy (`Init_Clear` chain `MapClass::Init_Clear 0x005659F0` → `DisplayClass::Init_Clear 0x004A88C0`). Layer assignment correlates with the object's cell (locomotor Remove/Submit on cell-list relink, `vtable+0x124` 0/1 around Set-coords). | The map/display owner holds the 5 layer vectors and the per-scenario clear; the cell-list relink is paired with layer re-submit. LAYER_CLASS §5c; primary §5/§7. |
| `techno-foot` | Locomotor piggyback (`FlyLocomotionClass::Process 0x004CD792` and drive helper `FUN_004CD2A0 0x004CD4E7`) calls Remove_From_Layer + Submit_Object every move tick; `BulletClass::Fire 0x00468B6D`, `AnimClass::SetOwnerObject 0x00424C00/0x00424C7C` also re-submit. | Most per-tick churn comes from FootClass/Techno locomotors re-submitting on move; the move-triggered re-sort keeps Layer 2 current. LAYER_CLASS §5 (call sites); §"move-triggered re-sort". |
| `drawing-helpers` | The walk + Y-sort comparator (`ObjectClass::YSortComparator 0x005F6220`, `Tactical_ObjectRenderingLoop 0x006D8DB0`) live in drawing-helpers; this service supplies the membership the walk consumes (and shares the `g_DisplayLayers` global with it). | Two profiles split one mechanism: frontier-render-layer = the list + churn; drawing-helpers = the walk + per-pixel draw + sort comparator. Shared global `0x008A0360`. drawing-helpers.md §"Owns"/§"Depends-on". |
| `frontier-saveload` | `DisplayClass::Save 0x004AE720` / `Load 0x004AE6F0` serialize all 5 layers (pointer-swizzle via `FUN_006CF240`), nested under `RadarClass::Save/Load`. | Layer order is part of the save image, so it round-trips the save/load swizzle path. LAYER_CLASS §5b. |

---

## Used-by (incoming edges)

| Source slug | Via symbol | Evidence |
|---|---|---|
| `frontier-render-tactical` | `Tactical_ObjectRenderingLoop 0x006D8DB0` (called from `TacticalClass::Draw 0x006D3D10`) walks `g_DisplayLayers` 0→4 each frame to draw objects in z-order. | The tactical pass is the sole reader of the draw list; without it the list is inert. LAYER_CLASS §7; primary §1. |
| `drawing-helpers` | The two-pass object render loop + Y-sort comparator iterate the layer vectors and read `+0x99 wasDrawn`; drawing-helpers OWNS the walk, this service owns the data it walks. | Mutual/adjacent: the consumer is drawing-helpers, which also lists `g_DisplayLayers` ownership-by-DisplayClass. drawing-helpers.md §"Owns"/§"Depends-on (cell-map)". |
| `abstract-object` | `ObjectClass::AI` (re-layer at `0x005F400E`), `Reveal`/`Conceal`/`DropIn`, `BulletClass::Fire`, `AnimClass` attach/detach all call Submit_Object / Remove_From_Layer — the object lifecycle DRIVES membership. | The object hierarchy is the writer; this service is the list those writers mutate. LAYER_CLASS §5 (13 Submit + 14 Remove call sites). |
| `techno-foot` | Locomotors (FootClass piggyback) re-submit on every move tick; this is the dominant runtime churn source. | Movement is the high-frequency writer into the draw list. LAYER_CLASS §5. |
| `factory-house` (transitive) | building placement/sell → `ObjectClass::Reveal`/`Conceal` → Submit/Remove. | Indirect: any object entering/leaving the map mutates the list via reveal/conceal, including buildings. LAYER_CLASS §5. |

---

## Active-in-YR / TS-legacy

- **Fully active in YR.** `LayerClass__Constructor` runs at boot (5 iterations); the render loop walks the layers every frame; not flag-gated. (Primary doc "Active in YR: Yes".)
- **TS-legacy nuance:** Layer **0 (Underground)** is for subterranean/tunnel locomotion, which is TS-legacy and absent in stock YR (`feedback_no_tunnel_subterranean`). The layer array, the service, and layers 1–4 are all live; only that one bucket is effectively empty in stock skirmish. Do NOT treat the Underground layer as a reason to implement tunnel logic.
- **Layer-index residual (cross-doc):** LAYER_CLASS §11 places airborne aircraft in layer **4 (Top)**, while an ANIMCLASS draw-traversal fallback uses layer **3 (Air)**; both are unsorted (submission-order) so observable ordering is unaffected, but the exact per-class layer index is an open `cell-map`/`abstract-object` detail (also flagged in drawing-helpers.md "Open / unverified edges").

---

## Open / unverified edges

- **Live re-verify owed (Ghidra offline this session).** All representative addresses (`0x004A862A`, `0x004A9720`, `0x004A9770`, `0x008A0360`) are corroborated from the primary report's prior decompilations, NOT re-verified by a LIVE Ghidra call THIS session. A confirming `decompile_function 0x004A9720` + `get_xrefs_to 0x004A9720` pass should be run when an instance is available, per the project's "treat your own prior claims as unverified" rule.
- **Layer-2 sort KEY equivalence vs Rust.** Primary doc D8/D8a/D8b: gamemd sorts Layer 2 by lepton `X+Y` (Z excluded) with per-class YSort bias (Building `+0x20/-0x10`, Anim `+0x104`), and does NOT sort layers 0/1/3/4. The Rust port currently rebuilds flat instance lists per frame and sorts by a `screen_y + z·HEIGHT_STEP` key — a different ordering contract. Result-equivalence is **DRIFT** until proven (drawing-helpers carries the same D8 ledger).
- **FIFO equal-key tie-break.** SortedInsert inserts equal-key elements FIFO (insertion order); the Rust stable sort must reproduce the same tie-break direction for sprite-vs-sprite occlusion at equal X+Y. UNCHECKED at pixel level.
- **Incremental-vs-rebuild ordering drift.** gamemd keeps Layer 2 sorted incrementally (Remove/Submit per move); Rust rebuilds + re-sorts per frame. These can diverge only if the per-frame full sort produces a different equal-key order than the incremental insertion history — UNCHECKED, but a candidate for a 1-pixel draw-order disparity.
