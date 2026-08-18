# Loading Theater Dynamic 13–25 Progress — Ghidra Research Report

**Address(es):** `Init_Theater 0x005349C0`; loop
`0x00534D31..0x00534DAE`; final callback `0x00534DC5`;
`Init_Color_Schemes_INI 0x0066D3A0`; create/find helper `0x0068C9C0`;
`ColorScheme` constructor `0x0068C710`; scheme rebuild helper `0x0068C860`;
progress callback `0x0069AE90`; selected-map caller
`ScenarioClass__Full_Init 0x00686B20`; later cache write `0x004AD7BC`.

**Investigation Mode:** exhaustive-slice.

**Claimed Scope:** the live selected-map `Init_Theater` branch that computes and
visibly presents changing loading-progress values from 13 through 25, including
its theater-cache gate, runtime scheme-count provenance, exact stock-YR fixture,
and the corresponding current Rust data source and lifecycle.

**Non-Scope:** MMPB markers, loading text, PROGBARM pixel rounding, random-map
generator milestones, campaign loading, and exact dwell time between presented
progress frames.

**Confidence:** High for the native formula, gates, count provenance, stock
fixture, and Rust mapping. The exact OS-visible dwell time of immediately
successive Rust presents is not claimed.

**Active in YR:** Yes. Standard offline Skirmish reaches `Init_Theater` from
`ScenarioClass__Full_Init`; the dynamic branch runs on the first process load and
whenever the requested theater differs from the cached theater.

## 1. Overview

`Init_Theater` does not emit a hardcoded `13..25` list. It rebuilds one
theater-dependent conversion object for every runtime `ColorScheme`, derives a
progress candidate from the current scheme index, and calls the global progress
callback only when that candidate changes.

The runtime array contains two schemes per `[Colors]` entry. Retail
`rulesmd.ini` has 21 entries, therefore stock YR enters this loop with
`N = 42`. Its quotient is `q = trunc(42 / 13) = 3`, producing one visible
advance every three scheme rebuilds: `13,14,...,25`.

## 2. Key State

| State | Type / value | Meaning | Evidence |
|---|---|---|---|
| `0x00B054D4` | pointer array | runtime `ColorScheme*` array | `decompile_function 0x005349C0`; `0x0068C9C0`; `0x0068C710` |
| `0x00B054E0` | signed `int` | current runtime scheme count `N` | `audit_globals_in_function 0x005349C0`; `get_xrefs_to 0x00B054E0` |
| `0x00822CF8` | signed theater enum cache | last theater whose map-section setup completed | initial bytes `FF FF FF FF` from `read_memory 0x00822CF8 16`; write at `0x004AD7BC` |
| local `ESI` | signed `int` | loop index `i`, initialized to zero | `0x00534D43`, `0x00534DAB` |
| local `EBX` | signed `int` | quotient snapshot `q = trunc(N / 13)` | `0x00534D31..0x00534D51` |
| local `EBP` | signed `int`, initial `12` | previously emitted local candidate | `0x00534D4C`, updated at `0x00534D9F` |
| local `EDI` | signed `int` | clamped candidate sent to progress callback | `0x00534D84..0x00534D9A` |

## 3. Exact Native Algorithm

The assembly, not the decompiler's labels, is authoritative:

```text
N = *(i32*)0x00B054E0
q = trunc_signed(N / 13)                 // signed magic divide
i = 0
previous = 12

if N > 0:
    do:
        rebuild ColorSchemeArray[i] for the new theater palettes
        candidate = trunc_signed(i / q) + 12
        if candidate >= 25:
            candidate = 25
        if candidate != previous:
            ProgressReport(candidate)
            previous = candidate
        Network_ServiceLoop()
        i += 1
    while i < live_count

finalize theater conversion state
ProgressReport(25)
```

### Tiny details that are load-bearing

1. `N/13` is a signed division compiled as the `0x4EC4EC4F` magic multiply,
   arithmetic shift, and sign correction at `0x00534D31..0x00534D48`.
2. The quotient is computed once before the loop and retained in `EBX`.
3. The loop bound is not retained: `N` is re-read from `0x00B054E0` at
   `0x00534DA6` before every signed `i < N` test.
4. Normal offline loading does not mutate the scheme array during the loop, so
   the quotient snapshot and live bound both observe 42 for the retail fixture.
5. The loop index starts at zero (`XOR ESI,ESI`) and increments after the
   network-service call.
6. The per-entry work occurs before the candidate is computed:
   `ColorSchemeArray[i]` is read at `0x00534D55..0x00534D6A`, then helper
   `0x0068C860` is called at `0x00534D77`.
7. Helper `0x0068C860` destroys the old `ColorScheme+0x30C` conversion object,
   rebuilds it from the new theater/unit palettes through `0x0068C3B0`, and
   stores the replacement before progress advances.
8. `i/q` is an explicit signed `CDQ; IDIV EBX` at
   `0x00534D7C..0x00534D7F`.
9. The constant `12` is added after division.
10. The cap is `candidate >= 25 -> 25`, not only `candidate > 25`:
    `CMP EAX,0x19; JL ...; MOV EDI,0x19`.
11. The local previous value begins at 12, so the `i=0` candidate is not sent.
12. Unchanged candidates still rebuild a scheme and service the network; only
    the progress callback is suppressed.
13. A changed candidate is sent before `previous` is updated.
14. The loop uses signed `JL` for its bound.
15. The unconditional final raw `25` is called after theater conversion
    finalization (`0x006267A0`, `0x00717840`).
16. The global callback `0x0069AE90` independently suppresses that final `25`
    when the loop already reached 25.
17. `N=0` skips the loop but still invokes the final raw `25`.
18. `1 <= N <= 12` produces `q=0`, enters the loop, and reaches signed
    `IDIV 0`; native therefore relies on the runtime invariant `N==0 || N>=13`.
19. Negative `N` would skip the loop after computing the quotient; construction
    and destruction paths maintain a nonnegative count in normal YR.
20. `Network_ServiceLoop 0x0048D080` runs once per scheme rebuild. Its
    multiplayer progress-message branches are inactive for `g_GameMode == 5`.

## 4. Stock YR Fixture

`Init_Color_Schemes_INI 0x0066D3A0` enumerates `[Colors]` and calls
create/find helper `0x0068C9C0` twice per key: once with type `1`, then with type
`0x35`. `ColorScheme` construction at `0x0068C710` appends the new object and
increments `0x00B054E0`.

Retail counts:

| Source | `[Colors]` entries | Runtime schemes |
|---|---:|---:|
| `ini/rulesmd.ini` | 21 | 42 |
| `ini/rules.ini` base fallback | 19 | 38 |

YR `rulesmd.ini` is authoritative, so the stock selected-map fixture is:

```text
N = 42
q = trunc(42 / 13) = 3
```

| Scheme indices completed | Candidate | Callback? |
|---|---:|---|
| `i=0..2` | 12 | no; local previous is 12 |
| `i=3..5` | 13 | once, at `i=3` |
| `i=6..8` | 14 | once, at `i=6` |
| continuing every three indices | `15..24` | once per changed value |
| `i=39..41` | 25 | once, at `i=39` |
| unconditional final callback | 25 | invoked, then globally suppressed |

Thus the exact stock visible loop output is every integer `13..25`, inclusive.

## 5. Theater Cache Gate And Ordering

`Init_Theater` emits raw `8` before comparing the requested theater against
`0x00822CF8`. The archive reload, raw `6`, raw `12`, dynamic loop, and final raw
`25` are all inside the mismatch branch.

The static cache value is `-1`, so the first valid theater always mismatches.
Later `Read_Map_Section_And_IsoMapPacks` stores
`ScenarioClass+0x1258` to the cache at `0x004AD7AC..0x004AD7BC`. Consequences:

- first selected-map load: `8`, suppressed `6`, `12`, dynamic `13..25`;
- later different-theater load: the same sequence;
- later same-theater load: `8`, then return; no `6`, `12`, dynamic loop, or
  final `25`;
- a load that fails before the cache write leaves the old cache, so a retry
  still takes the mismatch branch.

`ScenarioClass__Full_Init` calls `Init_Theater` at `0x0068765B`, then emits raw
`30`. It resets and reloads type registries only after that (`0x006686C0` after
raw `31`). Therefore the count consumed by `Init_Theater` is the scheme table
from the rules state already active before this selected-map load.

## 6. INI Inputs

No dedicated progress INI key exists. The loop consumes the size of the
already-built `[Colors]` scheme table.

| Source | Effect |
|---|---|
| `rulesmd.ini [Colors]` | 21 ordered YR entries; authoritative stock count |
| `rules.ini [Colors]` | 19-entry base fallback |
| `[Basic] Theater` in the selected map | selects the requested theater compared with the cache |

The values of individual H,S,V triples do not affect progress; only the number
of distinct runtime scheme objects does.

## 7. Current Rust Mapping

Rust already has the correct count source:

- `AppState` loads `startup_rules` before the shell from
  `app_init_helpers::load_rules_ini(..., None, None)` in `src/app.rs`.
- `RuleSet::from_ini` stores declaration-ordered `color_schemes` from
  `parse_color_schemes` and builds one Rust ramp per entry.
- Native runtime count maps to `2 * state.rules.color_schemes.len()`, because
  Rust intentionally stores one source entry rather than the native doubled
  object pair.
- After a completed match, `state.rules` is replaced by the loaded match
  rules, matching native's use of the currently active pre-load scheme table on
  the next match.
- `state.loaded_map_source == None` distinguishes the first successful map
  load; `state.theater_name` retains the last successful theater thereafter.
  Together they can model native's initial `-1` cache and later equality test.

Current drift:

- `src/app_init.rs::load_map_from_initial` emits raw `12`, raw `25`, then raw
  `30` unconditionally after `theater::load_theater`.
- It neither emits the count-derived changed values nor skips `12/25` for a
  same-theater reload.
- `src/app_loading.rs::theater_ramp_changed_values` only filters caller-supplied
  values and is used by tests; it does not derive the native sequence or feed
  the live loader.

## 8. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| selected-map owner and liveness | verified | `get_function_xrefs 0x005349C0`; `decompile_function 0x00686B20` | none |
| cache initial value and mismatch gate | verified | `read_memory 0x00822CF8 16`; `decompile_function 0x005349C0` | none |
| cache successful-load write | verified | `get_xrefs_to 0x00822CF8`; `disassemble_bytes 0x004AD790..0x004AD7D5` | none |
| quotient formula and signedness | verified | `disassemble_bytes 0x00534D20..0x00534DA5` | none |
| loop order, cap, changed-only callback | verified | same disassembly; `decompile_function 0x005349C0` | none |
| per-scheme conversion work | verified | `decompile_function 0x0068C860` | exact wall-clock cost is machine-dependent |
| final 25 and duplicate suppression | verified | `disassemble_bytes 0x00534DA0..0x00534DD0`; `decompile_function 0x0069AE90` | none |
| scheme-count construction | verified | `decompile_function 0x0066D3A0`; `batch_decompile 0x0068C9C0,0x0068C710` | none |
| stock YR count | verified | `ini/rulesmd.ini [Colors]`; two constructors per key | none |
| registry reset ordering | verified | `decompile_function 0x00686B20`; `decompile_function 0x006686C0` | none |
| current Rust source and lifecycle | verified | `src/app.rs`, `src/rules/ruleset.rs`, `src/app_loading.rs`, `src/app_init.rs` | implementation required |
| exact visible dwell per percentage | deferred | no runtime capture | runtime capture after implementation |

## 9. Open Questions — Final State

- `[RESOLVED] OQ-01 — Is Init_Theater live for selected-map Skirmish? -> Yes, Full_Init calls it before raw 30.` (evidence: `0x0068765B`)
- `[RESOLVED] OQ-02 — What gates the dynamic branch? -> Requested theater != cached theater.` (evidence: `decompile_function 0x005349C0`)
- `[RESOLVED] OQ-03 — Does the first load take the branch? -> Yes; cache starts at -1.` (evidence: `read_memory 0x00822CF8 16`)
- `[RESOLVED] OQ-04 — When is the cache updated? -> Near the successful end of map-section loading.` (evidence: `0x004AD7AC..0x004AD7BC`)
- `[RESOLVED] OQ-05 — What is q? -> Signed truncation of N/13, computed once.` (evidence: `0x00534D31..0x00534D51`)
- `[RESOLVED] OQ-06 — What is the iteration range? -> Signed i=0 while i<live N, with N re-read each iteration.` (evidence: `0x00534DA6..0x00534DAE`)
- `[RESOLVED] OQ-07 — What is the candidate formula? -> trunc_signed(i/q)+12.` (evidence: `0x00534D7C..0x00534D81`)
- `[RESOLVED] OQ-08 — Is the cap >25 or >=25? -> Candidate >=25 is replaced by 25.` (evidence: `0x00534D84..0x00534D8B`)
- `[RESOLVED] OQ-09 — Why is 12 not emitted by the loop? -> Local previous begins at 12.` (evidence: `0x00534D4C`, `0x00534D90`)
- `[RESOLVED] OQ-10 — What does each iteration rebuild? -> ColorScheme+0x30C theater conversion state.` (evidence: `decompile_function 0x0068C860`)
- `[RESOLVED] OQ-11 — Where does N come from? -> Global runtime ColorScheme array count.` (evidence: `audit_globals_in_function 0x005349C0`)
- `[RESOLVED] OQ-12 — Why is N doubled? -> Init_Color_Schemes_INI creates types 1 and 0x35 per [Colors] key.` (evidence: `decompile_function 0x0066D3A0`)
- `[RESOLVED] OQ-13 — What is stock YR N? -> 42 from 21 rulesmd entries.` (evidence: `ini/rulesmd.ini [Colors]`)
- `[RESOLVED] OQ-14 — What exact values result for N=42? -> 13..25 inclusive, first at i=3 and 25 at i=39.` (evidence: verified formula fixture)
- `[RESOLVED] OQ-15 — Is final 25 conditional? -> The call is unconditional inside the theater-mismatch branch; its visible effect is monotonic-gated.` (evidence: `0x00534DBE..0x00534DC5`; `0x0069AE90`)
- `[RESOLVED] OQ-16 — What happens for N=0? -> Loop skips; final raw 25 still occurs.` (evidence: `decompile_function 0x005349C0`)
- `[RESOLVED] OQ-17 — What happens for 1..12? -> Native reaches IDIV with q=0; this is outside the valid construction invariant.` (evidence: `0x00534D31..0x00534D7F`)
- `[RESOLVED] OQ-18 — Which current Rust collection maps to N? -> Two times RuleSet.color_schemes.len().` (evidence: `src/rules/ruleset.rs`; native doubling)
- `[RESOLVED] OQ-19 — Is that Rust collection available before theater loading? -> Yes, AppState holds startup/current rules before begin_loading.` (evidence: `src/app.rs` startup_rules; `src/app_loading.rs::begin_loading`)
- `[RESOLVED] OQ-20 — How can Rust represent the theater cache? -> first-load state from loaded_map_source plus last successful theater_name.` (evidence: `src/app.rs`; `src/app_transitions.rs`)
- `[RESOLVED] OQ-21 — Are pause/replay/save-restore relevant? -> No; this is a synchronous pre-match load slice with process/session cache state.` (evidence: owner call chain)
- `[DEFERRED] OQ-22 — How long is each percentage physically visible?` (category: `needs-runtime-debugger`; reason: native work and display timing are hardware/runtime dependent; next-step-if-pursued: record timestamped callback/present frames in both engines)
- `[DEFERRED] OQ-23 — Should Rust deliberately fault for malformed 1..12 runtime counts?` (category: `out-of-scope`; reason: stock and valid parsed rule sets satisfy N>=38; next-step-if-pursued: define malformed-mod compatibility policy)

Adversarial checks answered by the entries above: zero schemes, fewer than
thirteen schemes, first TEMPERATE load despite Rust's default theater string,
same-theater second load, load failure before cache commit, and count stability
across the registry reset.

## 10. Visual/UI Composition Ledger

This slice owns only progress ordering; existing loading reports own geometry.

| Order | Function / address | Condition | Visual effect | Active for selected-map target? |
|---:|---|---|---|---|
| 1 | `Init_Theater` callback raw 8 | always on entry | advances from first visible 3 to 8 | yes |
| 2 | raw 6 | theater mismatch | globally suppressed after 8 | yes as call, no visual advance |
| 3 | raw 12 | theater mismatch | visible 12 | first/different theater |
| 4 | dynamic loop | theater mismatch and N>0 | visible changed values 13..25 | first/different theater |
| 5 | final raw 25 | theater mismatch | duplicate-suppressed for stock N=42 | first/different theater |
| 6 | `Full_Init` raw 30 | after `Init_Theater` returns | visible 30 | yes |

Asset role matrix:

| Asset/data | Loaded | Used by loop | Visible role | Evidence |
|---|---:|---:|---|---|
| theater palette | yes | yes | rebuilt conversion source before each candidate | `0x005349C0`; `0x0068C860` |
| unit theater palette | yes | yes | second conversion source | `0x005349C0`; `0x0068C860` |
| `PROGBARM.SHP` | already loaded | no direct access here | receives each advancing percentage through ProgressClass | `0x0069AE90`; sibling loading reports |
| `[Colors]` scheme table | already built | count and per-entry object | determines cadence, not bar color in this loop | `0x0066D3A0`; `0x00534D31` |

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native pre-load scheme count is two times the current `[Colors]` entry count. | `0x0066D3A0`; `0x0068C9C0`; `0x0068C710` | count not retained on loading state | `src/app_loading.rs` | capture `2 * state.rules.color_schemes.len()` when loading begins | stock startup rules produce runtime count 42 | do not use only the selected player's scheme or hardcode 42 |
| On theater mismatch, changed loop values are `min(trunc(i/(N/13))+12,25)` for signed i=0..N-1, previous=12. | `0x00534D31..0x00534DAE` | helper filters supplied values instead of deriving them | `src/app_loading.rs` helper/tests | derive the changed sequence from the captured runtime count | N=42 emits exactly 13..25; N=38 emits 13..25 with the verified quotient behavior | do not emit every integer independently of N |
| Raw 12, dynamic loop, and final raw 25 are inside the theater-cache mismatch branch. | `decompile_function 0x005349C0`; cache write `0x004AD7BC` | Rust emits 12 and 25 on every load | `src/app_loading.rs`, `src/app_init.rs` | compute first/different-theater gate from successful-load state and apply it around 12/loop/25 | first TEMPERATE load emits the branch; a second TEMPERATE load emits 8 then 30; TEMPERATE→SNOW emits the branch | do not compare only against the AppState default `"TEMPERATE"` on the first load |
| Each advancing value goes through the existing synchronous presentation sink. | `0x0069AE90`; current `RenderingProgressSink` | live loader omits derived values | `src/app_init.rs` | call the sink with each derived raw value before raw 30 | captured selected-map ledger contains 8,12,13..25,30 in order on a cold load | do not interpolate or sleep between milestones |

Proposed focused tests:

- `theater_ramp_stock_rulesmd_count_emits_13_through_25`
- `theater_ramp_zero_count_leaves_only_final_25_boundary`
- `loading_theater_progress_first_same_and_changed_cache_cases`
- `loading_progress_ledger_places_dynamic_theater_values_between_12_and_30`

## Sources

- Ghidra read-only:
  `audit_globals_in_function 0x005349C0`;
  `decompile_function 0x005349C0`;
  `disassemble_bytes 0x00534D20..0x00534DA5`;
  `disassemble_bytes 0x00534DA0..0x00534DD0`;
  `decompile_function 0x0068C860`;
  `decompile_function 0x0066D3A0`;
  `batch_decompile 0x0068C9C0,0x0068C710`;
  `decompile_function 0x0069AE90`;
  `get_function_xrefs 0x005349C0`;
  `decompile_function 0x00686B20`;
  `get_xrefs_to 0x00B054E0`;
  `decompile_function 0x006686C0`;
  `read_memory 0x00822CF8 16`;
  `get_xrefs_to 0x00822CF8`;
  `disassemble_bytes 0x004AD790..0x004AD7D5`;
  `decompile_function 0x0048D080`.
- Retail INI: `ini/rulesmd.ini [Colors]`, `ini/rules.ini [Colors]`.
- Prior reports:
  `LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md`;
  `LOADING_FUN_0069AE90_SKIRMISH_CALLERS_AFTER_FIRST_RENDERER_GHIDRA_REPORT.md`;
  `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`;
  `HOUSE_COLOR_REMAP_PIPELINE_GHIDRA_REPORT.md`.
- Rust:
  `src/app.rs`;
  `src/app_transitions.rs`;
  `src/rules/color_scheme.rs`;
  `src/rules/ruleset.rs`;
  `src/app_loading.rs`;
  `src/app_init.rs`.

**Status:** COMPLETE for the selected-map dynamic theater progress slice. No
material open question remains for implementing the count-derived sequence and
the first/different-theater gate. Exact frame dwell remains a separate runtime
capture question and does not change the emitted values or ordering.
