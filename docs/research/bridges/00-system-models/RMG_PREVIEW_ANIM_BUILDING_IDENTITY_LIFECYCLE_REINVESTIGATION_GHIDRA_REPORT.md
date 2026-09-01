# Active-retail RMG preview Anim/Building identity lifecycle — reinvestigation report

Date: 2026-08-31

Binary: active retail Yuri's Revenge `gamemd.exe`, image base `0x00400000`

Mode: random-map setup **preview Generate/OK generation**, not `.SED` gameplay launch

System ownership hypotheses: GSI-04.12 / GSI-04.13 terrain-attached bridge animation dependency; GSI-04.15 negative boundary; GSI-01.07 Scenario RNG, GSI-01.12 object identity/lifetime, shell RMG, Anim, and audio dependencies

Status: **native behavior verified for this bounded lifecycle; current Rust behavior remains open**

## Verdict

The active setup preview is not a small or inert rendering pass. The dialog calls
`RandomMapGenerator__Generate @ 0x00598960` with preview argument `1`. That value is
forwarded to `InitMapFromSyntheticINI @ 0x00599650`, which takes the
`ScenarioClass::Set_Defaults @ 0x00683610` plus manual-setup branch. It does **not**
call `ScenarioClass::Full_Init @ 0x00686B20`, `Clear_Scene @ 0x006851F0`, or the
map-read unique-ID reservation on this path. Those are launch-only effects for the
`.SED` generation call with argument `0`.

`Main__PrepareSession @ 0x0052D9A0` sets `g_GameActive @ 0x00A8E9A0` to `1` before
the shell dialogs run. Consequently, preview-created Buildings and tile Anims take
their ordinary active `Unlimbo`, Object/Display/Logic/Anim registration, cell-latch,
and sound paths. The preview flag does not skip the later generator-wide
`CellClass::RecalcAttributes` calls or terminal `MapClass::InitCellAttributes(1)`.

Every preview Generate setup first frees the old tiberium spread queue and then the
growth queue, then resets `ScenarioClass+0x214` to `1,000,000`. The numeric prefix
after that reset is branch-specific. An exact four-field storage match skips Resize,
type reconstruction, House/Super reconstruction, and theater reload, so the first
new Building/Anim can receive `1,000,001`. Missing/changed storage first constructs
all real Size-diamond Cells plus the dummy Cell, rebuilds ID-bearing types, and
constructs Houses and their Supers. Every actually constructed Building then
consumes a raw Scenario RNG word in the Techno constructor, preincrements/stores one
native unique ID, and only later reaches placement outcome. Every actually
constructed tile Anim preincrements/stores an ID before optional Scenario
`RandomRate`, enters active registries, and can start sound. Failed Building
placement refunds neither effect. Stock bridge/tunnel tile Anim declarations have
no `RandomRate`, so they consume IDs but zero Scenario RandomRate draws; four stock
waterfall heads declare `StartSound=WaterfallLoop`.

Replacement has two distinct native lifetimes:

- A changed four-field storage key, or a missing snapshot such as first Generate
  after dialog re-entry, invokes the full object cleanup. Old preview Anims and
  their valid sound handles die before new generation.
- A matching storage key does not delete Anim objects in the early selective
  cleanup. Old final tile Anims, cell latches, and valid sounds survive until the
  terminal `InitCellAttributes(1)` pass, where marked old/new tile Anims are
  destructed, latches are cleared, and the final set is reconstructed.

Cancel/common dialog teardown destroys only presentation/snapshot owners. It does
not clear the map, delete Buildings/Anims, release their sounds, or reset/roll back
`ScenarioClass+0x214`. Final preview objects and the advanced counter persist across
Cancel and a no-Generate re-entry. Final generated tiberium growth/spread queues
persist with them. The first subsequent Generate frees spread then growth, resets
the counter and, because the snapshot was destroyed, performs full cleanup and the
Cell/type/House prefix before generating a new candidate; it later rebuilds growth
then spread.

The Rust shell currently represents the presentation candidate and Building RNG
replay, but not this native preview object/identity lifetime. It also assigns an
Anim's collision-free stable ID after `RandomRate` and aliases that stable ID to
`native_unique_id`. Native requires the opposite order and permits a same-storage
replacement window in which a reset native numeric counter can reuse values while
old Anims, Types, Houses/Supers, real/dummy Cells, and other untouched Abstracts are
still live. Rust therefore needs a separate preview lifecycle owner, including the
generated growth/spread queues, and separate collision-free runtime handles versus
native numeric IDs for every live class.

## Scope and exclusions

Included:

1. the exact dialog preview argument and branch through synthetic-map setup;
2. later Recalc/InitCellAttributes execution in preview;
3. tile-Anim identity, optional Scenario RNG, registry, latch, and sound effects;
4. generated Building constructor word, ID, and placement outcome order;
5. all unique-counter writers/resets that can affect preview setup, replacement,
   Cancel, teardown, and re-entry;
6. final preview Anim/sound lifetime on both replacement branches and Cancel;
7. retail INI declarations that make those conditional effects active;
8. matching versus changed/missing Cell/Type/House native-ID prefixes and retained
   duplicate-ID scope;
9. generated tiberium queue lifetime through replacement, Cancel, and launch;
10. current Rust ownership and order mismatches.

Excluded except where needed to distinguish the preview branch:

- `.SED` gameplay generation formulas and all later gameplay bootstrap;
- authored fixed-map object order outside the comparative Full_Init prefix;
- general Anim gameplay AI, damage, looping, and expiration after launch;
- TS-only generator behavior and dormant inherited object types;
- pixel rendering of `RandMap.img`;
- Ghidra metadata changes.

OpenTS was used only to find likely correspondences. Every material result below was
cold-checked in active `gamemd.exe` and, where data-controlled, in YR retail INIs.

## Coverage ledger

| Requested mechanism | Active-retail result | Evidence | Closure |
|---|---|---|---|
| Preview argument into Generate | dialog Generate and implicit OK push `1`; editor and `.SED` launch push `0` | `0x00596649..0x0059664C`, `0x00596A49..0x00596A6B`, `0x00684989` | RESOLVED |
| Synthetic setup branch | preview reaches Set_Defaults/manual setup and excludes Full_Init | `0x00598A73..0x00598A74`, `0x00599650`, `0x00599A3E..0x00599B23` | RESOLVED |
| Launch-only prefix | argument `0` reaches Full_Init; its map-read path reserves `+10,000` IDs | `0x00599A56`, `0x00686B20`, `0x004ACE70`, `0x004AD026`, `0x004AD05F` | RESOLVED negative boundary |
| Preview active-object gate | `g_GameActive=1` before shell; ordinary Unlimbo is enabled | `0x0052D9D7`, `0x005F4EC0`, `0x006F6CA0`, `0x00440580` | RESOLVED |
| Later Recalc phases | all common generator Recalc sites execute in preview | `0x00598E48`, `0x00598FE7`, `0x00599153`, `0x0059937D`; conditional helper `0x005A4259` | RESOLVED |
| Terminal cell initialization | preview calls `InitCellAttributes(1)` | push/call at `0x0059943F..0x0059944C`, callee `0x00568BB0` | RESOLVED |
| Anim ID/RNG order | AssignUniqueID/register precede optional Scenario RandomRate | `0x00421EA0`, `0x00410230`, `0x0068BCB0` | RESOLVED |
| Stock RandomRate | all 20 active tile Anim names have absent/zero RandomRate | theater INIs plus `ini/artmd.ini` | RESOLVED: zero stock draws |
| Stock sound | WA01X/WB01X/WC01X/WD01X declare WaterfallLoop | `ini/artmd.ini:18670..18842` | RESOLVED, audio-conditional |
| Building word/ID/outcome | raw Scenario word, then unique ID, then placement result | `0x006F3254..0x006F3259`, `0x0043BA15`, RMG owners | RESOLVED |
| Counter resets/increments | reset on each Generate; exact-match has no setup constructor; rebuild spends Cells/types/Houses/Supers; no rollback | `0x00683633`, `0x0068BCB7`, `0x00565C10`, rules/House constructors | RESOLVED |
| Matching-key ID prefix | skips Resize/type/House/theater helpers; first new object can receive `1,000,001` while retained Abstract IDs remain live | `0x00599BFB..0x00599D95` | RESOLVED |
| Changed/missing ID prefix | real Cells row-major plus dummy, ID-bearing types, Houses/Supers, then theater K; active-retail K=0 | `0x00599C62..0x00599D95`, `0x00565C10` | RESOLVED |
| Matching-key replacement | selective cleanup omits Anim; terminal pass deletes/recreates | `0x00599650`, `0x00568BB0`, `0x00422900` | RESOLVED |
| Changed/missing-key replacement | full cleanup deletes old Anim/sounds before generation | `0x00599650`, `0x00534450`, `0x00422900` | RESOLVED |
| Cancel/common teardown | presentation/snapshot only; no world cleanup/counter reset | `0x00595BC0`, `0x005E8590` | RESOLVED |
| Tiberium queues | Generate frees spread then growth before reset/branch, later rebuilds growth then spread; final queues survive Cancel/re-entry | `0x00599A13`, `0x00599A18`, `0x0059939B`, `0x005993A0` | RESOLVED |
| Re-entry | no Generate preserves state; first later Generate resets then full-cleans | snapshot creation/destruction and branch predicate | RESOLVED |
| Current Rust parity | no preview native-ID/object/sound owner; replay/order/lifetime differ | Rust sources cited below | OPEN FOR IMPLEMENTATION |

## 1. Exact preview branch; Full_Init is launch-only here

### 1.1 Call-site argument proof

`RandomMapGenerator__Generate @ 0x00598960` is a `MapSeedClass` receiver call.
The relevant callers supply:

| Caller | Stack arguments immediately before call | Meaning |
|---|---|---|
| setup dialog Generate | `PUSH hDlg` at `0x00596649`, `PUSH 1` at `0x0059664A`, call `0x0059664C` | active preview |
| setup OK when no candidate exists | `PUSH hDlg`, `PUSH 1`, call at `0x00596A66` | generate preview candidate, then accept |
| editor-oriented branch in the same command handler | `PUSH hDlg`, `PUSH 0`, call at `0x00596A49` | non-preview/tool boundary |
| accepted `.SED` scenario launch | `PUSH 0`, `PUSH 0`, call at `0x00684989` | full scenario generation |

Generate forwards the incoming Boolean at `0x00598A73` to
`InitMapFromSyntheticINI @ 0x00599650` at `0x00598A74`; it is not replaced by a
derived flag.

### 1.2 The two synthetic initialization paths

In `InitMapFromSyntheticINI`:

- argument `0` calls the scenario/map preparation helper at `0x00599A3E`, calls
  `Clear_Scene @ 0x006851F0` at `0x00599A4B` only for the editor-mode subcase,
  and then calls `ScenarioClass::Full_Init @ 0x00686B20` at `0x00599A56`;
- argument nonzero calls `ScenarioClass::Set_Defaults @ 0x00683610` at
  `0x00599B23`, followed by the preview's manual map-storage and cell setup.

`Full_Init` itself calls `Clear_Scene` at `0x00686B65`. Therefore the ordinary
preview path reaches neither Full_Init nor its Clear_Scene/map-read prefix.

This corrects the generated-preview statement in
`TERRAIN_ATTACHED_ANIM_LOAD_LIFECYCLE_SIDE_EFFECTS_REINVESTIGATION_GHIDRA_REPORT.md:109`:
synthetic **launch** reaches Full_Init; setup **preview** does not. The later common
generator Recalc and terminal InitCellAttributes effects remain active in preview.

### 1.3 Why preview objects are active

A tempting but wrong inference is that shell preview runs with `g_GameActive=0`.
Cold assembly at `Main__PrepareSession @ 0x0052D9A0` shows:

```text
0052D9D7  MOV byte ptr [0x00A8E9A0],0x1
```

This occurs at function entry before the shell/dialog loop. Unrelated launcher,
network/gameplay, game-exit, and shutdown owners can temporarily or permanently
clear/restore the byte, but none is on the traced random-map setup corridor; the
active retail setup dialog is entered with it set.

`ObjectClass::Unlimbo @ 0x005F4EC0` stores the coordinate, marks the object,
submits it to Display, and conditionally registers it with Logic when this gate is
active. `BuildingClass::Unlimbo @ 0x00440580` reaches it through
`TechnoClass::Unlimbo @ 0x006F6CA0`. Anim construction uses the same active Object
path. Preview-created objects are consequently real registered objects, not merely
heap allocations used to paint the bitmap.

## 2. Preview generator phases and terrain Anim effects

### 2.1 Recalc and terminal phases are common, not preview-gated

After the preview/manual setup returns, the generator executes its common pipeline.
Direct whole-map `CellClass::RecalcAttributes @ 0x0047D2B0` call sites occur at:

1. `0x00598E48`, after bridge/CABHUT generation and before neutral-tech placement;
2. `0x00598FE7`;
3. `0x00599153`;
4. `0x0059937D`.

The temperate/theater helper has an additional conditional direct Recalc call at
`0x005A4259`. The generator finally pushes `1` at `0x0059943F` and calls
`MapClass::InitCellAttributes @ 0x00568BB0` at `0x0059944C`. None of these common
sites is bypassed by the preview Boolean.

### 2.2 Actual tile-Anim constructor transaction

When Recalc resolves an eligible tile extra-data animation and the cell's latch is
clear, `AnimClass::Constructor @ 0x00421EA0` performs this material order:

1. its Object base constructor joins the process object vectors and initializes
   sound-handle state;
2. Anim vtables are installed;
3. `AbstractClass::AssignUniqueID @ 0x00410230` obtains the next Scenario native
   unique ID and stores it;
4. the object is appended to `g_AnimClass_Array @ 0x00A8E9AC` with count at
   `0x00A8E9B8`;
5. if either AnimType RandomRate endpoint is nonzero and `minimum <= maximum`, one
   inclusive Scenario RNG draw selects the rate;
6. active Unlimbo registers/places the object at the tile coordinate;
7. a zero-delay terrain Anim reaches `AnimClass::Middle @ 0x00424CE0`.

The Recalc producer then marks the object as terrain-attached (`Anim+0x196=1`,
`Anim+0x197=1`), applies its Z adjustment, and sets bit `0x00020000` in
`CellClass+0x140`. That latch prevents a later Recalc from constructing a second
tile Anim on the same cell until destruction clears it.

`AnimClass::Middle` marks the object and, when the relevant sound state is clear and
AnimType `StartSound` is valid, gets the placed coordinate and calls
`VocClass::PlayAt @ 0x007509E0`. `PlayAt` has no `g_GameActive==0` preview
suppression. A handle is material only when normal audio enablement, lookup, and
volume admission succeed.

### 2.3 Retail-data activation

An active-line census of `Tile##Anim=` in retail theater INIs yields exactly 20
names:

```text
TUNTOP01 TUNTOP02 TUNTOP03 TUNTOP04
WA01X WA02X WA03X WA04X
WB01X WB02X WB03X WB04X
WC01X WC02X WC03X WC04X
WD01X WD02X WD03X WD04X
```

The commented `;Tile05Anim=UFO` line is not active data and is excluded.

In both the retail base `ini/art.ini` sections and their YR `ini/artmd.ini`
counterparts:

- TUNTOP01..04 have `Rate=0`;
- all 16 waterfall declarations have `Rate=320`;
- WA01X, WB01X, WC01X, and WD01X have
  `StartSound=WaterfallLoop`;
- none of these 20 sections declares `RandomRate` or `StopSound`.

Thus every stock preview tile-Anim construction still consumes one native unique
ID and performs registration. It consumes **zero** Scenario RandomRate draws.
Eligible waterfall-head cells also attempt one positioned WaterfallLoop start.
Valid custom content using an active Tile##Anim name with a passing RandomRate
range consumes one Scenario draw per actual construction through the same active
code path.

## 3. Generated Building word, identity, and outcome

`TechnoClass::Constructor @ 0x006F2B90` directly calls the Scenario raw-word
generator at `0x006F3254` and stores the low word at Techno offset `+0x3C8` at
`0x006F3259`. After Building vtables are established,
`BuildingClass::Constructor @ 0x0043B740` calls
`AbstractClass::AssignUniqueID` at `0x0043BA15`.

The native order for every actual generated Building construction is therefore:

```text
Scenario raw word -> store Techno event word -> preincrement/store native unique ID
-> try placement/Unlimbo -> keep or destroy according to that owner
```

This order is independent of placement success.

### 3.1 Neutral-tech owners

The general neutral-tech owner at `0x005A96F8` and the type-2 region owner at
`0x005954A1` construct the Building before their bounded placement attempts. If no
attempt succeeds, scalar deletion removes the object, but neither the Techno word
nor the native ID is refunded. A successful object remains in the Building,
Object, Display, and applicable Logic registries.

### 3.2 CABHUT distinction

The CABHUT helper at `0x005904B0` searches for a qualifying cell before
construction. Search failure constructs nothing and consumes neither a Scenario
word nor an ID. Once it constructs a CABHUT, the normal Techno-word and Building-ID
transaction has already happened. The helper calls Unlimbo and does not branch on
the returned value before reporting success; stock active one-cell placement is
the normal successful case.

The distinction matters for trace shape: `search failed before construction` must
not be represented as a discarded constructor event, while `constructed, later
placement failed` must retain both side effects.

## 4. Scenario native unique-ID counter writer census

The native counter is `ScenarioClass+0x214`. It is not the Scenario RNG state at
`+0x218`.

| Writer | Exact effect | Preview-corridor reachability |
|---|---|---|
| `ScenarioClass::Set_Defaults @ 0x00683610` | writes decimal `1,000,000` at `0x00683633` | **Every preview Generate**, before replacement cleanup |
| `ScenarioClass::Clear_Scene @ 0x006851F0` | writes `1,000,000` near entry at `0x006851FC`, calls Set_Defaults, writes it again at `0x00685659` | not preview Generate; Full_Init/launch and other scene teardown only |
| `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70` | saves the current counter at `0x004AD026`, later writes saved value `+0x2710` at `0x004AD05F` | Full_Init/map-read launch prefix only; excluded from preview |
| `ScenarioClass::NextUniqueID @ 0x0068BCB0` | preincrements and writes at `0x0068BCB7`, returns the new value | every preview Building/Anim construction |
| `FootClass::ClickedAction_Cell @ 0x004D7F35` | saves counter, writes `-3` at `0x004D7F42` for move-feedback Anim construction, restores at `0x004D7FCB` | gameplay click only; not modal RMG preview generation |
| Scenario stream restoration | serialized state can restore the field | save/load corridor only; not dialog preview |

A direct instruction search for `MOV` writers using displacement `0x214`, followed
by receiver/caller resolution, found no additional writer in the preview setup,
replacement, dialog teardown, or re-entry corridor. Most raw displacement hits
were fields of unrelated classes or stack aliases.

`AbstractClass::AssignUniqueID @ 0x00410230` simply calls NextUniqueID and stores the
returned dword. It performs no live-object collision check and no rollback.

### 4.1 Preview versus launch numeric prefix

Preview setup first calls `TiberiumClass__FreeSpreadQueues_All` at `0x00599A13`
and `TiberiumClass__FreeGrowthQueues_All` at `0x00599A18`. It then calls
Set_Defaults, placing the counter at `1,000,000` before the storage-key decision.

On an exact four-field match, preview skips full cleanup, Resize, rules/type
reconstruction, House reconstruction, and theater reload. The selective Cell
payload iterator does not write `AbstractClass+0x10`, and it does not reconstruct
the dummy Cell. No AssignUniqueID constructor intervenes before the generator:

```text
C_preview_match_before_generator = 1_000_000
first new generator object ID     = 1_000_001
```

On missing/changed storage, preview performs full cleanup and then constructs the
prefix in this order:

```text
Seed(1_000_000)
-> R(W,H) Cell/dummy events
-> |P_preview| ID-bearing type events
-> HB(H_preview,S_preview) House/Super events
-> K_preview theater AnimType events
```

where `R(W,H)=H*(2W-1)+1`, `HB(H,S)=H*(1+S)`, and:

```text
C_preview_rebuild = 1_000_000
                    + R(W,H)
                    + |P_preview|
                    + HB(H_preview,S_preview)
                    + K_preview                  // wrapping u32

first generator object ID = C_preview_rebuild + 1
```

All 176 active `Tile%02dAnim` rows (20 distinct names) in the six retail YR
theater INIs already exist in `rulesmd.ini [Animations]`, so active-retail
`K_preview=0`. Custom theater data retains that constructor arm.

Launch Full_Init instead clears the scene, then its map-read owner applies the
saved-counter `+10,000` reservation before later allocations. Preview must not
import that launch-only reservation or any Full_Init-created transient tile-Anim
prefix. The complete fresh Full_Init source-dependent prefix is proved separately
in `../01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`.

### 4.2 Lifecycle matrix

| Boundary | Counter state | Object consequence |
|---|---|---|
| before first Generate | whatever process shell state currently holds | no preview-specific writer yet |
| each preview Generate setup | free old spread, then growth; Set_Defaults writes `1,000,000` | all three occur **before** deciding full versus selective replacement cleanup |
| exact-match preparation | unchanged at `1,000,000` | retains live Type/House/Super/Cell/dummy/Anim IDs; no setup constructor |
| missing/changed preparation | advances by `R+P_preview+HB+K_preview` | full cleanup, row-major Cells/dummy, types, Houses/Supers, theater |
| Building constructor | `+1` | spent even if later placement/deletion fails |
| tile-Anim constructor | `+1` | spent even if terminal InitCellAttributes later deletes it |
| optional Anim RandomRate | counter unchanged | separate Scenario RNG draw only |
| terminal delete/recreate | deletion does not change counter; every recreated final Anim adds `+1` | transient IDs remain spent |
| end of Generate | final advanced value persists | growth then spread queues are rebuilt; final candidate objects/queues remain live |
| another Generate, same dialog | frees spread/growth, then resets to `1,000,000` | previous candidate's total does not seed the new one |
| Cancel/common teardown | counter and queues unchanged | no rollback, object cleanup, or queue free |
| re-entry without Generate | counter and queues unchanged | no snapshot clone is created merely by opening |
| first Generate after re-entry | free spread/growth, reset, then full cleanup and rebuild prefix | old objects die after reset; new generator starts after prefix; growth/spread rebuild at end |
| accepted `.SED` launch | initial spread/growth free, then independent Clear_Scene/Full_Init/map-read prefix | Clear_Scene frees both queues again; launch rebuilds growth then spread |

The Scenario RNG at `+0x218` is different: Set_Defaults does not turn the above
counter reset into a Scenario RNG rewind. Building words and custom RandomRate draws
therefore advance the process Scenario cursor even though the numeric ID counter is
reset for every candidate.

## 5. Replacement lifetime

### 5.1 Snapshot identity and the two cleanup branches

`MapSeedClass+0x178` aliases `DAT_00ABE150`, the heap snapshot clone retained by an
open setup dialog. Generate replaces the clone after generation at
`0x00596705..0x00596742`; common dialog teardown destroys it at
`0x00595CB2..0x00595CC2`. WM_INIT does not allocate it.

After Set_Defaults, `InitMapFromSyntheticINI` compares the current record with the
snapshot using four fields at `+0x64`, `+0x68`, `+0x38`, and `+0x50`: normalized
width, height, theater, and player count. Seed, map type, and other generation
options do not define storage identity.

This matches the current Rust `RandomMapStorageKey`; that four-field key should be
preserved.

### 5.2 Changed key or missing snapshot

If the snapshot is missing or any storage-key field differs, preview setup removes
IsoTile types and calls the full cleanup owner `FUN_00534450`. Its Anim registry
loop at `0x005346C3` calls each object's scalar-deleting virtual destructor. The
verified `AnimClass::Destructor @ 0x00422900`:

- removes/limbos the active object and removes it from display/registries;
- clears the tile cell's `0x00020000` latch when applicable;
- releases two sound-event handles and detaches two VOC handles;
- removes the object from the Anim array and chains to Object destruction.

It does not invoke AnimType `StopSound`; handle release/detach is the cleanup.
Therefore valid old WaterfallLoop handles end before new generation begins on this
branch.

Set_Defaults ran before this cleanup. Destruction does not restore the old counter.
Resize then constructs every admitted real Cell in row-major order and the dummy
Cell, followed by ID-bearing type reconstruction, House/Super reconstruction, and
the theater type arm. The new generator therefore starts after the branch-specific
prefix in section 4.1, not unconditionally at the reset base.

### 5.3 Matching key

When all four storage fields match, full cleanup is skipped. Preview setup still
explicitly deletes Unit, Infantry, Building, and Terrain registries, but there is
no early Anim loop. Its cell payload reset writes fields including `+0x11C`,
`+0x11B`, `+0x38`, `+0x11A`, `+0x44`, and `+0x11E`; it does not clear the
terrain-Anim latch at `Cell+0x140`, overwrite any real Cell's native ID at `+0x10`,
or reconstruct the dummy Cell. The branch also skips type and House/Super helpers,
so those objects retain their native IDs.

Consequences through the next generation:

1. old preview Buildings are gone early;
2. Types, Houses/Supers, real Cells, the dummy Cell, and other untouched Abstracts
   retain their prior numeric IDs;
3. old final terrain Anims remain registered;
4. their cell latches suppress new tile-Anim construction on those cells during
   intermediate Recalc passes;
5. their valid sound handles remain active;
6. newly eligible, previously unlatched cells can construct new marked tile Anims;
7. terminal `InitCellAttributes(1)` scans the Anim registry and scalar-deletes the
   marked old/new terrain Anims;
8. destruction releases handles and clears latches;
9. the terminal anti-diagonal cell sweep reconstructs the final tile-Anim set,
   assigning fresh native IDs and attempting fresh StartSound playback.

### 5.4 Native numeric ID reuse window

This branch resets `Scenario+0x214` while retained TypeClass, HouseClass,
SuperClass, real/dummy CellClass, old AnimClass, and other untouched Abstract IDs
remain live. AssignUniqueID has no collision query. Native numeric IDs are globally
non-unique in this window, not merely non-unique among Anims. After a successful
missing/changed rebuild, the first retained real Cell owns `1,000,001`; the next
exact-match first generator constructor also receives `1,000,001`, making that
cross-class overlap deterministic whenever such a constructor occurs. Further
Building/Anim overlaps with retained objects depend on construction counts.

Rust must not force an early cleanup or alter constructor order merely because its
object store requires collision-free keys. Use a collision-free runtime handle as
the collection key and carry `native_unique_id` as a separate parity field.

## 6. Cancel, teardown, accept, and re-entry

### 6.1 What common dialog teardown actually destroys

`RandomMapSetupDialog__Run @ 0x00595BC0` performs common close work for OK and
Cancel:

1. if the preview surface wrapper at `DAT_00ABE154` exists, writes `RandMap.img`;
2. destroys and nulls that wrapper;
3. destroys and nulls the MapSeed snapshot at `DAT_00ABE150`;
4. returns the dialog result.

There is no call to Clear_Scene, Set_Defaults, full object cleanup,
InitCellAttributes, Anim destruction, a `Scenario+0x214` writer, or either
tiberium-queue free routine.

`ChooseMap__AcceptRandomMapSetup @ 0x005E8590` calls the runner and immediately
returns `-1` when the result is not exactly `1`. Cancel returns `2`. The parent does
not add a hidden cleanup after the common teardown.

### 6.2 Native state after Cancel

After Cancel, the following final-candidate state remains live:

- generated Buildings and their object/display/logic registrations;
- final terrain Anims and their Anim/object/display/logic registrations;
- terrain-Anim cell latches;
- any normally admitted WaterfallLoop handles;
- the advanced Scenario native unique-ID counter;
- retained Types, Houses/Supers, real/dummy Cells, and other untouched Abstract
  native IDs;
- the final generated growth and spread queues;
- all Scenario RNG advancement already spent by Building words and any custom
  RandomRate calls.

Only dialog presentation/snapshot ownership is gone. Cancel itself neither plays
StopSound nor releases a terrain Anim's sound handle.

### 6.3 Re-entry and next Generate

Reopening setup without pressing Generate creates no MapSeed snapshot and performs
no world cleanup or counter write. Closing that dialog again preserves the same
native state.

The first Generate after re-entry performs these ordered effects:

```text
free old spread queue, then growth queue
-> Set_Defaults: ID counter = 1,000,000
-> snapshot is absent, so full cleanup destroys old preview objects/sounds
-> construct real Cells/dummy, types, Houses/Supers, and theater type arm
-> generate the new candidate and spend new IDs
-> rebuild growth queue, then spread queue
-> allocate the replacement snapshot after generation
```

This ordered reset-before-destruction boundary is observable to an identity model
even though the old objects are gone before new construction begins.

### 6.4 Accepted Use Map

If an existing preview candidate is accepted, dialog close does not generate it a
second time and does not clean its native objects. The preview state remains until
the later `.SED` gameplay launch takes the independent Full_Init/Clear_Scene path.
That launch cleanup and `+10,000` reservation belong to the launch transaction, not
to preview acceptance. Launch initially frees preview spread/growth queues at the
same synthetic-setup boundary; Clear_Scene frees them again, and the later Full_Init
tail rebuilds growth then spread for gameplay.

## 7. OpenTS navigation leads, checked against retail

The readable OpenTS tree at `C:\Users\enok\Documents\OpenTS` supplied these leads:

- `code/mapgen.cpp`: preview reset, four-field storage comparison, conditional
  Delete_All_Objects, unconditional Unit/Infantry/Building/Terrain deletion, and
  omission of Anim from the selective loop;
- `code/display.cpp`: save the unique counter and reserve `+10000` during map read;
- `code/scenario.cpp`: Reset/Clear base `1,000,000` and preincrementing ID getter;
- `code/abstract.cpp`: object Create_ID delegates to Scenario;
- `code/foot.cpp`: temporary `-3` feedback identity with save/restore;
- `code/anim.cpp`: ID/registry/RandomRate/Unlimbo/Start ordering.

None of those source correspondences is used as parity authority. The report's
addresses, branch outcomes, active gate, counter effects, cleanup behavior, and
retail activation were independently checked in active `gamemd.exe` and YR data.
TS-only names, legacy Vein behavior, and dormant inherited paths were excluded.

## 8. Current Rust correspondence and corrections

### 8.1 What already matches and should be preserved

- `src/app/shell_random_map.rs:31` defines the correct four-field
  `RandomMapStorageKey`.
- the MapSeed snapshot lifetime is represented separately from the generated
  candidate and is destroyed at dialog close.
- `src/app/shell_random_map.rs:83..96` correctly treats the explicit-clear Fill
  range `(0,0)` as zero Scenario-cost. That narrow fact remains true; it does not
  account for later Building/Anim side effects.
- `src/map/construction_trace.rs` distinguishes constructed Building events from
  pre-construction absence and keeps discarded constructions.
- presentation `cancel_setup` clearing the UI candidate is reasonable; the missing
  state is a separate native preview-object owner, not a reason to keep presenting
  a cancelled candidate.

### 8.2 Verified mismatches

| Rust evidence | Mismatch | Required correction |
|---|---|---|
| `src/app/frontend/skirmish_session.rs:86..92` | runtime owns Scenario RNG and MapSeed options, but no Scenario native-ID counter or preview object lifetime | add a process/shell preview native-lifecycle owner |
| `src/app/frontend/skirmish_session.rs:515` | Building-only trace replay advances one raw Scenario word and records no result or ID | record/apply the raw word, ID assignment, outcome, and ordered Anim events |
| `src/app/shell_random_map.rs:377..385` | all Building constructor effects replay only after complete generation | consume a complete ordered lifecycle journal preserving native generator phase order |
| `src/app/shell_random_map.rs:398` | generation entry remembers options/clears candidate only | reset native ID to 1,000,000 before the replacement cleanup decision |
| `src/app/shell_random_map.rs:178..217` | retention has presentation and storage owners only | keep presentation lifetime, but add persistent registered preview Buildings/Anims/latches/sound ownership |
| `src/map/construction_trace.rs:6..35` | flat Building-only events omit event word, ID, animation boundaries, and destructor/recreation events | replace/extend with a source-ordered lifecycle journal |
| `src/sim/anim_class.rs:543,550` | `choose_anim_rate` runs before `allocate_stable_id` | allocate/register native identity before optional RandomRate |
| `src/sim/anim_class.rs:554..558` | stable key rejects duplicates and is copied into `native_unique_id` | keep stable handles unique; assign native numeric ID independently and allow native-ID duplication |
| preview native-lifecycle model | no branch-specific Cell/type/House prefix or retained cross-class native IDs | exact-match emits no setup Assign; rebuild emits `R+P_preview+HB+K_preview`; preserve retained IDs independently of handles |
| preview native-lifecycle model | no generated growth/spread queue ownership | free spread then growth before reset/branch; rebuild growth then spread; preserve across Cancel/re-entry |

The lifecycle journal may be applied atomically when a worker result is collected,
provided it retains exact native event order and applies replacement retention and
terminal cleanup in that order. It must not remain a post-hoc list of surviving
Buildings because failed constructors and transient/recreated Anims own the
identity/RNG/sound effects.

### 8.3 Minimal implementation contract

1. Add a preview-native state owner that survives dialog Cancel and contains at
   least the native unique counter, collision-free handles and independent numeric
   IDs for every retained live class, terrain latches, sound-handle lifetime, and
   generated growth/spread queue ownership or equivalent deterministic contracts.
2. On every preview Generate, free spread then growth, set the native unique counter
   to `1,000,000`, and only then decide replacement cleanup. Do not rewind Scenario
   RNG.
3. If the storage snapshot is absent or the existing four-field key differs, full
   cleanup old preview objects/sounds, then consume real Cell/dummy, type,
   House/Super, and theater prefix events before generator construction. If it
   matches, delete the selective object families but emit no setup Assign event and
   retain old Types/Houses/Supers/Cells/dummy/Anims/latches/sounds through their
   native boundaries.
4. Emit/apply generated Building events where the native constructor occurs:
   Scenario raw word first, native ID second, placement outcome third. Preserve
   both costs for discarded constructed objects; emit nothing for CABHUT search
   failure before construction.
5. Emit/apply every actual tile-Anim construction, including intermediate and
   terminal recreation: native ID/register first, optional Scenario RandomRate
   second, active placement/latch, then conditional StartSound.
6. At terminal InitCellAttributes, destroy marked old/new terrain Anims in registry
   order, release their valid handles and latches, then recreate final Anims in the
   verified cell sweep order.
7. On Cancel/common teardown, destroy only presentation/snapshot owners. Preserve
   the native preview state, Scenario RNG advancement, native counter, and final
   growth/spread queues until a real cleanup/reset boundary.
8. Keep collision-free runtime handles separate from `native_unique_id`. Do not
   use early cleanup, ID skipping, or collision rejection to erase native temporary
   numeric reuse in any live class.
9. On `.SED` launch, discard the preview owner through the launch Full_Init cleanup
   and apply the launch-only unique-ID reservation in the separate launch
   transaction. Preserve the initial spread/growth free, Clear_Scene's second free,
   and Full_Init's growth/spread rebuild order.

### 8.4 Acceptance tests

1. **Preview branch test:** Generate uses Set_Defaults/manual setup and never
   records Full_Init, Clear_Scene, or `+10,000` effects.
2. **Building failure test:** one constructed-then-discarded neutral Building spends
   exactly one raw Scenario word followed by native ID `1,000,001` when it is the
   first generator constructor on the exact-match branch; no refund occurs.
3. **CABHUT pre-search failure test:** no construction event, word, or ID is spent.
4. **Stock tile-Anim test:** each constructed TUNTOP/waterfall Anim spends an ID but
   no RandomRate word; only the four `*01X` types attempt WaterfallLoop.
5. **Custom RandomRate order test:** native ID/register precedes exactly one Scenario
   inclusive draw; Rust stable handle remains independent.
6. **Terminal churn test:** transient tile Anims spend IDs, are destroyed, and final
   recreations spend new IDs in terminal sweep order.
7. **Same-key prefix/replacement test:** free spread/growth, reset to `1,000,000`,
   emit no Cell/type/House/theater Assign, retain old Types/Houses/Supers,
   real/dummy Cells and Anims, and give the first new object `1,000,001`; a retained
   first Cell with that number remains live. Old final Anims/latches/valid sounds
   survive to terminal cleanup, then are released and recreated.
8. **Changed-key prefix test:** with `W=2,H=3`, `P_preview=5`, one House and two
   Supers, and retail theater, consume `R=10`, `HB=3`, `K=0`; the cursor before the
   generator is `1,000,018` and its first object receives `1,000,019`.
9. **Changed-key cleanup test:** old preview Anims/sounds die before the first new
   constructor; counter was reset before destruction and prefix constructors follow.
10. **Queue lifetime test:** every Generate frees spread then growth before reset;
    completion rebuilds growth then spread; Cancel and no-Generate re-entry preserve
    the final queues unchanged.
11. **Cancel/re-entry test:** Cancel removes presentation/snapshot state but preserves
    native objects/sounds/counter/queues and Scenario cursor; reopen-and-close
    without Generate changes none of them.
12. **First Generate after re-entry test:** absent snapshot selects full cleanup;
    spread/growth free and reset occur first, branch prefix follows cleanup, and
    growth/spread rebuild after candidate generation.
13. **Accepted launch separation test:** acceptance alone does not clean preview
    state; later `.SED` launch does and applies the independent Full_Init/map-read
    prefix plus its queue free/rebuild boundaries.

## 9. Open questions log

| ID | Question | Resolution |
|---|---|---|
| OQ-01 | Does setup Generate pass preview or launch mode? | **RESOLVED:** literal `1`; `.SED` launch passes `0`. |
| OQ-02 | Can preview reach Full_Init through another synthetic branch? | **RESOLVED:** no; exact conditional reaches Set_Defaults/manual setup. |
| OQ-03 | Is preview object Unlimbo suppressed by `g_GameActive`? | **RESOLVED:** no; Main sets it to `1` before shell. |
| OQ-04 | Are later generator Recalc phases preview-gated? | **RESOLVED:** no; all listed common sites and terminal InitCellAttributes execute. |
| OQ-05 | Do stock tile Anims consume Scenario RandomRate? | **RESOLVED:** no; all 20 active retail sections have absent/zero RandomRate. |
| OQ-06 | Can stock preview terrain start sound? | **RESOLVED:** yes, conditionally; four waterfall heads declare WaterfallLoop and PlayAt has no preview gate. |
| OQ-07 | Is Building ID assigned before its Techno word? | **RESOLVED:** no; raw Scenario word is stored first, then AssignUniqueID. |
| OQ-08 | Does failed Building placement refund either effect? | **RESOLVED:** no. |
| OQ-09 | Does every candidate continue the prior candidate's numeric IDs? | **RESOLVED:** no; every Generate resets to 1,000,000. |
| OQ-10 | Does Set_Defaults also rewind the Scenario RNG effect stream? | **RESOLVED:** no; `+0x214` ID and `+0x218` RNG are distinct. |
| OQ-11 | Are old Anims deleted early on a matching storage key? | **RESOLVED:** no; selective cleanup omits them and does not clear their latches. |
| OQ-12 | Are old Anims deleted early on changed/missing storage? | **RESOLVED:** yes, by full cleanup. |
| OQ-13 | Does Anim destruction stop cleanup at registry removal? | **RESOLVED:** no; it also clears applicable latch and releases/detaches sound handles. |
| OQ-14 | Does Cancel clean final preview objects or reset ID? | **RESOLVED:** no. |
| OQ-15 | Does a hidden parent cleanup run after Cancel? | **RESOLVED:** no; non-OK returns immediately after the runner. |
| OQ-16 | Does re-entry alone create the snapshot or change lifetime? | **RESOLVED:** no. |
| OQ-17 | Can matching-key replacement reuse a live native numeric ID? | **RESOLVED:** yes; retained Types/Houses/Supers/Cells/dummy/Anims remain live and unchecked AssignUniqueID permits global duplication. After a prior rebuild, first-Cell/new-first-object `1,000,001` reuse is deterministic when a new constructor occurs. |
| OQ-18 | What exact count of live waterfall handles exists for every candidate? | **DEFERRED — INPUT/DEVICE SPECIFIC, NOT A BEHAVIOR GAP:** count is determined by generated eligible cells plus ordinary audio/volume admission; creation and cleanup rules are exact. |
| OQ-19 | Does changed/missing preview start generator objects at `1,000,001`? | **RESOLVED:** no; Cells/dummy, types, Houses/Supers, and theater K precede them. Active-retail K=0. |
| OQ-20 | Do final growth/spread queues survive Cancel/re-entry? | **RESOLVED:** yes; the next Generate frees spread/growth before reset and later rebuilds growth/spread. |

No material behavior question in the requested bounded lifecycle remains open.

## 10. Zero-add and adversarial pass

### Five adversarial questions

1. **Could `1` mean Full_Init rather than preview?** No. The value reaches the
   nonzero branch that directly calls Set_Defaults; the `0` branch contains the
   Full_Init call, and `.SED` launch supplies `0`.
2. **Could preview-created objects still be inert because the app has not entered a
   match?** No. The actual gate checked by Object Unlimbo is set before the shell,
   and the active constructor/Unlimbo paths are reached.
3. **Could old tile Anims be removed by the selective cleanup under an indirect
   family loop?** No. The explicit family loops omit Anim, the cell reset omits
   `+0x140`, and the old marked Anims are observed by the later terminal registry
   scan/destructor path.
4. **Could Cancel cleanup occur after `RandomMapSetupDialog__Run` returns?** No. The
   parent tests `result == 1` and immediately returns on Cancel's non-1 result.
5. **Could Rust safely use native ID as its map key because resets happen only after
   cleanup?** No. Preview calls Set_Defaults before replacement cleanup, and the
   matching-key branch retains Types, Houses/Supers, Cells/dummy, and old Anims.

### Cold spot checks

1. **Active gate challenge:** independent assembly context at `0x0052D9D7` confirmed
   the literal write of `1` to `g_GameActive` before shell entry, overturning the
   initial inert-preview hypothesis.
2. **Cancel/sound challenge:** fresh decompilation of
   `AnimClass::Destructor @ 0x00422900`,
   `RandomMapSetupDialog__Run @ 0x00595BC0`, and
   `ChooseMap__AcceptRandomMapSetup @ 0x005E8590` independently confirmed that Anim
   destruction releases/detaches sounds, while Cancel teardown never invokes that
   destructor and the parent has no later cleanup.
3. **Cell-prefix challenge:** `CellClass::Constructor @ 0x0047BBF0` calls
   AssignUniqueID; both the new/in-place Resize sites and the unconditional dummy
   site run on rebuild but are skipped on exact match. This overturned the prior
   unconditional preview-first-object `1,000,001` claim.
4. **Queue-lifetime challenge:** the unconditional free calls at
   `0x00599A13/0x00599A18`, terminal init calls at `0x0059939B/0x005993A0`, and
   absence of either free routine from common dialog teardown prove persistence and
   the next-Generate replacement boundary.

Zero-add result: no additional active writer, matching-branch ID constructor,
preview-only cleanup, stock RandomRate source, hidden queue free, or hidden Cancel
cleanup was found. The bounded inventory is stable.

## 11. Ghidra annotation candidates (not applied)

No Ghidra metadata was changed. Useful future candidates:

- name the forwarded parameter of `0x00598960`/`0x00599650` `bPreview` and document
  nonzero Set_Defaults versus zero Full_Init;
- label `ScenarioClass+0x214` as the native unique-ID counter, explicitly separate
  from RNG state at `+0x218`;
- label `MapSeedClass+0x178` / `DAT_00ABE150` as the dialog storage snapshot clone;
- comment the matching-key selective cleanup with “Anim intentionally retained”;
- document matching-key retained Cells/types/Houses/Supers and globally non-unique
  native IDs after the counter reset;
- document preview queue order: free spread/growth before reset; rebuild
  growth/spread after generation;
- comment `RandomMapSetupDialog__Run` common teardown with “presentation/snapshot
  only; world registries and Scenario counter persist.”

## Sources

### Active `gamemd.exe`

- dialog setup/generation: `0x00595BC0`, `0x0059664C`, `0x00596A49`, `0x00596A66`,
  `0x005E8590`
- RMG: `RandomMapGenerator__Generate @ 0x00598960`,
  `InitMapFromSyntheticINI @ 0x00599650`, Recalc sites `0x00598E48`, `0x00598FE7`,
  `0x00599153`, `0x0059937D`, terminal call `0x0059944C`
- scenario lifecycle: `Set_Defaults @ 0x00683610`, `Clear_Scene @ 0x006851F0`,
  `Full_Init @ 0x00686B20`, `NextUniqueID @ 0x0068BCB0`, map read `0x004ACE70`
- object identity: `AbstractClass::AssignUniqueID @ 0x00410230`,
  `TechnoClass::Constructor @ 0x006F2B90`,
  `BuildingClass::Constructor @ 0x0043B740`
- preview prefix: `CellClass::Constructor @ 0x0047BBF0`,
  `MapClass::Resize @ 0x00565C10`, rules reset/rebuild `0x006686C0`,
  `HouseClass::Constructor @ 0x004F54A0`, `SuperClass::Constructor @ 0x006CAF90`
- object activation: `Main__PrepareSession @ 0x0052D9A0`,
  `ObjectClass::Unlimbo @ 0x005F4EC0`, `TechnoClass::Unlimbo @ 0x006F6CA0`,
  `BuildingClass::Unlimbo @ 0x00440580`
- Anim/cell/audio: `CellClass::RecalcAttributes @ 0x0047D2B0`,
  `MapClass::InitCellAttributes @ 0x00568BB0`,
  `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::Middle @ 0x00424CE0`,
  `AnimClass::Destructor @ 0x00422900`, `VocClass::PlayAt @ 0x007509E0`
- cleanup: `FUN_00534450`, Anim loop near `0x005346C3`
- tiberium queues: FreeSpread all `0x00722390`, FreeGrowth all `0x00722E50`,
  InitGrowth all `0x00722D00`, InitSpread all `0x00722240`
- generated Building owners: `0x005904B0`, `0x005954A1`, `0x005A96F8`
- excluded temporary-ID owner: `FootClass::ClickedAction_Cell @ 0x004D7F35`

### Retail data

- `ini/desertmd.ini:1163..1340`
- `ini/snow.ini:1094..1175`
- `ini/art.ini:13742..14132`
- `ini/artmd.ini:18670..18881`
- `ini/artmd.ini:19005..19058`

### Rust inspected

- `src/app/frontend/skirmish_session.rs:86..92,515..524`
- `src/app/shell_random_map.rs:31..102,178..217,377..405`
- `src/map/construction_trace.rs:1..62`
- `src/sim/anim_class.rs:543..558`

### Cross-report reconciliation

- `docs/research/bridges/01-assets-map-load-overlay/FULL_INIT_AND_PREVIEW_NATIVE_ID_PREFIX_REINVESTIGATION_GHIDRA_REPORT.md`

### Navigation leads only

- `C:\Users\enok\Documents\OpenTS\code\mapgen.cpp`
- `C:\Users\enok\Documents\OpenTS\code\display.cpp`
- `C:\Users\enok\Documents\OpenTS\code\scenario.cpp`
- `C:\Users\enok\Documents\OpenTS\code\abstract.cpp`
- `C:\Users\enok\Documents\OpenTS\code\foot.cpp`
- `C:\Users\enok\Documents\OpenTS\code\anim.cpp`

### Prior reports used only to locate claims requiring cold checks

- `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/bridges/01-assets-map-load-overlay/TERRAIN_ATTACHED_ANIM_LOAD_LIFECYCLE_SIDE_EFFECTS_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/MAPGEN_SAME_PROCESS_LIFECYCLE_BRIDGE_CALLER_RECONCILIATION_GHIDRA_REPORT.md`
