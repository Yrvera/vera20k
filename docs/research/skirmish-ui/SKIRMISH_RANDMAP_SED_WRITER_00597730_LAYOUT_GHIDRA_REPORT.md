# Skirmish RandMap.Sed Writer 0x00597730 Layout - Ghidra Research Report

**Address(es):** `0x00597730`, concrete writer `0x00597760`, reader wrapper `0x00597A10`, concrete reader `0x00597A30`, constructor `0x00595680`, normalizer `0x005975E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `RandMap.Sed` writer/reader layout for Rust seed/options data: emitted `[RandomMap]` keys, value encoding, defaults, liveness from Create Random Map, and normalization boundaries.  
**Non-Scope:** terrain generation formulas inside `0x00598960`, `RandMap.img` preview format, random-map dialog visual layout, and exact `CCINIClass` internals beyond value encoding needed for this layout.  
**Confidence:** High for key names/order/value encoding/default behavior/liveness; Medium for malformed external `.SED` post-load consequences because no runtime experiment was performed.  
**Active in YR:** Conditional. Active when standard YR Create Random Map accepts and saves `DAT_00ABDFD8` to `RandMap.Sed`, and when a `.SED` selected map later loads through the random-map branch.

## Working Notes

- Target question: What exact `RandMap.Sed` layout does writer `0x00597730` emit, and how does reader `0x00597A30` consume it for a Rust seed/options model?
- Non-goals: Do not investigate `0x00598960` terrain formulas, `RandMap.img`, Choose Map list paint/preview refresh, or general INI parser internals outside this file layout.
- Evidence needed to mark COMPLETE: writer vtable target, reader vtable target, section/key names, value encoding, defaults/fallbacks, clamp/normalization boundary, Create Random Map liveness, Rust handoff.
- Stop conditions: stop once a Rust implementation can round-trip native `RandMap.Sed` seed/options and route launch correctly; defer only malformed runtime UX and terrain formula consumers.

## 1. Overview

`0x00597730` is a wrapper. With a non-null filename it calls `MapSeedClass` vtable slot `+0x8`, which resolves to concrete writer `0x00597760`; with a null filename it uses a separate existence/check helper and does not serialize this layout. The writer emits one `[RandomMap]` section. It writes `Description` first, then sixteen integer keys in native order. All integer values are written as signed decimal text using the integer INI writer's `%d` format path.

The matching reader wrapper `0x00597A10` calls vtable slot `+0x4`, concrete reader `0x00597A30`. The reader uses the existing object field as the default for every integer key, so a missing key preserves whatever the `MapSeedClass` object currently contains. Constructor defaults therefore matter for fresh objects, and accepted Create Random Map state matters because the writer serializes already-mutated dialog state.

## 2. Class Layout / Key Offsets

| Offset | Serialized key | Type / encoding | Constructor default | Writer evidence | Reader evidence | Active in YR |
|---|---|---:|---:|---|---|---|
| `+0x38` | `Theater` | decimal int | `0` | `0x0059789D..0x005978B0` | `0x00597B90..0x00597BAA` | Conditional |
| `+0x3C` | `MapType` | decimal int | `1` | `0x00597885..0x00597898` | `0x00597B76..0x00597BA2` | Conditional |
| `+0x40` | `Resources` | decimal int | `1` | `0x0059798D..0x005979A0` | `0x00597C97..0x00597CB2` | Conditional |
| `+0x44` | `Ruggedness` | decimal int | `0` | `0x005978E5..0x005978F8` | `0x00597BDE..0x00597BF8` | Conditional |
| `+0x48` | `Time` | decimal int | `1` | `0x005978B5..0x005978C8` | `0x00597BAD..0x00597BC7` | Conditional |
| `+0x4C` | `WaterAmount` | decimal int | `0` | `0x00597915..0x00597928` | `0x00597C12..0x00597C3E` | Conditional |
| `+0x50` | `NumPlayers` | decimal int | `2` | `0x00597855..0x00597868` | `0x00597B42..0x00597B5C` | Conditional |
| `+0x54` | `Tiberium` | decimal int | `0` | `0x0059792D..0x00597940` | `0x00597C2C..0x00597C46` | Conditional |
| `+0x58` | `TiberiumLayout` | decimal int | `0` | `0x00597945..0x00597958` | `0x00597C49..0x00597C63` | Conditional |
| `+0x5C` | `Vegetation` | decimal int | `0` | `0x0059795D..0x00597970` | `0x00597C60..0x00597C8C` | Conditional |
| `+0x60` | `UrbanPresence` | decimal int | `0` | `0x00597975..0x00597988` | `0x00597C7A..0x00597C94` | Conditional |
| `+0x64` | `Width` | decimal int, size bucket | `0` | `0x00597825..0x00597838` | `0x00597B11..0x00597B2B` | Conditional |
| `+0x68` | `Height` | decimal int, size bucket | `0` | `0x0059783D..0x00597850` | `0x00597B28..0x00597B54` | Conditional |
| `+0x6C` | `Accessibility` | decimal int | `0` | `0x005978FD..0x00597910` | `0x00597BFB..0x00597C15` | Conditional |
| `+0x70` | `RegionSize` | decimal int | `0` | `0x005978CD..0x005978E0` | `0x00597BC4..0x00597BF0` | Conditional |
| `+0x74` | `Seed` | decimal int | `-1` | `0x0059786D..0x00597880` | `0x00597B5F..0x00597B79` | Conditional |
| `+0x78` | `Description` | wide string encoded as comma-separated hex UTF-16 code units | localized `TXT_RANDOM_MAP_DESCRIPTION` if string-table helper succeeds, else empty | `0x00597804..0x00597820`, string writer `0x00528E00` | `0x00597ADE..0x00597B0C`, string reader `0x00528F00` | Conditional |

## 3. Core Logic

### Writer wrapper and concrete writer

Active in YR: Conditional. `0x00597730` loads the filename from `[ESP+4]`. If non-null, it pushes `this+0x78` and the filename into vtable slot `+0x8`. The `MapSeedClass` constructor writes vtable `0x007ED8E4`; bytes at `0x007ED8E4+0x8` resolve to `0x00597760`.

Evidence: wrapper assembly `0x00597730..0x00597744`; constructor `0x005956CB`; vtable bytes `70 C2 5A 00 30 7A 59 00 60 77 59 00`.

`0x00597760` returns false if the filename argument is null. Otherwise it logs `"Saving random map: %s - %ls\n"`, initializes a file/INI object for that filename, optionally copies the supplied description pointer into `this+0x78`, then writes the `[RandomMap]` section.

Evidence: `0x00597760..0x005979F4`; log string at `0x0082BBEC`; filename gate `0x00597772..0x00597776`.

### Emitted section and key order

Active in YR: Conditional. The section literal is `RandomMap` at `0x0082BB24`. The writer emits keys in this order:

1. `Description`
2. `Width`
3. `Height`
4. `NumPlayers`
5. `Seed`
6. `MapType`
7. `Theater`
8. `Time`
9. `RegionSize`
10. `Ruggedness`
11. `Accessibility`
12. `WaterAmount`
13. `Tiberium`
14. `TiberiumLayout`
15. `Vegetation`
16. `UrbanPresence`
17. `Resources`

Evidence: writer assembly `0x00597811..0x005979A0`, string table at `0x0082BB24..0x0082BBFF`, individual key addresses `0x0081B1A4`, `0x0082BBE4`, `0x0081A7A8`, `0x0082BBD8`, `0x0082BBD0`, `0x0082BBC8`, `0x00818658`, `0x0081F11C`, `0x0082BBBC`, `0x0082BBB0`, `0x0082BBA0`, `0x0082BB94`, `0x00817278`, `0x0082BB84`, `0x0082BB78`, `0x0082BB68`, `0x0082BB5C`.

### Value formatting

Active in YR: Conditional. All integer writes pass `0` as the format selector to `0x005275C0`. That path formats the value through `"%d"` (`0x00817F6C`), not `"$%X"` or `"%Xh"`. Values are therefore serialized as signed decimal text.

Evidence: every integer write in `0x00597825..0x005979A0` pushes `EBX` where `EBX=0` before the field value; integer writer context `0x005275C0..0x005276CD`; format strings `0x00817F6C="%d"`, `0x00825BB0="$%X"`, `0x00825BAC="%Xh"`.

`Description` is not stored as a plain string. The string writer `0x00528E00` iterates each UTF-16 code unit, converts it to base-16 text, appends comma delimiter `","`, then stores that ASCII sequence as the INI value. The reader `0x00528F00` reads the value, splits on comma, parses each token with `"%x"` (`0x00825BD4`), and writes UTF-16 code units into the output buffer, null-terminating it. Native output can therefore look like `52,61,6e,64,6f,6d,20,4d,61,70,` for `"Random Map"`-style text.

Evidence: string writer `0x00528E00..0x00528EC4`, delimiter `0x00817F70=","`, base-16 conversion call `0x007D468C`; string reader `0x00528F00..0x0052915A`, delimiter search `0x005290E9..0x0052913E`, parse format `0x00825BD4="%x"`.

### Reader defaults and missing-key behavior

Active in YR: Conditional. The reader wrapper `0x00597A10` calls vtable slot `+0x4`, resolved to `0x00597A30`. The concrete reader returns false for a null filename or failed file/section load, and true after reading the known keys.

Evidence: wrapper `0x00597A10..0x00597A2B`; vtable bytes at `0x007ED8E4+0x4`; reader filename gate `0x00597A39..0x00597A46`; success/failure returns `0x00597CE6..0x00597D34`.

For integer keys, the reader loads the current field value and passes it as the default into `0x005276D0`, then stores the returned value back to the same field. Missing keys preserve existing object state. On a freshly constructed object this means the constructor defaults in Section 2 are used; on a reused object, omitted keys can preserve previous state.

Evidence: repeated read/store pattern `0x00597B11..0x00597CB2`; integer reader `0x005276D0`; constructor defaults `0x00595693..0x005956C4`.

For `Description`, the reader first prepares a default string from localized `TXT_RANDOM_MAP_DESCRIPTION` into `this+0x78`, then calls the comma-hex string reader for `[RandomMap] Description`. If the key is missing/empty, the buffer remains the default/fallback text path.

Evidence: default string setup `0x00597ADE..0x00597AF8`; key read `0x00597AFD..0x00597B0C`; constructor/default string path `0x005956DE..0x00595704`; string key `0x0081B1A4`.

### Clamp and normalization boundary

Active in YR: Conditional. `0x005975E0` clamps an already-mutated `MapSeedClass` object, but `0x00597A30` does not call it, and the `.SED` launch caller proceeds directly from `0x00597A10` to `0x00598960(0,0)`.

Evidence: reader body `0x00597A30..0x00597D34` has no call to `0x005975E0`; launch caller `0x0068496B..0x00684989` calls `0x00597A10`, checks `BL`, then calls `0x00598960`; normalizer assembly `0x005975E0..0x0059772D`.

The normalizer's verified bounds are:

| Offset / key | Clamp |
|---|---|
| `+0x40 Resources` | `0..3` |
| `+0x3C MapType` | `0..4` |
| `+0x48 Time` | `0..3` |
| `+0x44 Ruggedness` | `0..100` |
| `+0x4C WaterAmount` | `0..100` |
| `+0x50 NumPlayers` | `2..8` |
| `+0x54 Tiberium` | `1..100` |
| `+0x58 TiberiumLayout` | `0..100` |
| `+0x5C Vegetation` | `0..100` |
| `+0x60 UrbanPresence` | `0..100` |
| `+0x64 Width` | `0..3` |
| `+0x68 Height` | `0..3` |
| `+0x6C Accessibility` | `0..100` |
| `+0x70 RegionSize` | `0..100` |
| `+0x74 Seed` | `0..65535` |

No clamp for `+0x38 Theater` was observed in `0x005975E0`; the dialog/setup code constrains it through UI option state. Do not claim reader-side clamping for external `.SED` files without a runtime/native malformed-file trace.

## 4. INI Keys

| Section | Key | Type | Native default on fresh object if missing | Writer emits? | Reader consumes? | Active in YR |
|---|---|---:|---:|---|---|---|
| `RandomMap` | `Description` | comma-separated hex UTF-16 words | localized `TXT_RANDOM_MAP_DESCRIPTION` or empty fallback | Yes | Yes | Conditional |
| `RandomMap` | `Width` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Height` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `NumPlayers` | decimal int | `2` | Yes | Yes | Conditional |
| `RandomMap` | `Seed` | decimal int | `-1` before normalization/dialog randomize | Yes | Yes | Conditional |
| `RandomMap` | `MapType` | decimal int | `1` | Yes | Yes | Conditional |
| `RandomMap` | `Theater` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Time` | decimal int | `1` | Yes | Yes | Conditional |
| `RandomMap` | `RegionSize` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Ruggedness` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Accessibility` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `WaterAmount` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Tiberium` | decimal int | `0` before normalization; normalizer minimum is `1` | Yes | Yes | Conditional |
| `RandomMap` | `TiberiumLayout` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Vegetation` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `UrbanPresence` | decimal int | `0` | Yes | Yes | Conditional |
| `RandomMap` | `Resources` | decimal int | `1` | Yes | Yes | Conditional |

No local repo `ini/rules*.ini` key controls these `.SED` fields. Theater data files contain many `RequiredForRMG=true` entries, but those are terrain template availability flags and are not this seed/options layout.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Create Random Map accept | `0x005E8590` calls `0x00597730` on `DAT_00ABDFD8` with `RandMap.Sed` after dialog result `1` | prior setup report; save call `0x005E85D1..0x005E85E2`; writer wrapper `0x00597730` | Conditional |
| Writer vtable | `MapSeedClass` vtable `+0x8` resolves to `0x00597760` | constructor `0x005956CB`; vtable bytes `0x007ED8E4` | Conditional |
| Reader vtable | `MapSeedClass` vtable `+0x4` resolves to `0x00597A30` | wrapper `0x00597A10`; vtable bytes `0x007ED8E4` | Conditional |
| Launch branch | `.SED` launch loads seed/options, then calls generator without reader-side clamp | `0x0068496B..0x00684989` | Conditional |
| Generator consumer | `0x00598960` immediately consumes `+0x74 Seed` to initialize RMG RNG | `0x0059897B..0x0059899B` | Conditional |

## 6. Current Rust Implementation Status

Rust has only the sentinel shell state. [src/skirmish_scenarios.rs](src/skirmish_scenarios.rs:14) defines `RANDMAP_SED`, and [src/skirmish_scenarios.rs](src/skirmish_scenarios.rs:82) creates a `RandomMapSentinel` with no seed/options model, no `.SED` layout parser/writer, `min_players=None`, `max_players=None`, and `official=false`.

[src/ui/skirmish_shell/state.rs](src/ui/skirmish_shell/state.rs:185) can upsert a random sentinel display name, but it does not model accepted random-map dialog state, seed/options, or native `.SED` persistence. [src/app.rs](src/app.rs:676) currently logs that Create Random Map generation is not implemented.

Launch still treats requested map strings as ordinary concrete maps. [src/app_init.rs](src/app_init.rs:255) routes requested map names to [src/app_list_maps.rs](src/app_list_maps.rs:147), which checks files/extensions and has no `.sed` random seed branch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00597730` wrapper | verified | `0x00597730..0x00597755` | none |
| writer vtable target `0x00597760` | verified | vtable bytes `0x007ED8E4+8`; `0x00597760..0x005979F4` | none |
| writer key order | verified | `0x00597811..0x005979A0` | none |
| integer decimal formatting | verified | `0x005275C0` format selector `0`, strings `0x00817F6C/0x00825BB0/0x00825BAC` | none |
| `Description` comma-hex UTF-16 encoding | verified | `0x00528E00..0x00528EC4`; `0x00528F00..0x0052915A` | none for layout |
| `0x00597A10` reader wrapper | verified | `0x00597A10..0x00597A2B` | none |
| reader vtable target `0x00597A30` | verified | vtable bytes `0x007ED8E4+4`; `0x00597A30..0x00597D34` | none |
| reader missing-key defaults | verified | repeated current-field default pattern `0x00597B11..0x00597CB2`; constructor defaults `0x00595693..0x005956C4` | malformed runtime UX deferred |
| normalizer bounds | verified | `0x005975E0..0x0059772D` | no reader-side clamp found; exact UI theater constraint is outside this layout |
| Create Random Map liveness | verified by prior report plus writer wrapper | `0x005E85D1..0x005E85E2`; `0x00597730` | none |
| Rust implementation delta | verified | `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_init.rs`, `src/app_list_maps.rs` | implementation not performed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which concrete method does writer wrapper 0x00597730 call? -> vtable +0x8, concrete 0x00597760.` (evidence: `0x00597730..0x00597744`; vtable bytes at `0x007ED8E4`)
- `[RESOLVED] OQ-02 - Which concrete method does reader wrapper 0x00597A10 call? -> vtable +0x4, concrete 0x00597A30.` (evidence: `0x00597A10..0x00597A1E`; vtable bytes at `0x007ED8E4`)
- `[RESOLVED] OQ-03 - What section is emitted and read? -> `RandomMap`.` (evidence: string `0x0082BB24`; writer/reader call sites)
- `[RESOLVED] OQ-04 - What keys are emitted? -> `Description`, then the sixteen integer keys listed in Section 3.` (evidence: `0x00597811..0x005979A0`)
- `[RESOLVED] OQ-05 - What keys are read? -> Same key set and order as writer.` (evidence: `0x00597AFD..0x00597CB2`)
- `[RESOLVED] OQ-06 - Are integers decimal or hex? -> Decimal signed `%d`.` (evidence: `0x005275C0`; format string `0x00817F6C`; writer format arg `0`)
- `[RESOLVED] OQ-07 - Is Description plain text? -> No, it is comma-separated hex UTF-16 code units.` (evidence: `0x00528E00`; `0x00528F00`; delimiter `0x00817F70`; parse format `0x00825BD4`)
- `[RESOLVED] OQ-08 - Does reader clamp values after loading? -> No reader-side normalizer call was found; launch calls generator directly after successful load.` (evidence: `0x00597A30..0x00597D34`; `0x0068496B..0x00684989`)
- `[RESOLVED] OQ-09 - What are fresh object defaults for missing keys? -> Constructor defaults in Section 2, including `Seed=-1`, `NumPlayers=2`, `MapType=1`, `Resources=1`, `Time=1`.` (evidence: `0x00595693..0x005956C4`)
- `[RESOLVED] OQ-10 - What are native clamp ranges when the normalizer is used? -> Listed in Section 3; notably `Width/Height/Resources/Time` are `0..3`, `MapType` is `0..4`, `NumPlayers` is `2..8`, `Seed` is `0..65535`.` (evidence: `0x005975E0..0x0059772D`)
- `[RESOLVED] OQ-11 - Is the writer live from standard YR Create Random Map? -> Yes, conditionally after dialog accept result 1.` (evidence: `0x005E85D1..0x005E85E2`; prior setup report)
- `[RESOLVED] OQ-12 - Is this TS-only legacy? -> No, it is reached from standard YR Skirmish Create Random Map and `.SED` launch; conditional only on user path/result.` (evidence: `0x005E8590`; `0x0068496B..0x00684989`)
- `[RESOLVED] OQ-13 - Does Rust currently model this layout? -> No, only a display sentinel exists.` (evidence: Rust scan listed in Section 6)
- `[DEFERRED] OQ-14 - Exact malformed external `.SED` player-facing error/overflow UX.` (category: needs-runtime-debugger; reason: static binary proves reader/default/no-clamp boundary, but exact downstream failure or permissive generation with bad values needs native runtime observation; next-step-if-pursued: launch crafted `.SED` files with out-of-range/missing values)
- `[DEFERRED] OQ-15 - Exact random-map dialog UI constraints for Theater value.` (category: out-of-scope; reason: this slot covers persisted layout; no `+0x38` clamp in `0x005975E0`; next-step-if-pursued: random-map dialog control/value report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `RandMap.Sed` is a `[RandomMap]` seed/options INI with one encoded `Description` and sixteen decimal integer keys. | writer `0x00597760`; reader `0x00597A30`; string/int helpers `0x00528E00`, `0x00528F00`, `0x005275C0`, `0x005276D0` | missing | new app/map-layer random-map seed model; `src/app_init.rs`; `src/app_list_maps.rs` | Add a `.sed` seed/options parser/writer that uses the exact section/key names and decimal integer values. | Native-style `RandMap.Sed` parses into the expected seed/options struct and can round-trip the known key order. Proposed test: `skirmish_randmap_sed_round_trips_native_randommap_keys` | Do not parse `RandMap.Sed` as a concrete map or omit fields because the UI sentinel already exists. |
| `Description` is serialized as comma-separated hex UTF-16 code units, not plain localized text. | writer `0x00528E00`; reader `0x00528F00`; delimiter `0x00817F70`; parse format `0x00825BD4="%x"` | missing | random-map seed/options parser/writer; shell display-name bridge | Decode/encode description as native wide-code-unit hex CSV when interoperating with `.SED`. | A `.SED` containing `Description=52,61,6e,64,6f,6d,20,4d,61,70,` decodes to `Random Map` and re-encodes with comma delimiters. Proposed test: `skirmish_randmap_sed_description_uses_hex_utf16_csv` | Do not write `Description=Random Map`; native reader expects the encoded form for this helper path. |
| Reader defaults are current object fields; fresh defaults come from constructor, and reader does not visibly clamp before launch generation. | constructor `0x00595693..0x005956C4`; reader default pattern `0x00597B11..0x00597CB2`; launch `0x0068496B..0x00684989` | missing | parser validation/default policy | Use native constructor defaults for missing keys; keep parser normalization separate from native writer/dialog normalization. | A `.SED` missing `NumPlayers`, `MapType`, `Resources`, and `Seed` yields defaults `2`, `1`, `1`, and `-1` before any Rust validation layer. Proposed test: `skirmish_randmap_sed_missing_keys_preserve_native_defaults` | Do not silently clamp every loaded external `.SED` and then claim native parity; native load path does not show that clamp. |
| Accepted Create Random Map state should be normalized before write, with normalizer ranges listed in Section 3; the writer serializes object fields as-is. | normalizer `0x005975E0..0x0059772D`; writer field reads `0x00597825..0x005979A0`; prior dialog/setup paths | missing | `src/ui/skirmish_shell/state.rs`; future random-map dialog state | When Rust implements Create Random Map accept, normalize dialog values before writing/storing the `.SED` model, but keep external file parsing default behavior distinguishable. | Accepting dialog values outside UI bounds stores clamped values such as `NumPlayers=2..8`, `Seed=0..65535`, `Tiberium=1..100`. Proposed test: `skirmish_randmap_create_dialog_clamps_before_sed_write` | Do not put clamp logic only in the generator after parse; native dialog/setup state is normalized before writer use. |
| `.SED` launch liveness is writer -> selected sentinel -> reader -> `0x00598960(0,0)`, not preview reuse. | save call `0x005E85D1..0x005E85E2`; reader/generator caller `0x0068496B..0x00684989`; generator seed use `0x0059897B..0x0059899B` | missing | `src/app.rs`, `src/app_init.rs`, `src/app_list_maps.rs`, random map generator entry | Route selected `.sed` tokens to random-map generation using parsed seed/options before normal Skirmish spawn setup. | Selecting/accepting `RandMap.Sed` calls a random-map load branch and does not report "map not found". Proposed test: `skirmish_randmap_sed_launch_uses_seed_options_branch` | Do not use `RandMap.img` or modal preview bytes as gameplay terrain. |

### Negative Facts / Do Not Do

- Do not serialize `Description` as plain text. Active in YR: No for this writer. Evidence: `0x00528E00` hex-code-unit writer and `0x00528F00` hex-code-unit reader.
- Do not serialize integer values as hex. Active in YR: No for this writer. Evidence: `0x005275C0` with format selector `0`, `%d` at `0x00817F6C`.
- Do not claim `.SED` reader-side clamping. Active in YR: No evidence on the launch path; reader goes straight to generator caller. Evidence: `0x00597A30..0x00597D34`, `0x0068496B..0x00684989`.
- Do not treat `Width` and `Height` as literal cell dimensions in the `.SED` model. Active in YR: No; the persisted values are option buckets with normalizer range `0..3`. Evidence: writer/reader offsets `+0x64/+0x68`, clamp `0x005976C2..0x005976EA`.
- Do not treat `RandMap.Sed` as a normal map INI. Active in YR: No; `.SED` launch calls seed reader then generator. Evidence: `0x0068496B..0x00684989`.

### Remaining Uncertainty

- Malformed external `.SED` UX and downstream behavior for out-of-range values needs native runtime testing. Static evidence proves no reader-side call to `0x005975E0`, but not the player-visible failure mode for every bad value.
- The exact random-map dialog control constraints for `Theater` are outside this report. No `+0x38` clamp was observed in `0x005975E0`, but normal UI paths still constrain the value before writing.
- Full terrain-generation interpretation of each option remains in the generator formula reports.

### Stale Docs / Follow-up Docs

Path: `docs/research/skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`

Replace the deferred layout sentence with:

> `0x00597730` resolves through `MapSeedClass` vtable `+0x8` to writer `0x00597760`, which emits a `[RandomMap]` section containing `Description`, `Width`, `Height`, `NumPlayers`, `Seed`, `MapType`, `Theater`, `Time`, `RegionSize`, `Ruggedness`, `Accessibility`, `WaterAmount`, `Tiberium`, `TiberiumLayout`, `Vegetation`, `UrbanPresence`, and `Resources`. Integer fields are signed decimal `%d`; `Description` is comma-separated hex UTF-16 code units. Reader `0x00597A30` uses current object fields as per-key defaults and does not visibly clamp before launch calls `0x00598960(0,0)`.

Path: `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`

Refine the clamp statement with:

> `0x005975E0` is the dialog/setup normalizer and clamps persisted option fields before native-created `RandMap.Sed` is written, but the `.SED` reader `0x00597A30` itself was not observed to call the normalizer before launch generation. External/malformed `.SED` handling therefore remains a runtime-observation question.

## Sources

- Ghidra read-only assembly/memory: `0x00595680`, `0x005975E0`, `0x00597730`, `0x00597760`, `0x00597A10`, `0x00597A30`, `0x00598960`, `0x00684961`, `0x00528E00`, `0x00528F00`, `0x005275C0`, `0x005276D0`.
- Ghidra memory/string evidence: vtable `0x007ED8E4`, `[RandomMap]` and key strings at `0x0082BB24..0x0082BBFF`, `Description` at `0x0081B1A4`, `Height` at `0x0081A7A8`, `Theater` at `0x00818658`, `Time` at `0x0081F11C`, `Tiberium` at `0x00817278`, integer formats `0x00817F6C`, `0x00825BB0`, `0x00825BAC`, string reader format `0x00825BD4`.
- Prior docs referenced: `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_init.rs`, `src/app_list_maps.rs`.
