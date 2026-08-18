# Gate D1 — Verses f64 read (ApplyWarheadDamage) + Verses INI parse bit-parity

**Verdict: CLOSED.** Both halves resolved by live disassembly/decompile this run (2026-06-04).
**Status:** RESOLUTION (read-only RE). No Rust written.
**Scope:** exactly how `Verses[armor]` is *read* inside the damage kernel and *parsed* from INI, to the bit.
**Authority:** binary → Ghidra. Every load-bearing fact cites the MCP call made this run.
**Parent study:** `DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (treated as prior; one of its constants is **corrected** below — see §D1c).

> **2026-07-13 verified correction:** every 128-lepton claim in the original
> gate was caused by decoding `0x43800000` incorrectly. The value is 256.0f,
> and `Apply_area_damage` uses the same constant. See
> `DAMAGE_KERNEL_CONSTANTS_REVERIFICATION_2026-07-13.md` for fresh raw-memory
> and instruction evidence. The corrected text below supersedes the original
> 2026-06-04 interpretation.

> **2026-07-13 parser correction:**
> `disassemble_function(address="0x0075d590", program="gamemd.exe")` shows
> `ReadString(..., size=0x80, default=0x00847c40)` at
> `0x0075ddcc..0x0075dde6`, followed by a fixed 11-iteration loop at
> `0x0075de0c..0x0075de58`. `read_memory(address="0x00847c40", length=128,
> program="gamemd.exe")` decodes the default as eleven `100%%` tokens, so a
> missing key parses that fallback through the same atoi×0.01 stores. A present
> empty/whitespace-only value returns length zero after ReadString trim and skips
> the loop, retaining constructor ones. `decompile_function(address="0x00528a10",
> program="gamemd.exe")` proves the 0x80-byte copy, forced NUL at byte 127, trim,
> and returned length occur before tokenization. Native `strtok` collapses empty
> fields; a present nonempty list that yields fewer than 11 tokens reaches
> `strchr(NULL, '%')`. `disassemble_function(address="0x007caf30",
> program="gamemd.exe")` proves `strchr` dereferences that null input, so the
> native behavior is a fault—not identity-fill or a normal parse error. This
> correction supersedes conflicting default/short-list prose below.

---

## D1(a) — Verses read inside ApplyWarheadDamage @ 0x00489180

Verified via `decompile_function 0x00489180` + `disassemble_function 0x00489180` this run.

### Signature & frame
`__fastcall(uint damage /*ECX*/, WarheadType* wh /*EDX*/, ??? /*[ESP+0x18] in body*/, int armorIndex /*[ESP+0x18]→EDX scale index*/)`. The caller-supplied stack arg used as the Verses index is at `[ESP+0x18]` inside the body (`MOV EDX,[ESP+0x18]` @ `0x00489229`), which becomes the `EDX*8` scale in the Verses load. (The decompiler renames params; the body is authoritative.)

### Armor-index source — TechnoTypeClass+0x9c (VERIFIED at the call site)
`decompile_function 0x005f5390` (ObjectClass::ReceiveDamage) shows the kernel invoked with the target's armor field:
```
iVar4 = (**(this->vtable + 0x88))();      // GetType() -> TechnoTypeClass*
iVar4 = FUN_00489180(*(undefined4*)(iVar4 + 0x9c), warhead);  // arg = TechnoTypeClass+0x9c (Armor)
```
So the Verses index `armorIndex` = **TechnoTypeClass + 0x9c** (the unit's `Armor=` class index 0..10). Confirmed `get_function_callers 0x00489180` = {ObjectClass__ReceiveDamage 0x005f5390, TechnoClass__ReceiveDamage 0x00701900, FUN_006fdb80}.

### The exact Verses load — double[11] at warhead+0xA0, stride 8 (ASM-VERIFIED)
From `disassemble_function 0x00489180`:
```
00489229  MOV EDX,[ESP+0x18]                 ; EDX = armorIndex
00489239  FILD dword ptr [ESP+0x1c]          ; promote zero-floored falloff int -> x87
0048923d  FMUL double ptr [EDI + EDX*0x8 + 0xa0]  ; * Verses[armorIndex]  (EDI = wh)
00489244  CALL 0x007c5f00                    ; Math__ftol  (truncate toward zero)
```
`Verses` is a **`double`** array (`FMUL double ptr`, 8-byte stride, base +0xA0). It is read **directly** as f64 and multiplied in 80-bit x87 against the falloff int — **no f32 narrowing** anywhere on the Verses value. The product is then truncated by `Math__ftol @ 0x007c5f00` (`decompile_function 0x007c5f00`: ROUND under the truncate control word — the documented `_ftol2` truncate-toward-zero).

### Full distance-falloff formula (f64/x87, ASM-VERIFIED)
The three `Math__ftol @ 0x007c5f00` calls in order:
```
; --- cellSpreadLeptons = ftol(CellSpread * 256.0) ---
004891d8  FLD   float ptr [EDI+0x124]        ; CellSpread (float, cells)
004891de  FMUL  float ptr [0x007e2224]       ; * 256.0f   <-- see §D1c CORRECTION
004891e4  CALL  0x007c5f00                    ; ftol  -> cellSpreadLeptons (ECX/[ESP+0x10])
; --- falloff branch guard: (damage*PercentAtMax != damage) && (cellSpreadLeptons != 0) ---
004891c6  FILD  dword ptr [ESP+0xc]          ; (float)damage
004891ca  FST   float ptr [ESP+0x8]          ; save damage-as-float
004891ce  FMUL  float ptr [EDI+0x12c]        ; * PercentAtMax (float, +0x12C)
004891ed  FCOMP float ptr [ESP+0x8]          ; compare damage*PAM vs damage
004891f9  TEST  AH,0x40                       ; equal? -> skip (flat damage)
004891fe  TEST  ECX,ECX / JZ                  ; cellSpreadLeptons==0 -> skip
; --- if branch taken: lerp in x87, then ftol ---
00489206  FLD   float ptr [ESP+0x8]          ; damage
0048920a  FSUB  float ptr [ESP+0xc]          ; damage - damage*PAM = damage*(1-PAM)
0048920e  SUB   ECX,EAX                       ; (cellSpreadLeptons - distance)
00489214  FIMUL dword ptr [ESP+0x1c]         ; * (cellSpreadLeptons - distance)
00489218  FIDIV dword ptr [ESP+0x10]         ; / cellSpreadLeptons
0048921c  FADD  float ptr [ESP+0xc]          ; + damage*PAM
00489220  CALL  0x007c5f00                    ; ftol  -> falloff int (ESI)
; --- zero-floor, then Verses multiply (above) ---
00489227..00489235  ECX = (falloff<=0 ? 0 : falloff)
```
Algebraically:
```
cellSpreadLeptons = ftol(CellSpread * 256.0)
if (damage*PercentAtMax != damage) AND cellSpreadLeptons != 0:
    falloff = ftol( damage*PAM + (damage - damage*PAM) * (cellSpreadLeptons - distance) / cellSpreadLeptons )
            = ftol( damage * lerp(PAM, 1.0, (cellSpreadLeptons - distance)/cellSpreadLeptons) )
else:
    falloff = damage                      # PAM==1.0 (float-exact) OR CellSpread==0
falloff = max(falloff, 0)
scaled  = ftol( (double)falloff * Verses[armorIndex] )   # Verses = f64, +0xA0+idx*8
return (scaled >= Rules.MaxDamage[+0x16C8]) ? MaxDamage : scaled
```
So the canonical contract is **`ftol( ftol(lerp) * Verses_f64 )`** — two truncations on the damage value, plus one on `cellSpreadLeptons`. The lerp intermediates run in 80-bit x87; `PercentAtMax` and `CellSpread` are read as **f32** (`FLD float`), but `Verses` stays **f64**. The healing path (`(int)damage < 0`) returns `(7 < armorIndex) - 1 & damage` — **bypasses falloff and Verses entirely** (`0x004891ad`).

### MaxDamage cap (ASM-VERIFIED)
`0x00489249 MOV ECX,[0x008871e0]; MOV ECX,[ECX+0x16c8]; CMP EAX,ECX; JL` → cap kernel output at `Rules+0x16C8` (constructor fallback 1000; stock YR runtime 10000).

---

## D1(b) — Verses INI parse → double[11]

Verified via `disassemble_function 0x0075de31` (WarheadTypeClass__ReadINI) — the Verses loop is at `0x0075de06..0x0075de58`:
```
0075de06  LEA EBX,[ESI+0xa0]                 ; EBX -> Verses base (WarheadType+0xA0)
0075de0c  MOV EBP,0xb                         ; 11 entries
0075de11  PUSH 0x25 / PUSH EDI / CALL 0x007caf30   ; strchr(token, '%')
0075de1c  TEST EAX,EAX / JZ 0x0075de39        ; '%' NOT found -> strtod branch
; --- '%'-suffixed branch ---
0075de1e  PUSH EDI / CALL 0x007c9bfd          ; atoi(token)
0075de2d  FILD dword ptr [ESP+0x10]           ; (double)(int)atoi
0075de31  FMUL double ptr [0x007e3808]        ; * 0.01
0075de37  JMP 0x0075de41
; --- no-'%' branch ---
0075de39  CALL 0x007c9d66                      ; strtod-family parse -> f64
0075de41..
0075de48  FSTP double ptr [EBX]               ; store as DOUBLE
0075de52  ADD EBX,0x8                          ; stride 8
0075de55  DEC EBP / JNZ 0x0075de11             ; loop x11
```

### Identities (VERIFIED this run)
- `0x007caf30` = **`strchr`** (`decompile_function 0x007caf30`).
- `0x007c9bfd` = **`atoi` wrapper** (`decompile_function 0x007c9bfd` → calls `CRT__atoi`).
- `0x007c9d66` = **`strtod` wrapper** (`decompile_function 0x007c9d66`: skips leading ws, then `FUN_007d151e` = CRT strtod core, returns the parsed `double`).
- `0x007e3808` = **0.01** (`read_memory 0x007e3808` = `7b 14 ae 47 e1 7a 84 3f` = IEEE-754 double `0.01`). This is the trailing-`%` ⇒ `×0.01` constant.

### Parse rule (bit-exact)
For **each** of the 11 Verses tokens:
- **Token contains `'%'`** → value = `(double)atoi(token) * 0.01`. **`atoi` truncates to the integer part** before scaling — so `"100%"`→1.0, `"50%"`→0.5, `"0.5%"`→`atoi("0.5")`=0→**0.0**, `"50.9%"`→`atoi`=50→0.5. A negative like `"-50%"`→`atoi`=-50→-0.5 (heal). Trailing-`%` ⇒ `×0.01` **does** apply to Verses specifically.
- **Token has no `'%'`** → value = `strtod(token)` (full f64). `"0.5"`→0.5, `"100"`→100.0, `"-0.5"`→-0.5, `"50"`→50.0.
- Result stored as `double` at `warhead+0xA0 + i*8`.

**Adversarial-decimal consequence (DRIFT-relevant):** the `%` path goes through **integer `atoi`**, the bare path through **`strtod`**. They are NOT the same function on fractional input. `"50.5%"` parses to **0.5** (atoi drops `.5`), whereas `"0.505"` parses to **0.505**. Any Rust parse must branch on `%`-presence and use integer-truncating atoi for the `%` case — not float-parse-then-multiply.

### Defaults and bounded tokenization

The constructor initializes all 11 entries to 1.0. A missing key makes
`ReadString` copy the eleven-`100%%` fallback at `0x00847c40`, after which the
normal fixed 11-token parse loop runs. A present value that trims to length zero
skips the loop and keeps constructor ones. Every other value is first truncated
to the 127-byte payload of the 0x80-byte buffer, forced-NUL, and trimmed; only
then does native `strtok(",")` tokenize it. The loop performs exactly 11 stores
without guarding token exhaustion. Fewer than 11 resulting tokens therefore
fault in `strchr(NULL, '%')`; they do not retain default tail values.

---

## §D1c — 2026-07-13 correction (leptons-per-cell constant)

`DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` D3/D5/§2b state `cellSpreadLeptons = ftol(CellSpread * 256.0)` and label `0x007e2224 = 256.0f`. **That parent value is correct.** `read_memory 0x007e2224` returned `00 00 80 43 00 00 00 00`; the f32 at `0x007e2224` is `0x43800000` = **256.0f**. The original version of this gate incorrectly called that bit pattern 128.0f. The kernel computes `cellSpreadLeptons = ftol(CellSpread * 256.0)` (`disassemble_function 0x00489180` @ `0x004891de FMUL float ptr [0x007e2224]`).

**Impact:** the falloff denominator reaches its `PercentAtMax` edge at `CellSpread * 256` leptons. `Apply_area_damage @ 0x00489280` also executes `FLD [wh+0x124]; FMUL [0x007e2224]; Math__ftol`, closing the former constant-reconciliation thread. Exact per-target distance and traversal behavior remain separate AoE-dispatch verification work.

---

## Rust handoff — exact f64 contract

**Parse (`rules/ini_value.rs::parse_verses`, consumes the slice-1 typed accessor):**
- Model the 0x80-byte ReadString buffer, forced NUL, and trim before native
  `strtok` tokenization. Missing parses the eleven-`100%%` fallback; present
  empty retains constructor ones; a present nonempty short token list faults.
- Per token, branch on byte `'%'` (0x25) presence:
  - has `%`: `value = (atoi(token) as f64) * 0.01` — integer-truncating atoi (drop any fractional part of the token *before* ×0.01), sign-preserving.
  - no `%`: `value = strtod(token)` (full f64 parse).
- Store as **f64** (or `SimFixed` with ≥ f64-equivalent precision), 11 entries. Do NOT store as `u8` percent — that loses fractional `strtod` values and sub-1% (current Rust `Vec<u8>` is DRIFT, study R-6/P5).

**Read/apply (`sim/combat/damage` kernel, D1(a)):**
- Index Verses by **TechnoTypeClass.Armor (0..10)**, the target type's armor class.
- `cellSpreadLeptons = ftol(CellSpread * 256)` — use the same lepton scale on the incoming distance.
- Falloff branch guard is float-exact `damage*PercentAtMax != damage` (skip falloff when `PercentAtMax == 1.0` or `CellSpread == 0`).
- `falloff = ftol(damage * lerp(PAM,1.0,(csLeptons-dist)/csLeptons))`, then `max(0)`.
- `scaled = ftol((falloff as f64) * verses_f64[armor])` — Verses kept f64 through the multiply; truncate-toward-zero (`ftol` = sim_to_i32 trunc) on the product.
- Cap at `Rules.MaxDamage` (constructor fallback 1000; stock YR runtime 10000).
- Healing (`damage < 0`) bypasses falloff+Verses; armor index ≥ 8 cannot heal.

---

## What current Rust gets wrong (D1-scoped)

1. **Verses stored as `u8` percent (0..200)** — loses gamemd's f64 precision (any `Verses=` with a non-integer `%` or a bare decimal like `0.005`). DRIFT. → migrate to f64/`SimFixed`.
2. **Single-multiply order** `base*verses*falloff/10000` instead of `ftol(ftol(lerp)*Verses)` — last-digit DRIFT (the two interior truncations are skipped).
3. **Parse path** likely float-parses then multiplies uniformly — must split `%` (integer atoi ×0.01) vs bare (strtod), or `"50.5%"` and `"0.505"` will both round wrong.
4. **(2026-07-13 correction) leptons-per-cell is 256** in both this kernel and the AoE radius conversion. Any 128 implementation is drift.

---

## Verification ledger (this run)
- `decompile_function 0x00489180` + `disassemble_function 0x00489180` — kernel body, three ftol, Verses `FMUL double [EDI+EDX*8+0xA0]`, MaxDamage cap, `*256.0f`.
- `read_memory 0x007e2224` (len 8) = `0x43800000` = **256.0f**.
- `decompile_function 0x005f5390` — armor index = TechnoTypeClass+0x9c at call site.
- `get_function_callers 0x00489180` — 3 callers (ObjectClass/TechnoClass ReceiveDamage, FUN_006fdb80).
- `disassemble_function 0x0075de31` (WarheadTypeClass__ReadINI) — Verses loop @ 0x0075de06: base +0xA0, 11×, stride 8, strchr('%') → atoi×0.01 | strtod, FSTP double.
- `decompile_function 0x007caf30` = strchr; `0x007c9bfd` = atoi wrapper; `0x007c9d66` = strtod wrapper.
- `read_memory 0x007e3808` (len 8) = `0x3f847ae147ae147b` = **0.01** (double).
- `decompile_function 0x007c5f00` — Math__ftol (truncate-toward-zero ROUND).

## Former open thread — constant reconciliation CLOSED 2026-07-13
- `Apply_area_damage @ 0x00489280` reads the same `0x007e2224 = 256.0f` constant for its CellSpread radius conversion. Full dispatcher order and distance-special-case parity remain separate work.
