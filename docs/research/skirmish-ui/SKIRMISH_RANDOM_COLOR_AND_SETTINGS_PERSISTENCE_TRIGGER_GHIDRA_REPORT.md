# Skirmish Random Colour and Settings Persistence Trigger - Ghidra Research Report

**Address(es):** `0x0049B610`, `0x0049BC80`, `0x0049BCC0`, `0x0049CAF0`, `0x0049D070`, `0x0049D9F0`, `0x0049DB00..0x0049E3A9`, `0x0049E3B0`, `0x0052BA60`, `0x0052D9A0`, `0x0052FC20`, `0x005C1470`, `0x005C1980`, `0x005C1A10..0x005C1E8A`, `0x005C34F0`, `0x005C35F0`, `0x005D6430`, `0x005D6440`, `0x005D7CE0`, `0x005D82B0`, `0x00697F10`, `0x006980C0`, `0x00698F90`, `0x006990A0`, `0x0069B600`, `0x0069B670`, `0x0069B760`, `0x0069B7E0`, `0x0069B8C0`, `0x006ACEE0`, `0x006AE2C0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline Skirmish Random Colour resolution, the RNG object/order/collision mechanism that affects it, Cooperative progress-map initialization and active-record binding, and the load/update/write trigger for the persisted `[Skirmish]` setup snapshot.  
**Non-Scope:** online lobby packet authority; Random Map generation; shell paint geometry; map-row visibility; AI gameplay after house creation.  
**Confidence:** High for the ordinary offline call sequence, Random Colour source/order/collision behavior, normal-process Scenario RNG lifetime across startup mode construction, Back/re-enter, and match-return paths, Cooperative progress initialization/selection plus eligibility parser/callback mechanism, snapshot layout, key order/format, and Start/Back write trigger.  
**Active in YR:** Yes for standard offline Skirmish. Cooperative callback details are conditional on selecting the stock Cooperative mode.

## 0. Working Notes and Stop Conditions

- Target question A: Which exact native state is consumed when a local or AI row is left on Random Colour, in which order, and which selections block a candidate?
- Target question B: When does active YR load, update, and write `[Skirmish]`, and does Start differ from Back?
- Evidence needed for completion: live decompile plus assembly for the offline command owner, random helpers, collision helper, RNG primitive/source, outer shell owner, settings reader/writer, and the broader RA2MD.INI writer; current Rust ownership scan; one retail RA2MD.INI fixture.
- Stop condition: every branch that changes Random Colour draw count or `[Skirmish]` bytes is verified or explicitly deferred with a named next investigation.
- No Rust source was changed during this investigation.

## 1. Executive Finding

Active YR has two separate but coordinated representations of a Random Colour selection:

1. The shell/persistence representation keeps the raw `-2` sentinel.
2. A launch representation is resolved to a concrete colour before the dialog returns.

The ordinary offline command order is not simply "run `ProcessRandomAssignments` over a finished session":

```text
Start or Back command
  -> resolve local Random Country immediately
  -> pack all eight AI country/colour/start/team arrays
  -> pack the seven persisted AI Slot triples, preserving -2 sentinels
  -> resolve local Random Colour immediately
  -> allocate/add the local player node
  -> ProcessRandomAssignments (human nodes, then all eight AI slots)
  -> mirror sliders/checkboxes into the persisted snapshot
  -> close dialog
  -> write RA2MD.INI, including [Skirmish]
```

Both **Start (`0x617`) and Back (`0x5C0`)** take that common pack/randomize path when their notification code is zero. Both then reach the unconditional settings writer after the dialog is destroyed. Evidence: live `disassemble_function 0x006ACEE0`, especially `0x006ACF60..0x006ACFA4`, `0x006AD34B..0x006AD8E4`; live `disassemble_function 0x006AE2C0`, especially `0x006AE37B..0x006AE3DF`.

Random Colour uses `Random__RandomRanged(0,7)` on `ScenarioClass+0x218`. These are **pre-game shell draws**. `Main_Game` calls the offline shell at `0x0052E168`, and only later calls `Init_Random_Number_System @ 0x0052FC20` at `0x0052E619`. That initializer overwrites both `ScenarioClass+0x218` and `g_MainRng` from the newly chosen game seed. Consequently, shell Random Colour draws affect launch assignments but do **not** advance the subsequently seeded gameplay Scenario RNG. Evidence: live `disassemble_function 0x0052D9A0`; live `decompile_function 0x0052FC20`.

The first front-end cursor is already advanced before the first Skirmish dialog. Process initialization seeds the Scenario RNG at `0x0052CAD2`, then calls the MPModes loader at `0x0052CAF6`. Its stock Cooperative factory constructs one progress record per Cooperative campaign and calls `RandomRanged(0,2)` once for each campaign-map stage to choose one of three `MapN` variants. Stock `CoopCampMD.ini` has `3 + 3 + 3 + 1 = 10` stages, so fresh stock startup makes ten logical ranged calls after the seed even when Battle, not Cooperative, is later selected. Because span three uses mask-and-reject sampling, a logical `(0,2)` call can reject raw value `3` and consume more than one raw word. Evidence: live assembly `0x0052CAD2..0x0052CAF6`, `0x005D7E1C..0x005D7E36`, factory slot `0x005D82B0..0x005D82E6`, constructor `0x005C1470`, and progress initializer `0x0049CAF0`.

## 2. Relevant Layout and Sentinels

### 2.1 Session and snapshot addresses

`SessionClass` is the static object at `0x00A8B238` in this slice. The selected MPModes pointer is `Session+0x04` (`0x00A8B23C`). The persisted offline snapshot is `Session+0x18C` (`0x00A8B3C4`). Evidence: the offline shell passes `ECX=0x00A8B238` to `0x006990A0` at `0x006AE3B5`; the broader writer passes `LEA ECX,[ESI+0x18C]` to `0x00698F90` at `0x006991C5..0x006991D5`.

| Location | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `Session+0x04` | selected MPModes object pointer | `0x006AD2BA`, `0x0069B90A`, `0x0069B9EA` | Yes |
| `Session+0x15C/+0x160` | local colour value / random marker mirrors | `0x0069B890..0x0069B8A2` | Yes |
| `Session+0x174/+0x178` | local country value / random marker mirrors | `0x0069B7C0..0x0069B7D2` | Yes |
| `Session+0x17C/+0x180` | local persisted/profile colour value / marker | `0x0069B85E..0x0069B890` | Yes |
| `Session+0x184/+0x188` | local persisted/profile country value / marker | `0x0069B788..0x0069B7C0` | Yes |
| `Session+0x18C` | `[Skirmish]` snapshot base | `0x0069844B`, `0x006991CF` | Yes |
| `Session+0x84..+0xA0` | eight AI colour dwords | `0x0069B600`, `0x0069B8C0` | Yes |
| `Session+0x2840/+0x284C` | human node pointer vector / count | `0x0069B600`, `0x0069B8C0` | Yes |

### 2.2 Random/observer values

| Value | Meaning in this slice |
|---:|---|
| `-3` | observer sentinel for human node country; canonicalized to country `-3`, colour `8` |
| `-2` | unresolved Random sentinel |
| `-1` | concrete/no-random marker or inactive AI value, depending on field |
| `0..7` | playable colour indices |
| `8` | observer colour |
| `0..9` | ordinary country-index draw range (ten inclusive values) |

The range `0..9` contains ten values. Older prose that calls it a nine-country range is arithmetically stale.

## 3. Exact Offline Random Assignment Flow

### 3.1 Start and Back share the same command body

At `0x006ACF60`, the dispatcher recognizes Back by subtracting `0x5AA` then `0x16` (original id `0x5C0`) and Start by the following `0x57` subtraction (original id `0x617`). Both reach `0x006ACF7B` when the notification code is zero. Only Start disables the Start control and runs the dialog validation block. Back bypasses those validations at `0x006AD05F` but rejoins the common packing path at `0x006AD2BA`. Evidence: live `disassemble_function 0x006ACEE0`.

This means Back is not a no-op cancel for session state. It packs the current controls, consumes any required pre-game random draws, mirrors option state, stores modal result `0x5C0`, and later writes the settings file.

### 3.2 Local Random Country happens first

The handler reads local country control `0x6A1` and calls `FUN_0069B760` with its second argument true:

- `0x006AD3A4..0x006AD3BA`: read combo item data, `PUSH 1`, call `0x0069B760`.
- If item data is `-2`, `0x0069B760` calls `Random__RandomRanged(0,9)` using `ECX=(*0x00A8B230)+0x218`, stores the concrete value at `Session+0x184`, and retains marker `-2` at `Session+0x188`.
- It mirrors those two fields to `Session+0x174/+0x178`.

Evidence: live `decompile_function 0x0069B760`; live `disassemble_function 0x0069B760`, especially `0x0069B774..0x0069B7D2`.

Therefore a local Random Country selection consumes one Scenario draw **before** any local Random Colour draw.

### 3.3 AI arrays and persisted Slot triples are packed before local colour

The first seven-row loop at `0x006AD3C1..0x006AD4F2` reads AI row kind, country, colour, start, and team into parallel session arrays. The second loop at `0x006AD4F8..0x006AD5F8` writes the compact persisted triples at `0x00A8B3F0` onward. These triples retain the raw country and colour combo item data, including `-2`.

The persisted row-type mapping is:

| AI row combo item data | Persisted type code |
|---:|---:|
| `-1` (None) | `1` |
| `0` (Hard) | `4` |
| `1` (Normal) | `5` |
| `2` (Easy) | `6` |

For every `SlotNN`, the triple is `(type_code, country, colour)`. It is not `(country, colour, team)` and not `(side, colour, start)`. Evidence: `0x006AD5B0..0x006AD5E8`; later writer reads the same three dwords at snapshot offsets `+0x28/+0x2C/+0x30` for Slot01.

### 3.4 Local Random Colour is resolved before the local node exists

The handler reads local colour control `0x6A2` and calls `FUN_0069B7E0` with its second argument true at `0x006AD5FE..0x006AD63B`.

For raw colour `-2`, that helper:

1. Draws `Random__RandomRanged(0,7)` from `ScenarioClass+0x218`.
2. Scans every existing human node colour.
3. Scans all eight AI colour dwords at `Session+0x84..+0xA0`.
4. Retries on any collision.
5. Stores the concrete colour at `Session+0x17C` while preserving marker `-2` at `Session+0x180`.
6. Mirrors value/marker to `Session+0x15C/+0x160`.

Evidence: live `decompile_function 0x0069B7E0`; live `disassemble_function 0x0069B7E0`, especially `0x0069B7FF..0x0069B8A2`.

The local player node is allocated only afterward at `0x006AD647`, receives the already concrete country/colour at `+0x4B/+0x53`, and is appended to the human vector at `0x006AD6E1..0x006AD6F6`. Thus the ordinary offline local row does not wait for `ProcessRandomAssignments` to choose its colour.

### 3.5 `ProcessRandomAssignments`: human nodes, then eight AI slots

`SessionClass__ProcessRandomAssignments @ 0x0069B8C0` is called with `ECX=0x00A8B238` at `0x006AD6F9`. Its direct callers are the offline command handler and a network/lobby path at `0x005DC350`. Evidence: live `get_function_callers 0x0069B8C0`; live decompile/disassembly of `0x0069B8C0`.

The function captures the human count once from `0x00A8DA84`, then walks `0x00A8DA78[0..count)` in pointer order. Within each human node:

1. Observer canonicalization runs first and consumes no RNG.
2. Random country marker `-2`, if still present, is cleared to `-1` and resolved.
3. Random colour marker `-2`, if still present, is cleared to `-1`, the concrete colour is temporarily set to `-1`, and `(0,7)` is retried until the collision helper returns false.
4. Human node zero's concrete colour is unconditionally mirrored to `0x00A8B394`.

After all humans, the AI loop visits exactly eight country entries at `0x00A8B29C..0x00A8B2B8`, with the corresponding colours at `+0x20` (`0x00A8B2BC..0x00A8B2D8`). It increments the country pointer by one dword and stops when the next pointer reaches `0x00A8B2BC`. It does not use the active-AI count as the loop bound. Inactive `-1` fields simply consume no draw.

For each AI slot, country resolution precedes colour resolution. A random colour draw is checked against all current human colours and all eight AI colour entries, so concrete colours in later AI rows already block earlier random rows. Previously resolved AI colours also block later rows. Evidence: `0x0069B9D6..0x0069BAA0`.

### 3.6 Collision helper contract

`FUN_0069B600` receives `ECX=SessionClass` and one candidate colour:

- Candidate `-2` immediately reports no collision.
- Human nodes with marker `-2` and concrete field `-1` are treated as colour `-2`, not `-1`.
- Every other human node contributes its concrete `+0x53` colour.
- All eight AI colour dwords are compared without an active-row filter.
- Return low byte is `1` on a match, `0` otherwise.

Evidence: live `decompile_function 0x0069B600`; live assembly around `0x0069B600..0x0069B664`.

There is no exhaustion guard. A malformed state with all eight colours occupied plus another random-colour slot retries forever. The valid eight-player shell avoids that state because the random slot itself leaves at most seven other distinct playable colours occupied.

### 3.7 Selected-mode country callbacks can shift later colour draws

When `Session+0x04` is non-null, human-node country resolution calls selected-mode vtable `+0x6C`; AI country resolution calls vtable `+0x70`. The ordinary offline selected mode pointer is non-null.

For Battle, ManBattle, FreeForAll, Unholy, and the dormant stock-Siege category, vtable `+0x6C` is `0x005D6430` and `+0x70` is `0x005D6440`:

- `0x005D6430` sets `ECX=0x00A8B238` and jumps to `0x0069B670`.
- `0x0069B670` performs one `ScenarioClass+0x218` `Random__RandomRanged(0,9)` draw.
- `0x005D6440` dispatches the same object's `+0x6C`, so the ordinary AI path uses the same draw.

Evidence: live vtable memory at `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, and `0x007EE424`; live `get_assembly_context` for `0x005D6430`, `0x005D6440`, and `0x0069B670`.

Cooperative overrides both callbacks: human `0x005C34F0`, AI `0x005C35F0`. Both consume the current Cooperative progress record at object `+0x40`:

1. `0x0049BC80` reads `CampaignType` from progress-record `+0x44`, gated by valid byte `+0x6C`.
2. `0x0049E3B0` looks up that campaign in the lazy global Cooperative registry.
3. Missing campaign data returns country index `0` and consumes no RNG.
4. `0x0049BCC0` reads `CurrentMap` from progress-record `+0x38`; getter failure leaves the callback's local default at map `0`.
5. The human callback selects the per-map family at campaign-record `+0x10` (array `+0x14`, family count `+0x20`). The AI callback selects `+0x28` (array `+0x2C`, family count `+0x38`). An out-of-range map also returns `0` without a draw.
6. Otherwise it repeatedly calls `RandomRanged(0, global_country_count - 1)` on `ScenarioClass+0x218` until the candidate equals one token resolved from the selected list.

Each selected per-map list stores its token pointer at `+0x04` and count at `+0x10`. `0x0049D9F0` resolves `<random>` to `-2`, a valid HouseType ID/name to its current global country index, and null/unmatched text to `-1`. Duplicates are preserved but do not change membership. There is no attempt cap or empty/all-invalid fallback, so malformed empty eligibility can loop forever. The draw bound is data-driven; stock has ten registered countries, but the mechanism is not intrinsically hardcoded to `0..9`. Evidence: Cooperative vtable `0x007EE27C`; live assembly contexts for `0x005C34F0` and `0x005C35F0`; live decompile/assembly for `0x0049BC80`, `0x0049BCC0`, `0x0049D9F0`, and `0x0049E3B0`.

The registry parser is `0x0049DB00..0x0049E3A9`, and its authority file is retail **`CoopCampMD.ini`**, not `MPCoopMD.ini`. It enumerates `[Campaigns]` entries in source order; each value names one campaign section. The section keys consumed here are `NumberOfCampaignMaps`, `CampaignName`, `CampaignLoadScreen`, native-spelled `CampaignLoadScreenPallet`, `CampaignAI`, `Map%d`, `CampaignPlayer%d`, and `CampaignEnemy%d`. Player/enemy values are comma-tokenized in source order; no explicit whitespace trim is visible. `NumberOfCampaignMaps` defaults to zero and `CampaignAI` to `Easy`; stock explicitly uses `Normal`. Open/load failure leaves the registry invalid for retry, whereas a successful empty parse marks it valid with zero campaigns. Evidence: live decompile/assembly `0x0049DB00..0x0049E3A9`; retail plaintext payload embedded in `ra2md.mix`.

Stock country eligibility, in global rules order, is:

| Campaign / map | Human `CampaignPlayerN` indices | AI `CampaignEnemyN` indices |
|---|---|---|
| Allied 1 | `{0}` | `{8,7}` |
| Allied 2 | `{0,4,2}` | `{9}` |
| Allied 3 | `{0,1,3,2,4}` | `{8,9}` |
| Soviet 1 | `{8,6,7}` | `{0}` |
| Soviet 2 | `{6,5}` | `{9}` |
| Soviet 3 | `{8}` | `{3,9}` |
| Yuri 1 | `{9}` | `{0}` |
| Yuri 2 | `{9}` | `{7,8}` |
| Yuri 3 | `{9}` | `{8,4}` |
| World 1 | `{0,1,2,3,4,5,6,7,8}` | `{9}` |

Indices are `0 Americans, 1 Alliance, 2 French, 3 Germans, 4 British, 5 Africans, 6 Arabs, 7 Confederation, 8 Russians, 9 YuriCountry`. Consequently, exact colour outputs are coupled to preceding random-country choices. For ordinary modes each random country makes one ranged call before that slot's colour retries. Cooperative may make several logical ranged calls, and each stock `(0,9)` ranged call may itself reject raw values `10..15`, before the following Random Colour draw.

### 3.8 Cooperative progress-map construction and active binding also consume Scenario RNG

`FUN_0049B610` initializes a new `0x70`-byte Cooperative progress record with `CurrentMap = -1` at `+0x38`, no chosen-map vector at `+0x3C/+0x40`, `CampaignType = -1` at `+0x44`, and valid byte zero at `+0x6C`.

`FUN_0049CAF0(record, campaign_index)` then performs the following verified transaction when the campaign index is valid:

1. It marks the record valid, resets `CurrentMap` and the progress counters, and finds the source-order campaign record.
2. If `record+0x44` differs from the requested campaign, it frees the old chosen-map vector and allocates one pointer per campaign stage.
3. For each stage, it calls `RandomRanged(0,2)` on `ScenarioClass+0x218`, selects that stage's corresponding one of three `MapN` filename variants, and stores a private `0x104`-byte filename copy through the vector at `+0x3C`.
4. It writes the stage count at `+0x40`, resets `CurrentMap` to `0`, and finally writes the requested `CampaignType` at `+0x44`.

If the record already has that same `CampaignType`, the function resets the other progress fields but does not rebuild the filename vector and consumes no map-variant draw. A fresh record starts at type `-1`, so its first successful initialization always draws once per stage. Evidence: live decompile plus assembly `0x0049B610`, `0x0049CAF0`, especially the `Scenario+0x218` call sequence at `0x0049CBC5..0x0049CBD4`.

The stock Cooperative object constructor `0x005C1470` enumerates the campaign registry, allocates one fresh progress record per campaign, and initializes each through `0x0049CAF0`. The MPModes loader reaches this constructor through its sixth factory: `0x005D7E1C` installs vtable `0x007EEE80`, `0x005D7E36` runs the parser/factory, and vtable slot `+0x04` at `0x005D82B0` allocates the `0x344`-byte Cooperative object and calls `0x005C1470` at `0x005D82E1`. Process initialization calls this loader at `0x0052CAF6`, after Scenario seeding at `0x0052CAD2`; `Main_Game` then enters the front-end loop directly at `0x0048CDD3`. Thus stock fresh startup's ten logical `(0,2)` calls are part of the first shell transcript.

The selected-map binding is also verified:

- `0x005C1980` compares the shell's currently selected scenario filename against all three `MapN` variants for each source-order Cooperative campaign and returns its campaign index or `-1`.
- The Cooperative open/selection path `0x005C1A10..0x005C1C7B` creates and initializes active progress pointer `object+0x40` when absent, stores the matched/default campaign at `object+0x33C`, and obtains the chosen filename through `0x0049D070` from the active progress record's `+0x3C` vector.
- On accepted Choose Map campaign switching, `0x005C1DC0..0x005C1E8A` destroys the old active record, moves the selected campaign's preconstructed record from the vector at `object+0x328` into active pointer `object+0x40`, allocates a replacement record into that vector slot, initializes the replacement for the selected campaign through `0x0049CAF0`, and stores the selected campaign at `object+0x4C`. Creating that replacement consumes one `(0,2)` logical call per stage because the replacement starts with `CampaignType = -1`.

The later human/AI country callbacks read `CampaignType` and `CurrentMap` from this active `object+0x40` record. The progress-map selection draws therefore precede and can shift the Cooperative random-country rejection transcript and every following Random Colour result; they cannot be modeled as visual-only map choice state.

## 4. RNG Primitive, Authority, and Seed Boundary

`Random__RandomRanged @ 0x0065C7E0` is inclusive on both ends and uses the 250-word XOR-lag state with mask-and-reject sampling. Equal bounds return without a draw. Every ordinary `(0,7)` call uses one raw draw because the span is exactly eight values; collision retries cause additional ranged/raw draws. `(0,9)` can reject masked values `10..15`, so one logical country result may consume multiple raw draws. Evidence: live `decompile_function 0x0065C7E0`; the canonical helper report `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`.

Every offline colour, ordinary-country, Cooperative eligibility, and Cooperative progress-map call identified above loads `*0x00A8B230` and passes `Scenario+0x218` as `ECX`. No colour or Cooperative progress call in this slice uses `g_MainRng`.

The timing boundary is load-bearing. `ScenarioClass__Constructor @ 0x006832C0`
does call `Random__Seed(0)`, but that seed-zero state is only a transient
construction default. The process-level initializer `0x0052BA60` later calls
`Init_Random_Number_System @ 0x0052CAD2` before the front-end loop. That call
establishes matching boot-time states in the Scenario and main RNG objects; the
exact boot entropy-byte contract is outside this report. The same initializer
then calls the MPModes loader at `0x0052CAF6`, and stock Cooperative progress
construction advances the Scenario copy through ten logical `(0,2)` calls.
Therefore the first Skirmish shell normally begins from neither seed zero nor the
untouched post-seed cursor.

The normal cursor lifetime is:

| Stage | Native action | RNG consequence |
|---|---|---|
| Scenario object construction | transiently initializes `Scenario+0x218` with seed `0` | overwritten before the ordinary front-end loop |
| process initialization | `Init_Random_Number_System @ 0x0052CAD2` establishes the boot-time Scenario/main states | establishes the initial post-seed cursor |
| startup MPModes construction | loader `0x005D7CE0` at `0x0052CAF6` constructs stock Cooperative progress records | advances Scenario by ten logical `(0,2)` calls, including any rejection draws, before the first shell |
| Start or Back command | local country/colour plus AI random assignment draws | advances pre-game Scenario cursor |
| Back -> main menu -> re-enter | no Scenario reconstruction/reset occurs | the advanced cursor survives and the next shell continues it |
| successful shell return | `[Skirmish]` and other preferences are written | does not reseed yet |
| later `Main_Game` startup | `Init_Random_Number_System @ 0x0052E619` | overwrites Scenario and main RNG states from the fresh game seed |
| map/scenario initialization | gameplay consumes newly seeded streams | shell draws do not shift this cursor |
| normal return from gameplay to the same front-end loop | the process-lifetime Scenario object is retained; no constructor or `0x00683560` reset is called | the next shell sees the final gameplay Scenario cursor until another successful Start reseeds it |

Evidence: live `get_function_callers(0x0065C6D0)` enumerates the seed helper owners as random-map generation, `0x00683560`, `Init_Random_Number_System`, and `ScenarioClass__Constructor`; the Scenario constructor has one direct process-start call at `0x0052BA8D`; save/load reconstruction helper `0x00683560` has one direct call at `0x006894C5`; and the direct `Init_Random_Number_System` call census is startup `0x0052CAD2`, successful match start `0x0052E619`, and network seed application `0x005C489C`. Live process-initializer assembly puts `0x005D7CE0` at `0x0052CAF6`, after the seed call, and Cooperative factory slot `0x005D82B0` reaches constructor `0x005C1470`, whose fresh progress records reach `0x0049CAF0`. The process initializer returns directly to `Main_Game`, which enters the front-end loop at `0x0048CDD3`. Live `disassemble_function 0x0052D9A0` shows Back returning to that internal front-end loop without a reset while successful Start reaches `0x0052E619`. `Init_Random_Number_System` copies `0xFD` dwords into both `Scenario+0x218` and `g_MainRng`.

## 5. `[Skirmish]` Read Contract

The broader RA2MD.INI loader `0x006980C0` constructs/opens `RA2MD.INI` and calls `SessionClass__ReadSkirmishSettings @ 0x00697F10` with:

- `ECX = Session+0x18C`
- INI object `0x008870C0`
- section string `Skirmish`
- ordinary AI row-type default `1`
- Slot01 row-type default `6`

Evidence: live `disassemble_function 0x006980C0`, `0x0069843D..0x00698451`; live `decompile_function 0x00697F10`.

The reader loads, in order:

1. `GameMode`
2. `ScenIndex`
3. `GameSpeed`
4. `Credits`
5. `UnitCount`
6. `ShortGame`
7. `SuperWeaponsAllowed`
8. `BuildOffAlly`
9. `MCVRepacks`
10. `CratesAppear`
11. `Slot01` through `Slot07`

Missing global keys use current Rules/session defaults. Missing Slot01 defaults to type `6` (Easy); Slot02..Slot07 default to type `1` (None). Every missing slot country and colour defaults to `-2` (Random). The loop stores Slot01 at snapshot offsets `+0x28/+0x2C/+0x30` and Slot07 at `+0x70/+0x74/+0x78`.

The snapshot is loaded as part of the session/RA2MD.INI initialization path, not each time the `0x102` dialog repaints.

## 6. Snapshot Update and Write Trigger

### 6.1 What the command updates

The common Start/Back handler updates the offline snapshot in phases:

- `0x006AD34B..0x006AD36B`: selected mode id and selected scenario index; out-of-range scenario index is clamped to `0`.
- `0x006AD4F8..0x006AD5E8`: seven raw AI Slot triples, preserving Random `-2`.
- `0x006AD703..0x006AD7A4`: game speed, credits, and unit count mirrors.
- `0x006AD7A4..0x006AD889`: checkbox booleans and their snapshot mirrors.

`ProcessRandomAssignments` is called between the Slot packing and the global option mirrors. It mutates the launch node/AI arrays, not the already packed Slot triples. This is why Random stays persisted as Random while the current launch receives concrete values.

### 6.2 The outer shell writes after teardown on every exit path

`FUN_006AE2C0` pumps dialog `0x102` until modal result Start, Back, or the broader pump/quit condition. After destroying the dialog and cleaning preview state, it executes:

```text
0x006AE3B0  call cleanup
0x006AE3B5  ECX = 0x00A8B238
0x006AE3BA  call 0x006990A0
0x006AE3BF  inspect modal result
```

There is no branch around the writer. It runs after Start, after Back, after a dialog-creation failure, and after a pump/quit exit. The latter two paths write the current snapshot without a fresh Start/Back command pack. Evidence: live `disassemble_function 0x006AE2C0`.

### 6.3 Broader writer order

`FUN_006990A0` builds a file object for `RA2MD.INI`, updates `[MultiPlayer]` preferences first, then calls `0x00698F90` three times:

1. `Session+0x18C`, section `Skirmish`
2. `Session+0x208`, section `LAN`
3. `Session+0x284`, section `WonlinePref`

It then updates serial/phone sections and performs one final INI save at `0x00699424..0x00699430`. Evidence: live `disassemble_function 0x006990A0`; memory strings at `0x0083F0F4..0x0083F104` and `0x00826444`.

This corrects an older label error: `0x008870C0` is the INI object passed on the stack to `0x00698F90`, not the offline Skirmish snapshot. The snapshot is `Session+0x18C`.

## 7. Exact `[Skirmish]` Write Bytes

`SessionClass__WriteSkirmishSettings @ 0x00698F90` writes the same ten globals and then formats `Slot%02d` for `1..7`. Its field order exactly matches the reader's order. Evidence: live `decompile_function 0x00698F90`; live `disassemble_function 0x00698F90`, `0x00698F98..0x00699097`.

Formatting is:

- integers: decimal `%d`
- booleans: lowercase `yes` or `no`
- slots: `%d,%d,%d` with no spaces
- keys: `Slot01` through `Slot07`

Evidence: live decompile of `0x005275C0`, `0x00529560`, `0x00477510`; memory at `0x00817F6C` (`%d`), `0x00825BF4/0x00825BF8` (`no`/`yes`), `0x008189B0` (`%d,%d,%d`), `0x0083EF9C` (`Slot%02d`).

The writer does not null-check its INI/section arguments and ignores all per-key setter results. `FUN_00528660` can return `0` for invalid strings/allocation failure, but those results are discarded. The broader writer also ignores the final save return. No player-facing error is generated in this path. Evidence: live `decompile_function 0x00528660`; live disassembly of `0x00698F90` and `0x006990A0`.

The installed retail fixture currently contains:

```ini
[Skirmish]
GameMode=1
ScenIndex=162
GameSpeed=3
Credits=10000
UnitCount=10
ShortGame=yes
SuperWeaponsAllowed=yes
BuildOffAlly=yes
MCVRepacks=yes
CratesAppear=yes
Slot01=6,8,1
Slot02=1,-2,-2
...
Slot07=1,-2,-2
```

Evidence: `<ra2-install>/RA2MD.INI:36`.

## 8. Adversarial and Edge-Case Ledger

| Case | Verified native result | Evidence |
|---|---|---|
| local Random Country + Random Colour | country draw occurs first; colour draw/retries follow | `0x006AD3A4..0x006AD3BA`, `0x006AD5FE..0x006AD63B` |
| local Random Colour + concrete later AI colour | later AI concrete colour already blocks the local candidate | `0x0069B7E0` scans all eight AI colour fields |
| local Random Colour + AI Random Colour | local resolves first; raw AI `-2` does not collide; AI later avoids local concrete colour | `0x0069B7E0`, `0x0069B8C0` |
| Back with random fields | consumes the same randomization path, persists raw Slot `-2`, returns false from shell | `0x006ACF60..0x006AD8E4`, `0x006AE3B0..0x006AE3DF` |
| no human nodes | human phase is skipped; all eight AI entries are still visited | `0x0069B8DE..0x0069B9D6` |
| human observer | country `-3`, marker `-1`, colour `8`, marker `-1`; no country/colour draw | `0x0069B8F5..0x0069B987` |
| inactive AI | loop visits it, but `-1` country/colour consume no draw | `0x0069B9DB..0x0069BA79` |
| all colours exhausted in malformed >8 state | unbounded retry | no exit other than unused candidate in `0x0069B7E0`/`0x0069B8C0` |
| dialog creation fails | writer still runs; no fresh control pack | `0x006AE32F` zero branch joins `0x006AE38C..0x006AE3BA` |
| quit/pump break without Start/Back | writer still runs with last snapshot | `0x006AE360..0x006AE3BA` |
| fresh stock process startup | after boot seeding, Cooperative construction makes ten logical `(0,2)` progress-map calls; raw `3` is rejected | `0x0052CAD2..0x0052CAF6`, `0x005D82B0`, `0x005C1470`, `0x0049CAF0` |
| Cooperative progress init with unchanged CampaignType | resets progress fields but retains chosen filename vector and consumes no map-variant draw | `0x0049CAF0` guards rebuild on `record+0x44 != campaign` |
| accepted Cooperative switch to a different campaign | active record is swapped from the campaign vector; a fresh replacement is initialized and consumes one `(0,2)` call per stage | `0x005C1DC0..0x005C1E8A` |
| Cooperative campaign missing or map index out of range | callback returns country index `0` and consumes no RNG | `0x005C34F0`, `0x005C35F0`, `0x0049BC80`, `0x0049BCC0`, `0x0049E3B0` |
| Cooperative valid nonempty eligibility list | retry global-country candidates until a listed index is accepted; every rejection shifts the following colour draw | `0x005C34F0`, `0x005C35F0`, `0x0049D9F0` |
| Cooperative empty/all-invalid eligibility list | unbounded retry | callbacks have no exit other than an accepted resolved index |

## 9. Visual/UI Composition Ledger

This investigation does not cover a paint/composition function, so no pixel-parity claim is made and no retail bitmap is consumed by the mechanisms above.

One visible shell-state consequence is in scope: the collapsed Random Colour face must remain the Random sentinel/label after selection. Native resolves a separate launch copy and retains the `-2` marker for profile/snapshot state. Replacing the visible sentinel with a cached concrete swatch conflates those two representations. Asset names, palette rows, swatch pixels, combo geometry, focus painting, and animation timing remain governed by the existing colour-combo visual reports.

| Visual facet | Status in this report | Authority |
|---|---|---|
| collapsed Random Colour label/sentinel | behavior dependency verified | raw `-2` retained while launch value resolves |
| swatch art/palette | not rechecked | `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md` |
| combo geometry/owner draw | non-scope | existing shell visual corpus |
| assets | not applicable to RNG/persistence slice | none loaded here |

## 10. Current Rust Disparities

### 10.1 Random Colour marker is dropped at launch conversion

`src/ui/skirmish_shell/state/launch.rs` currently writes `color_random: false` for both the local slot and every active AI slot even though shell state already distinguishes concrete ownership with `player_color_claimed` / `opponent.color_claimed`. Therefore the visible Random choice cannot reach the resolver.

### 10.2 Resolver mechanism is close locally but runs at the wrong lifecycle boundary

`src/skirmish_launch.rs::resolve_unverified_legacy_random_assignments` already models ordinary-mode human-before-AI and country-before-colour order, uses inclusive `(0,9)` / `(0,7)`, seeds collision checking with concrete colours, and retries. Those details align with the ordinary non-Cooperative portion of the native mechanism.

However, `src/app_skirmish.rs::apply_unverified_legacy_skirmish_launch_session` invokes it only after a `Simulation` has been created and map/entity loading has progressed, using `sim.unverified_legacy_random_assignment_rng()`. Native performs shell draws before the successful-start `Init_Random_Number_System` call and then overwrites the Scenario RNG before gameplay. Rust can therefore both choose different colours and incorrectly shift every downstream gameplay Scenario-RNG result. A parity owner must also preserve cursor continuity across Back/re-enter and copy the final gameplay Scenario cursor back to the app/front-end owner on normal return, because native uses one process-lifetime Scenario object for both phases.

The current resolver also treats every AI random country as one direct `(0,9)` draw. That matches the base selected-mode callback's logical range but not Cooperative's data-driven global-country bound and `CoopCampMD.ini` eligibility retries. `src/skirmish_modes.rs` loads `MPCoopMD.ini` only as the selected mode's common dialog override; it has no Cooperative campaign/list or progress-record model. Rust consequently omits both the ten stock post-seed startup map-variant calls and the replacement-record draws caused by accepted Cooperative campaign switching.

### 10.3 `[Skirmish]` load/write is absent

`AppState` builds `SkirmishShellState` from Rust/rules defaults and does not hydrate the ten `[Skirmish]` globals or seven Slot triples from RA2MD.INI. `handle_skirmish_shell_action` starts or leaves the shell without a `[Skirmish]` write on Start/Back. Existing app-layer Options persistence and `src/util/ini_writer.rs` provide reusable file-location and preservation patterns, but no batched Skirmish snapshot owner exists.

### 10.4 Per-AI difficulty flattening is adjacent, not part of this binary slice

The persisted Slot type code retains each AI row difficulty independently. Current Rust carries per-row `SkirmishAiSlot::difficulty` but later chooses the first opponent's difficulty for scalar `GameOptions`. The native downstream per-house difficulty consumer is covered by separate lobby/house research; it is an implementation dependency for faithful Slot round-trip but was not re-decompiled here.

## 11. Coverage Ledger

| Facet | Coverage | Evidence / disposition |
|---|---|---|
| offline Start caller/reachability | exhausted | `0x006ACEE0`, `0x006AE2C0`, `Main_Game` |
| offline Back caller/reachability | exhausted | same common command path and modal result |
| local country/colour helper order | exhausted | `0x0069B760`, `0x0069B7E0`, command assembly |
| human random-assignment loop | exhausted | decompile + full assembly `0x0069B8C0` |
| AI loop count/stride/order | exhausted | full assembly `0x0069B9D6..0x0069BAA0` |
| colour collision semantics | exhausted | `0x0069B600` plus inline AI scan |
| RNG primitive/source | exhausted for ordinary and Cooperative ranges in this slice | `0x0065C7E0`, all call-site `ECX` loads |
| base selected-mode country callbacks | exhausted | vtables + `0x005D6430`, `0x005D6440`, `0x0069B670` |
| Cooperative callback draw placement and retry mechanism | exhausted | `0x005C34F0`, `0x005C35F0`, progress getters, token resolver |
| Cooperative registry/parser and stock eligibility payload | exhausted | `0x0049DB00..0x0049E3A9`, `0x0049E3B0`, retail `CoopCampMD.ini` |
| Cooperative progress-map initialization and selected-map binding | exhausted | `0x0049B610`, `0x0049CAF0`, `0x0049D070`, `0x005C1470`, `0x005C1980`, `0x005C1A10..0x005C1E8A` |
| pre-game RNG full lifecycle across startup and repeat entry | exhausted for standard offline flow | constructor/boot seed/MPModes construction/Back/re-entry/match-start/normal-return direct-call census |
| `[Skirmish]` reader | exhausted | `0x00697F10`, caller `0x006980C0` |
| snapshot update order | exhausted | `0x006ACEE0` |
| writer trigger and abnormal exit | exhausted | `0x006AE2C0` |
| key order/format/slot bounds | exhausted | `0x00698F90` and helper formats |
| file setter/save failure propagation | exhausted at caller boundary | returns ignored, no UI branch |
| online lobby | deferred/out-of-scope | separate caller `0x005DC350` |
| paint/assets/pixels | deferred/out-of-scope | no visual claim |

## 12. Final Open-Question Log

- `[RESOLVED] OQ-01 - Is 0x00698F90 live in offline YR? -> Yes; 0x006AE2C0 unconditionally calls broader writer 0x006990A0, which calls 0x00698F90 for Session+0x18C/Skirmish.`
- `[RESOLVED] OQ-02 - Start or Back? -> Both commands take the common pack/randomize path and both reach the later unconditional writer.`
- `[RESOLVED] OQ-03 - What if dialog creation or the modal pump exits unusually? -> The writer still runs; no fresh command pack occurs unless Start/Back was dispatched.`
- `[RESOLVED] OQ-04 - What is the snapshot pointer? -> SessionClass+0x18C, not 0x008870C0.`
- `[RESOLVED] OQ-05 - Persistence order relative to randomization? -> Slot triples are packed raw before local colour and ProcessRandomAssignments; other option mirrors follow ProcessRandom; file write occurs after dialog teardown.`
- `[RESOLVED] OQ-06 - Exact keys/order/slots? -> Ten named globals followed by Slot01..Slot07, in reader/writer order documented above.`
- `[RESOLVED] OQ-07 - Write failure behavior? -> Per-key and final-save returns are ignored; this path exposes no error UI.`
- `[RESOLVED] OQ-08 - ProcessRandomAssignments callers? -> Offline 0x006ACEE0 and network/lobby 0x005DC350.`
- `[RESOLVED] OQ-09 - Human order/count? -> Pointer-vector order, count captured once at entry.`
- `[RESOLVED] OQ-10 - Marker clear order? -> Human marker is cleared and concrete field set to -1 before colour retry; local helper preserves -2 separately while writing a concrete mirror.`
- `[RESOLVED] OQ-11 - RNG function/source/bounds? -> inclusive RandomRanged on ScenarioClass+0x218; colour 0..7, ordinary country 0..9.`
- `[RESOLVED] OQ-12 - Collision scope? -> all humans plus all eight AI colour entries; unresolved human -1/-2 pair treated as -2.`
- `[RESOLVED] OQ-13 - AI loop? -> exactly eight parallel entries, dword stride, no active-count bound.`
- `[RESOLVED] OQ-14 - Saturation? -> unbounded retry; valid eight-player shell prevents exhaustion.`
- `[RESOLVED] OQ-15 - Observer branch? -> human observer canonicalization consumes no draw; AI array has no equivalent observer branch in this function.`
- `[RESOLVED] OQ-16 - Slot-zero mirror? -> local helper writes Session+0x15C first; ProcessRandomAssignments unconditionally re-mirrors first human node colour to the same address.`
- `[RESOLVED] OQ-17 - Is the offline country override pointer null? -> No for a selected mode. Base categories route to direct 0..9 Scenario draws; Cooperative uses eligibility-retry callbacks.`
- `[RESOLVED] OQ-18 - Zero/null/max edges? -> zero humans skips human loop; null selected mode uses direct country draw; malformed colour exhaustion loops forever.`
- `[RESOLVED] OQ-19 - Replay/pause/save relevance? -> Not part of the offline 0x102 command slice; no replay/pause gate occurs in the verified path.`
- `[RESOLVED] OQ-20 - Rust launch marker mismatch? -> UI launch conversion hardcodes color_random=false.`
- `[RESOLVED] OQ-21 - Rust resolver mismatch? -> algorithm is close for base modes, but it runs after Simulation creation on gameplay Scenario RNG and lacks Cooperative progress-map draws and country callbacks.`
- `[RESOLVED] OQ-22 - Rust persistence? -> no `[Skirmish]` read/write owner found.`
- `[RESOLVED] OQ-23 - Does local colour wait for ProcessRandomAssignments? -> No; 0x0069B7E0 resolves it before local node allocation.`
- `[RESOLVED] OQ-24 - Does Back consume RNG? -> Yes, for the current random fields, through the common command path.`
- `[RESOLVED] OQ-25 - Do shell draws advance gameplay RNG? -> No; later 0x0052FC20 overwrites both seeded gameplay RNG instances.`
- `[RESOLVED] OQ-26 - Does persistence save resolved AI colours? -> No; compact Slot triples are packed from raw controls before assignment resolution.`
- `[RESOLVED] OQ-27 - Read defaults? -> Slot01 type 6, Slot02..07 type 1, and country/colour -2 when absent.`
- `[RESOLVED] OQ-28 - Exact front-end Scenario cursor lifetime over startup, Back -> main menu -> re-enter, and normal match return? -> Seed zero is only the constructor default. Process initialization seeds the shared Scenario RNG, then stock Cooperative mode construction advances it through ten logical `(0,2)` calls before the first shell. Back/re-enter preserves the advanced cursor; successful Start reseeds it before gameplay; and a normal return retains the final gameplay cursor for the next shell.` Evidence: caller census and `0x0052BA8D`, `0x0052CAD2..0x0052CAF6`, `0x005C1470`, `0x0049CAF0`, `0x0052E168`, `0x0052E619` described in sections 3.8 and 4.
- **[RESOLVED] OQ-29 — Full semantic decode and Rust data source for Cooperative human/AI eligibility lists?** Retail `CoopCampMD.ini`; `[Campaigns]` section ordering plus `CampaignPlayerN` / `CampaignEnemyN` per-map token vectors, resolved through `0x0049D9F0`; callbacks retry data-driven global-country candidates until list membership succeeds. Evidence: parser `0x0049DB00..0x0049E3A9`, accessor `0x0049E3B0`, callbacks `0x005C34F0/0x005C35F0`, retail `ra2md.mix` payload.
- **[RESOLVED] OQ-30 — Which exact shell/progress writes bind the currently chosen Cooperative map filename to object `+0x40` CampaignType/CurrentMap?** `0x005C1980` maps the selected filename to a campaign; `0x005C1A10..0x005C1C7B` creates/binds active progress and stores campaign `+0x33C`; accepted switching at `0x005C1DC0..0x005C1E8A` moves the selected preconstructed record from `+0x328` to active `+0x40`, creates and random-initializes its replacement, and stores campaign `+0x4C`. `0x0049D070` returns the active record's chosen filename; country callbacks later read the same active record.

## 13. Implementation Handoff

| Native contract | Current Rust evidence | Required delta | Acceptance test | Risk / prohibition |
|---|---|---|---|---|
| visible Random Colour remains raw while launch copy resolves | `color_claimed` exists; launch hardcodes false | carry the sentinel explicitly into launch state and keep UI state unchanged | selecting Random still renders Random after preparing a launch copy | do not overwrite shell selection with resolved swatch |
| shell random draws use the current process-lifetime Scenario cursor and are later overwritten by successful-start game seeding | resolver receives `sim.scenario_rng` after load | add an app-owned Scenario cursor continuity object: seed it at process initialization, consume verified startup Cooperative progress draws in MPModes construction order, advance it on Cooperative selection and shell close, preserve it across Back/re-enter, seed gameplay independently, and replace it with the final gameplay Scenario state on return | fresh-stock fixed-seed transcript begins after ten logical `(0,2)` progress calls; two Back closes advance the next shell transcript; shell draws leave newly seeded gameplay state equal to a no-random launch with the same game seed; post-match shell starts from captured final gameplay state | do not run shell assignment on the newly seeded gameplay Simulation RNG, omit Cooperative progress draws, or reset the front-end cursor on every shell entry |
| ordinary order: local country, local colour, AI country/colour by row | current helper models human then AI but lacks command-stage split | preserve command-stage order and all collision retries | fixed-seed transcript with local + two AI random fields matches the verified call order | do not flatten to "all countries then all colours" |
| collision checks all concrete rows, including later AI rows | current helper pre-seeds concrete active opponent colours | retain full eight-row shell view during resolution, with inactive `-1` ignored naturally | later concrete AI colour forces local/earlier random redraw | do not filter only previously visited active rows |
| base selected modes use 0..9; Cooperative owns source-order progress records, three map variants per stage, and `CoopCampMD.ini` eligibility retries over the global country count | current helper always uses one hardcoded 0..9 draw and has no progress model | parse `CoopCampMD.ini` through `AssetManager`; construct source-order progress records after boot seeding; preserve each stage's chosen variant, active/reserve record swap, and replacement initialization; resolve player/enemy tokens against the active country roster; branch country authority by selected mode/progress | stock startup consumes the exact ten logical progress calls; accepted campaign switching consumes the selected stage count; Allied/Soviet/Yuri/World fixed-seed country transcripts accept only the exact player/enemy sets and preserve every rejected draw before colour | do not use `MPCoopMD.ini` as the eligibility source, regenerate unchanged records, omit reserve replacement draws, or cap retries while claiming parity |
| `[Skirmish]` loads once into a durable snapshot | no reader | parse ten globals plus seven triples at app/session initialization with native defaults | temp RA2MD.INI fixture hydrates map/mode/options/mixed row difficulties/random sentinels | do not treat Slot triple third value as team/start |
| Start and Back both pack, randomize, and save | action handler starts or leaves without write | centralize a common close/commit path; validation remains Start-only | Back persists current values and consumes expected shell draws; Start does the same after validation | do not put persistence only inside successful Start |
| one broader in-memory update is saved once | single-key writer exists | add/use a batched, preservation-safe INI update so all `[Skirmish]` keys commit together | comments, sibling sections, line endings, and unrelated duplicate sections stay intact under fixture policy | do not rewrite RA2MD.INI from a freshly serialized subset |
| per-row difficulty survives Slot type codes | launch carries it, gameplay flattens to first | preserve per-house/AI difficulty in downstream state | mixed Hard/Normal/Easy rows reach matching AI/house consumers | do not use one global difficulty as authority for every AI |

## 14. Corrections to Stale Research Prose

The following older claims must not be used as implementation authority without this correction:

- `fn-sessionclass-processrandomassignments.md` says the function is called once when Start is clicked. Offline Back also reaches it.
- That document describes the AI phase as the same observer/random logic as humans. The AI phase is two parallel eight-dword arrays and has no human observer canonicalization branch.
- It leaves the AI bound/collision helper unverified; both are now live-verified.
- `HOUSE_CREATION_COLOR_SYSTEM.md` calls `0..9` nine sides. It is ten inclusive integer results.
- Several older docs imply offline local Random Colour is first resolved by `ProcessRandomAssignments`. The command handler calls `0x0069B7E0` before local node allocation.
- Any prose identifying `0x008870C0` as the `[Skirmish]` snapshot is wrong; it is the INI object. Snapshot base is `Session+0x18C`.
- Slot triples are `(persisted row type, country, colour)`, not team/start triples.
- A Rust resolver that advances the post-seed gameplay Scenario RNG is not native-equivalent even if its local draw order otherwise matches.
- Cooperative country eligibility comes from `CoopCampMD.ini`, not the common-dialog override `MPCoopMD.ini`, and its bound is the current global country count rather than an intrinsic constant ten.
- Any front-end RNG lifecycle prose that starts the first Skirmish shell at the untouched boot-seeded cursor is stale: stock Cooperative progress construction makes ten logical `(0,2)` calls immediately after seeding, regardless of the mode later selected.

## 15. Sources

### Live Ghidra / gamemd.exe

- `decompile_function` / `disassemble_function`: `0x0049B610`, `0x0049BC80`, `0x0049BCC0`, `0x0049CAF0`, `0x0049D070`, `0x0049D9F0`, `0x0049DB00..0x0049E3A9`, `0x0049E3B0`, `0x0052BA60`, `0x0052D9A0`, `0x0052FC20`, `0x005C1470`, `0x005C1980`, `0x005C34F0`, `0x005C35F0`, `0x005D7590`, `0x005D7CE0`, `0x0065C7E0`, `0x00697F10`, `0x006980C0`, `0x00698F90`, `0x006990A0`, `0x0069B600`, `0x0069B760`, `0x0069B7E0`, `0x0069B8C0`, `0x006ACEE0`, `0x006AE2C0`.
- `get_function_callers`: `0x0069B8C0`, `0x00698F90`, `0x006990A0`, `0x006AE2C0`, `0x0052FC20`.
- `disassemble_bytes` / `get_assembly_context`: `0x0048CCCF`, `0x005C1A10..0x005C1E8A`, `0x005D82B0..0x005D82E6`, `0x005D6430`, `0x005D6440`, `0x0069B670`, `0x005C34F0`, `0x005C35F0`.
- `read_memory`: factory vtable `0x007EEE80`; mode vtables `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, `0x007EE424`, `0x007EE27C`; strings/formats `0x00817F6C`, `0x00825BF4`, `0x00825BF8`, `0x008189B0`, `0x0083EF9C`, `0x00826444`.

### Repository and retail evidence

- `docs/research/RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`
- `docs/research/skirmish-cell-ui/fn-sessionclass-processrandomassignments.md`
- `docs/research/skirmish-cell-ui/fn-sessionclass-readskirmishsettings.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COOPERATIVE_PRECALL_0049B760_GHIDRA_REPORT.md`
- `src/skirmish_launch.rs`
- `src/ui/skirmish_shell/state/launch.rs`
- `src/app_skirmish.rs`
- `src/app.rs`
- `src/util/ini_writer.rs`
- `<ra2-install>/RA2MD.INI`
- `<ra2-install>/ra2md.mix` (`CoopCampMD.ini` plaintext payload)
