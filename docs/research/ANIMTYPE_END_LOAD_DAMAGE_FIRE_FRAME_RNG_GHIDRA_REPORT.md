# AnimType `End` Load and Damage-Fire Frame RNG — Ghidra Research Report

**Primary addresses:** `0x00427530`, `0x00427B50`, `0x00427D00`, `0x00421EA0`, `0x0043C0D0`, `0x0065C7E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the fresh `AnimTypeClass` construction/art-load/INI-read chain that resolves `AnimType+0x2C0` (`End`), and the subsequent use of that value by stock building damage-fire creation for `FIRE01`, `FIRE02`, and `FIRE03`  
**Non-Scope:** the full `AnimClass::AI` lifecycle, sprite draw composition, damage-fire threshold/offset selection except where it determines entry into the frame-RNG path, save/load, and mixer timing  
**Confidence:** High for the claimed slice  
**Active in YR:** Yes — `rulesmd.ini` selects these three stock types through `DamageFireTypes=` and stock damaged buildings reach `BuildingClass::CreateDamageFireAnims`

## 1. Overview

Fresh animation types do not generally keep an omitted `End=` at the constructor value zero. `AnimTypeClass::ReadINI` invokes the type's normal SHP-loading virtual before it reads the explicit `End=` key; that loader replaces a zero `End` with the signed 16-bit SHP header frame count, halving it only when `Shadow=yes`. An explicit `End=` is then read afterward and can override the loaded value.

`BuildingClass::CreateDamageFireAnims` constructs each fire first, then reads the constructed type's resolved `End` and calls scenario `RandomRanged(0, End - 1)` only when that value is positive. For unmodified stock data, the resulting bounds are `0..29` for `FIRE01`, `0..63` for `FIRE02`, and `0..29` for `FIRE03`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Width / interpretation | Verified role | Evidence |
|---|---:|---|---|---|
| `ObjectTypeClass` | `+0x24` | 25-byte name buffer | type/section identifier copied into the initial image-name buffer | Ghidra MCP `decompile_function(address=0x005F7090)` |
| `ObjectTypeClass` | `+0x1F8` | 25-byte image-name buffer | nonempty after construction even when `Image=` is omitted | `0x005F7090` copy from `this+0x24` to `this+0x1F8` |
| `ObjectTypeClass` | `+0xA4` | pointer | loaded SHP pointer used by the art loader | Ghidra MCP `decompile_function(address=0x00427B50)` |
| `AnimTypeClass` | `+0x298` | signed `int` cache | raw SHP header frame count divided by two, independent of `End` and `Shadow`; not the damage-fire RNG bound | `0x00427B50` |
| `AnimTypeClass` | `+0x2A4` | byte | set to one when the loader has a non-null SHP | `0x00427B50` |
| `AnimTypeClass` | `+0x2B0` | signed `int` | native rate in logic frames | `0x00427530`, `0x00427D00` |
| `AnimTypeClass` | `+0x2B4` | signed `int` | `Start` | `0x00427D00` |
| `AnimTypeClass` | `+0x2B8` | signed `int` | `LoopStart` | `0x00427D00` |
| `AnimTypeClass` | `+0x2BC` | signed `int` | `LoopEnd` | `0x00427B50`, `0x00427D00` |
| `AnimTypeClass` | `+0x2C0` | signed `int` | resolved `End`; exact damage-fire frame-RNG exclusive upper limit | `0x00427530`, `0x00427B50`, `0x00427D00`, `0x0043C0D0` |
| `AnimTypeClass` | `+0x2E4/+0x2E8` | two signed `int`s | `RandomRate` endpoints | `0x00427530`, `0x00427D00`, `0x00421EA0` |
| `AnimTypeClass` | `+0x372` | byte | `Shadow`; false in a fresh type | `0x00427530`, `0x00427D00` |
| `AnimClass` | `+0xAC` | signed `int` | current/start frame written by damage-fire RNG | `0x0043C0D0` |
| `AnimClass` | `+0xC8` | pointer | `AnimTypeClass*` read by damage-fire creation | `0x0043C0D0` |

## 3. Core Logic

### 3.1 Exact construction and load order

1. `ObjectTypeClass` construction receives the type identifier and copies its `+0x24` name buffer into `+0x1F8`. Therefore `[FIRE01]` has an effective initial image name of `FIRE01` even without an `Image=` key. The same holds for `FIRE02` and `FIRE03`. Evidence: Ghidra MCP `decompile_function(address=0x005F7090)`.
2. `AnimTypeClass` construction initializes `LoopEnd=0`, `End=0`, `RandomRate=(0,0)`, and `Shadow=false`. Evidence: Ghidra MCP `decompile_function(address=0x00427530)`.
3. `AnimTypeClass::ReadINI` first runs the base object-type reader, then reads `Shadow`, then dispatches vtable offset `+0xA0` while the image-name buffer is nonempty. Only after that call returns does it read `Rate`, `Start`, `End`, `LoopStart`, and `LoopEnd`. Evidence: Ghidra MCP `decompile_function(address=0x00427D00)`.
4. The `+0xA0` virtual is proven to be the loader at `0x00427B50`, not inferred from its label:
   - `vtable__AnimTypeClass-4` at `0x007E3604` contains `0x007FBBB0` (`read_memory`: `b0bb7f00`).
   - `COL+0x0C` at `0x007FBBBC` contains `0x00818330` (`read_memory`: `30838100`).
   - the TypeDescriptor at `0x00818330` contains `.?AVAnimTypeClass@@`.
   - vtable offset `+0xA0` at `0x007E36A8` contains `0x00427B50` (`read_memory`: `507b4200`).
   - Ghidra MCP `decompile_function(address=0x00427B50)` verifies the SHP-loading body.
5. With a non-null SHP pointer, the loader sets `+0x2A4=1`. If and only if `End==0`, it reads a **signed** 16-bit value from SHP header offset `+6`, sign-extends it into `End`, and divides by two only when `Shadow` is true.
6. If and only if `LoopEnd==0`, the same loader copies the then-current `End` into `LoopEnd`.
7. Independently, the loader writes `signed_short(SHP+6)/2` to `AnimType+0x298`; this unconditional half-count cache is a different field and is not what damage-fire creation reads.
8. The later `CCINIClass::ReadInt("End", current_End)` preserves the loaded value when the key is omitted, but an explicit key replaces it. The same later-override rule applies to `LoopEnd`.

### 3.2 Boundary cases

| Input state | Resolved behavior before damage-fire caller reads `End` | Evidence |
|---|---|---|
| SHP present, `End=` omitted, `Shadow=` omitted | loader changes zero to the full signed SHP frame count | `0x00427B50`, `0x00427D00` |
| SHP present, `Shadow=yes`, `End=` omitted | loader changes zero to signed SHP frame count divided by two, integer truncation toward zero | same |
| SHP present, explicit positive `End=N` | loader may first populate from SHP, then the later INI read replaces it with `N` | `0x00427D00` ordering |
| SHP present, explicit `End=0` | loader first populates, then the later INI read resets `End` to zero; damage-fire frame RNG is skipped | `0x00427D00`, `0x0043C0D0` |
| SHP present, explicit `End=-1` | later INI read sets `-1`; `AnimClass` construction reloads the signed header count, halves for `Shadow`, and mutates the shared type before the damage-fire caller reads it | `0x00421EA0`, `0x0043C0D0` |
| SHP missing, `End=` omitted | loader cannot populate; `End` remains zero and damage-fire frame RNG is skipped | null branch at `0x00427B50`; positive test at `0x0043C0D0` |
| SHP header count `>=0x8000` | signed-short load produces a negative `End`; damage-fire frame RNG is skipped unless a later explicit positive `End` overrides it | signed load at `0x00427B50`; signed `0 < End` at `0x0043C0D0` |
| `LoopEnd=0` explicitly | loader may first copy `End`, but the later INI read resets `LoopEnd` to zero | `0x00427D00` ordering |

### 3.3 Damage-fire RNG order

`BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` behaves as follows for this slice:

1. If `DamageFireTypes` count is zero, return with no RNG call.
2. Otherwise call scenario `RandomRanged(0, count-1)` before scanning the first slot. Stock count three means `RandomRanged(0,2)`.
3. Scan at most eight slots, using type-offset range `0x15D8` through `<0x1618` in increments of eight and building fire-slot pointers in increments of four.
4. Return immediately at the first sentinel offset pair or first occupied fire slot.
5. Allocate and construct an `AnimClass` with `delay=0`, `loop=1`, draw flags `0x600`, facing zero, and z adjustment argument zero.
6. Only after successful construction, store the returned pointer, calculate/write `Anim+0x100` z-adjust, and clamp positive z-adjust to zero.
7. Read `Anim+0xC8 -> AnimType+0x2C0`. If signed `End > 0`, call scenario `RandomRanged(0, End-1)` and store the result at `Anim+0xAC`; otherwise consume no frame-selection ranged call.
8. Advance and wrap the type index only after successful construction. A failed allocation/construction consumes neither frame RNG nor type advancement.

`RandomRanged @ 0x0065C7E0` sorts unequal bounds, treats both endpoints as inclusive, returns immediately without advancing RNG for equal bounds, and uses mask/rejection rather than modulo. Consequences for stock damage fires:

- the initial `0..2` type draw can consume more than one raw RNG word when the masked value is three;
- `FIRE01`/`FIRE03` use `0..29`, so masked values 30 and 31 are rejected and raw-word consumption is variable;
- `FIRE02` uses `0..63`, a power-of-two-width range, so one raw RNG word suffices for each successful slot's frame selection;
- the public semantic contract is one ranged call at each stated point, while raw RNG-word count depends on rejection.

### 3.4 Retail stock values

The retail probe loaded each name through the repository `AssetManager` from the configured install and read the SHP(TS) header fields directly at offsets `+2`, `+4`, and `+6`:

| Type / asset | Size | Raw SHP frames | Stock `Shadow` | Resolved stock `End` | Damage-fire RNG bounds | Source |
|---|---:|---:|---|---:|---|---|
| `FIRE01.SHP` | `42x80` | 30 | false (omitted) | 30 | inclusive `0..29` | `ra2.mix -> conquer.mix` |
| `FIRE02.SHP` | `42x70` | 64 | false (omitted) | 64 | inclusive `0..63` | `ra2.mix -> conquer.mix` |
| `FIRE03.SHP` | `30x58` | 30 | false (omitted) | 30 | inclusive `0..29` | `ra2.mix -> conquer.mix` |

## 4. INI Keys

| File / section | Key | Stock value | Binary effect |
|---|---|---|---|
| `ini/rulesmd.ini:519` `[General]` | `DamageFireTypes` | `FIRE01,FIRE02,FIRE03` | supplies the type array and count used by `0x0043C0D0` |
| `ini/artmd.ini:16018-16021` `[FIRE01]` | `Rate` | `450` | becomes `900/450 = 2` native logic frames |
| same | `LoopCount` | `-1` | read as signed integer; constructor later narrows/multiplies for runtime loop state |
| same | `StartSound` | `BuildingFireBig` | resolved before construction; audio behavior is outside this report |
| same | `End`, `Image`, `Shadow`, `RandomRate` | omitted | type-name image fallback; full 30-frame `End`; false `Shadow`; no constructor rate RNG |
| `ini/artmd.ini:16027-16030` `[FIRE02]` | same keys | `450`, `-1`, `BuildingFireBig`; others omitted | full 64-frame `End` |
| `ini/artmd.ini:16032-16035` `[FIRE03]` | same keys | `450`, `-1`, `BuildingFireMed`; others omitted | full 30-frame `End` |
| `ini/art.ini:11479-11492` | base fallback | same three definitions | `artmd.ini` has priority; base agrees for this stock slice |

The fresh `RandomRate` endpoints are both zero. `AnimClass` construction calls rate `RandomRanged` only when at least one endpoint is nonzero and the first endpoint is not greater than the second, so stock damage-fire construction adds no hidden rate-selection RNG call. Evidence: `0x00427530`, `0x00421EA0`.

## 5. Integration Points

- Rules initialization selects the three types through `[General] DamageFireTypes=`.
- `AnimTypeClass::ReadINI @ 0x00427D00` owns the active load/apply order; the helper at `0x00427B50` is proven active through the `AnimTypeClass` vtable and the constructor-populated image buffer.
- `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` constructs the animation before reading `End`, which is why the `End=-1` constructor fallback is visible to its frame-RNG decision.
- Damage-fire creation reads `End`, not `AnimType+0x298`, not a render-atlas frame count, and not a generic half-frame cache.
- This path is normal YR content, not dormant TS legacy: stock YR rules name all three types, stock art defines them, and stock retail archives contain their SHPs.

## 6. Current Rust Implementation Status

The current app-side damage-fire surface matches the high-level initial-type-then-per-slot RNG shape, but the frame-bound source is not native-equivalent:

- `src/rules/art_data.rs:278-279` parses omitted `LoopEnd` and `End` as zero without applying the SHP-loader mutation/order.
- `src/app_building_anim.rs:1228-1237` resolves SHP frame count only for `End==-1`; an omitted zero remains zero in this runtime helper.
- `src/app_building_anim.rs:1394` contains a regression test whose premise says omitted `End` must not use SHP frame count. That premise is contradicted by `0x00427B50` on the normal loaded-SHP path.
- `src/render/sprite_atlas.rs:833-849` unconditionally divides world-effect SHP counts by two before publishing `active_anim_frame_counts`. For the three stock files this publishes 15, 32, and 15 rather than the native `End` values 30, 64, and 30.
- `src/app_init.rs:737-742` copies those published values into `Simulation::effect_frame_counts`.
- `src/app_building_anim.rs:164-167` uses that map and forces a minimum of one for damage fires. This changes native missing/zero semantics and, for stock FIRE types, supplies half-sized frame-RNG bounds.
- `src/app_building_anim.rs:1228` and related runtime consumers already centralize part of `End` handling, so the future implementation should replace the incomplete resolution rule rather than add an unrelated second animation model.

This report does not modify those Rust files.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectTypeClass` name-to-image initialization | verified | `0x005F7090` decompile | none for claimed slice |
| `AnimTypeClass` constructor defaults | verified | `0x00427530` decompile | none |
| `AnimTypeClass` vtable identity and `+0xA0` slot | verified | reads at `0x007E3604`, `0x007FBBBC`, `0x00818330`, `0x007E36A8` | none |
| SHP loader non-null branch | verified | `0x00427B50` decompile and cold disassembly check | none |
| SHP loader null branch | verified | `0x00427B50` | none |
| `Shadow` before loader and `End` after loader ordering | verified | `0x00427D00` | none |
| explicit `End` values `>0`, `0`, and `-1` | verified | `0x00427D00`, `0x00421EA0`, `0x0043C0D0` | none |
| `LoopEnd` loader/default ordering | verified | `0x00427B50`, `0x00427D00` | none |
| damage-fire frame-bound field and call order | verified | `0x0043C0D0` decompile and cold disassembly check | none |
| ranged RNG inclusive/equal/rejection behavior | verified | `0x0065C7E0` decompile | none |
| stock art/rules activation | verified | `rulesmd.ini:519`; `artmd.ini:16018-16035`; base fallback | none |
| retail FIRE asset headers | verified | direct `AssetManager` lookup and SHP header read | none |
| current Rust producer/consumer chain | verified | file/line scan in section 6 | none |
| full animation advancement/draw lifecycle | deferred | out of claimed scope | separate AnimClass AI/draw investigation if needed |
| save/load and paused/replay behavior | deferred | the claimed mechanism runs at type-load and creation boundaries | separate lifecycle/persistence investigation if needed |

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-01 — What initializes fresh End, LoopEnd, Shadow, and RandomRate? -> 0, 0, false, and (0,0).` (evidence: `0x00427530`)
- `[RESOLVED] OQ-02 — Does an omitted Image prevent the loader from running? -> No; ObjectType construction copies the type name into the image buffer.` (evidence: `0x005F7090`)
- `[RESOLVED] OQ-03 — Is vtable +0xA0 really the AnimType SHP loader? -> Yes, proven through COL, TypeDescriptor, slot bytes, and function body.` (evidence: `0x007E3604`, `0x007FBBBC`, `0x00818330`, `0x007E36A8`, `0x00427B50`)
- `[RESOLVED] OQ-04 — What exact condition lets the loader write End? -> A non-null SHP pointer and signed End equal to zero.` (evidence: `0x00427B50`)
- `[RESOLVED] OQ-05 — What header field and signedness feed End? -> Signed 16-bit SHP header value at +6, sign-extended to int.` (evidence: `0x00427B50`)
- `[RESOLVED] OQ-06 — When is the value halved? -> Only when AnimType Shadow is true.` (evidence: `0x00427B50`)
- `[RESOLVED] OQ-07 — Does +0x298 carry the same contract as End? -> No; it is always raw header count divided by two and damage-fire creation does not read it.` (evidence: `0x00427B50`, `0x0043C0D0`)
- `[RESOLVED] OQ-08 — Does explicit End apply before or after asset-derived End? -> After; it can replace the loaded value.` (evidence: `0x00427D00`)
- `[RESOLVED] OQ-09 — What does explicit End=0 do? -> It resets the loaded value to zero and suppresses damage-fire frame RNG.` (evidence: `0x00427D00`, `0x0043C0D0`)
- `[RESOLVED] OQ-10 — What does explicit End=-1 do? -> AnimClass construction reloads from the SHP and mutates the type before the damage-fire caller reads it.` (evidence: `0x00421EA0`, `0x0043C0D0`)
- `[RESOLVED] OQ-11 — What happens when the SHP is missing and End is omitted? -> End remains zero; no per-slot frame ranged call occurs.` (evidence: `0x00427B50`, `0x0043C0D0`)
- `[RESOLVED] OQ-12 — Does stock construction consume hidden RandomRate RNG? -> No; both endpoints remain zero and fail the constructor's activation condition.` (evidence: `0x00427530`, `0x00421EA0`)
- `[RESOLVED] OQ-13 — What are the three retail frame counts? -> FIRE01=30, FIRE02=64, FIRE03=30.` (evidence: retail `ra2.mix -> conquer.mix` direct SHP header probe)
- `[RESOLVED] OQ-14 — What exact damage-fire frame bounds result? -> 0..29, 0..63, and 0..29 inclusive.` (evidence: retail counts plus `0x0043C0D0`)
- `[RESOLVED] OQ-15 — Is type selection random for every slot? -> No; one initial random index, then sequential wrap after each successful construction.` (evidence: `0x0043C0D0`)
- `[RESOLVED] OQ-16 — Can equal bounds consume a raw draw? -> No; RandomRanged returns immediately when bounds are equal.` (evidence: `0x0065C7E0`)
- `[RESOLVED] OQ-17 — Is raw RNG-word consumption fixed? -> Not for widths three or thirty because rejection can repeat; width sixty-four needs one word.` (evidence: `0x0065C7E0`)
- `[RESOLVED] OQ-18 — Does current Rust use the same resolved End? -> No; it publishes an unconditional half-count and separately leaves omitted End at zero.` (evidence: `src/render/sprite_atlas.rs:833-849`, `src/rules/art_data.rs:278-279`, `src/app_building_anim.rs:1228-1237`)
- `[DEFERRED] OQ-19 — Does the full later AnimClass AI/draw path expose additional End-dependent differences?` (category: `out-of-scope`; reason: the claimed slice ends after creation-time frame selection; next-step-if-pursued: run a bounded AnimClass AI/draw trace)
- `[DEFERRED] OQ-20 — How are these type/runtime values restored across save/load and replay?` (category: `out-of-scope`; reason: persistence does not decide the fresh-load/create mechanism established here; next-step-if-pursued: audit AnimType/AnimClass persistence together)

### Adversarial corner-case answers

1. **Explicit `End=0` despite a valid SHP?** The later INI read wins; no frame ranged call.
2. **Explicit `End=-1`?** The `AnimClass` constructor resolves and mutates the type before the caller tests it.
3. **Missing SHP?** Omitted `End` remains zero and suppresses frame RNG.
4. **Header count above signed-short range?** It becomes negative and suppresses frame RNG unless explicitly overridden.
5. **Construction failure in the middle of slot scanning?** No frame RNG and no type advance for that slot; scanning proceeds to the next offset/slot.

The final cold pass re-read `0x00427B50` and `0x0043C0D0`, plus their disassembly around the `+0x2C0` accesses. It added no new open question.

## 9. Visual/UI Composition Ledger

Omitted: this report establishes type-load and creation-time state/RNG semantics, not viewport composition or draw order.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Zero `End` is populated from the signed SHP header during normal art load, before explicit `End=` is read | `0x00427B50`, `0x00427D00` | missing | `src/rules/art_data.rs`; initialization metadata binding; shared AnimType runtime resolution | retain raw asset frame metadata but produce native-resolved `End` using loader-then-INI order | omitted `End` with 30-frame SHP resolves to 30; explicit `End=7` resolves to 7; explicit `End=0` resolves to 0 | do not model omitted zero as permanent zero; do not treat raw atlas count as already-resolved End |
| `Shadow` alone decides whether loader-derived `End` is halved | `0x00427530`, `0x00427B50`, `0x00427D00` | current world-effect scan halves unconditionally | `src/render/sprite_atlas.rs:833-849`; resolved metadata surface | preserve full raw SHP count and apply halving only through resolved AnimType `Shadow` semantics | same 30-frame fixture resolves to 30 with omitted/false Shadow and 15 with Shadow=yes | do not reuse the unconditional `frames/2` cache as `End` |
| Damage-fire creation uses resolved signed `End`, not a generic frame-count cache | `0x0043C0D0` | mismatch | future sim-owned damage-fire producer; current `src/app_building_anim.rs:159-168` bridge | call scenario ranged RNG only when resolved End is positive, with inclusive bounds `0..End-1` | stock seeded trace uses bounds 29, 63, 29 as types wrap | do not force `max(1)`; it invents a draw for native zero/missing cases |
| Stock FIRE01/02/03 End values are 30/64/30 | retail probe plus stock omitted Shadow/End | current published counts are 15/32/15 on the world-effect scan | asset metadata binding and damage-fire tests | bind the exact resolved values without hardcoding the names/counts | initialization assertion derives 30/64/30 from retail assets and merged art | do not encode these retail values as constants; derive them from assets and INI |
| Explicit `End=-1` resolves during `AnimClass` construction before caller inspection | `0x00421EA0`, `0x0043C0D0` | partially modeled only in one app helper | shared AnimType/AnimClass construction surface | preserve the mutation/order so later callers observe the resolved positive value | fixture with `End=-1`, 30 frames, Shadow=false consumes frame call with upper 29 | do not resolve only at draw time after the producer has already made its RNG decision |
| Stock RandomRate adds no hidden constructor RNG | `0x00427530`, `0x00421EA0`; stock INI omissions | no mismatch established for this narrow point | AnimClass constructor tests | keep RNG ledger free of a rate call when both endpoints are zero | seeded stock construction consumes initial type call and per-success frame calls only | do not call equal-zero RandomRanged merely because endpoints are equal |
| Rejection sampling controls raw-word count | `0x0065C7E0` | existing `SimRng` call shape needs direct verification when moved into sim | `src/sim/rng.rs`; damage-fire acceptance tests | compare both ranged results and final scenario RNG state | seeded multi-slot stock trace matches final RNG state across 0..2, 0..29, and 0..63 calls | do not replace with modulo or assume one raw word per ranged call |

### Stale Docs / Follow-up Docs

- `docs/research/ANIM_CLASS_DEEP_DIVE.md:242-255` documents only the `End==-1` constructor fallback. It is incomplete as a general statement of auto-detection. Replacement wording: **“Normal `AnimTypeClass` art loading also replaces `End==0` from SHP header `+6` before explicit `End=` is parsed; the later `End==-1` AnimClass-constructor branch is a separate fallback.”**
- `docs/plans/2026-07-18-scheduler-backed-animclass-damage-fire-design.md:113` should replace “positive frame count” with **“positive native-resolved `AnimType.End`.”**
- The same design's line 145 may keep frame counts as externally derived raw metadata, but must state that simulation consumes a resolved AnimType value after native loader/INI ordering; raw or unconditional half-count metadata is not itself the gameplay bound.
- The design's line 373 claim that no new Ghidra work was required was overtaken by the review contradiction. This report supplies the missing narrow investigation and resolves it.

## Sources

- Live Ghidra MCP, `gamemd.exe` image base `0x00400000`:
  - `decompile_function(address=0x005F7090)`
  - `decompile_function(address=0x00427530)`
  - `decompile_function(address=0x00427B50)`
  - `decompile_function(address=0x00427D00)`
  - `decompile_function(address=0x00421EA0)`
  - `decompile_function(address=0x0043C0D0)`
  - `decompile_function(address=0x0065C7E0)`
  - `disassemble_function(address=0x00427B50)` and `disassemble_function(address=0x0043C0D0)` cold spot-checks
  - `read_memory` at `0x007E3604`, `0x007FBBBC`, `0x00818330`, and `0x007E36A8`
- `ini/rulesmd.ini:519`, `ini/rulesmd.ini:2237-2239`
- `ini/artmd.ini:16018-16035`
- `ini/art.ini:11479-11492`
- Retail `ra2.mix -> conquer.mix`: direct header probe of `FIRE01.SHP`, `FIRE02.SHP`, `FIRE03.SHP`
- `docs/research/BUILDINGCLASS_DAMAGE_FIRE_SELECTOR_RNG_GHIDRA_REPORT.md`
- `docs/research/ANIM_CLASS_DEEP_DIVE.md`
- Current Rust surfaces named in section 6
