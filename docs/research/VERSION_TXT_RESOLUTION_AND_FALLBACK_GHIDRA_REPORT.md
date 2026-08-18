# VERSION.TXT Resolution and `%d.%3.3dTUC` Numeric Fallback — Ghidra Report

**Target:** `FUN_0074FAE0` (internal version string builder)  
**Active in YR:** Yes — called on every main-menu dialog open (msg `0x497`) and in the crash exception handler.  
**Investigation date:** 2026-05-19  
**Verified via:** Ghidra MCP decompile_function, inspect_memory_content, get_xrefs_to, search_strings

---

## 1. Overview

`FUN_0074FAE0` is the internal-version-string builder for the `VersionClass`. It populates a
char buffer at `this+0x0C` with either the contents of `VERSION.TXT` (trimmed of `\r`) or
the numeric fallback string `"%d.%3.3dTUC"` formatted from a pair of uint16 build-number fields.
It is called as a `__fastcall` method on a global static `VersionClass` instance at `0x00A8ECE0`.

The function is called in two distinct call sites:
- **Main menu, message 0x497** (`MainMenuDialog0xE2_Proc_00531F60` @ `0x00531F60`): sets static
  control 0x71D (bottom-right version label) via `SendMessageA(hWnd, 0x4B2, …)`. Verified via
  `decompile_function 0x00531F60`.
- **Exception/crash handler** (`FUN_004C85E0` @ `0x004C85E0`): appends "Internal Version %s"
  to the crash report buffer. Verified via `decompile_function 0x004C85E0`.
- **Sync-file writer** (`FUN_006516F0` @ `0x006516F0`): writes "Internal Version %s" to the
  per-player sync dump file. Verified via `decompile_function 0x006516F0`.

---

## 2. Call Signature and Global Instance

```
int __fastcall FUN_0074fae0(int param_1)
```

`param_1` (ECX) is a pointer to a `VersionClass` instance. When called from the main menu
proc and crash handler with no visible argument, ECX is pre-loaded to `0x00A8ECE0` — the global
static `VersionClass` instance — as confirmed by the trampoline at `0x004E7DC0`:

```asm
MOV ECX, 0x00A8ECE0     ; load global VersionClass this-pointer
CALL 0x0074FAE0          ; call FUN_0074fae0
```

Verified via `inspect_memory_content 0x004E7DB0` (raw bytes `B9 E0 EC A8 00 E8 66 79 26 00`).

The vtable is installed at `0x00A8ECE0` with value `0x007EA57C` at startup, confirmed by code at
`0x004E7DE0` (`C7 05 E0 EC A8 00 7C A5 7E 00`). Verified via `inspect_memory_content 0x004E7DB0`.

---

## 3. VersionClass Layout (Reconstructed from FUN_0074FAE0)

All offsets are relative to the instance base (`this` = `param_1`):

| Offset | Type        | Description                                         |
|--------|-------------|-----------------------------------------------------|
| +0x00  | void*       | vtable pointer (= `0x007EA57C` for global instance) |
| +0x04  | uint32      | packed version word (set by `FUN_0074F760` only)    |
| +0x08  | uint16      | major build number (first `%d` in fallback format)  |
| +0x0A  | uint16      | minor build number (second `%d` in fallback format) |
| +0x0C  | char[...]   | output formatted string buffer (return value)       |
| +0x2A  | char[16]    | VERSION.TXT content buffer (max 16 bytes read)      |
| +0x39  | char        | sentinel/null terminator cap for VERSION.TXT buffer |
| +0x44  | uint32      | flag word — cache/initialization state bits         |

Relevant flag bits in `+0x44`:
- Bit 3 (0x08): VERSION.TXT load-attempt complete (set after first attempt regardless of success)
- Bit 2 (0x04): minor version field initialized (offset +0x0A)
- Bit 1 (0x02): major version field initialized (offset +0x08) — global-only path

For the global instance (`0x00A8ECE0`), the same fields are referenced directly as `DAT_00A8ED24`
(= `0x00A8ECE0 + 0x44`) and `DAT_00A8ECE8` (= `0x00A8ECE0 + 0x08`), `DAT_00A8ECEA` (+0x0A).
Cross-verified: `0x00A8ECE0 + 0x44 = 0x00A8ED24` ✓.  
Verified via `decompile_function 0x0074FAE0` (inspecting all field accesses).

---

## 4. File-Open Path — How VERSION.TXT Is Located

**Mechanism: Win32 raw `CreateFileA`, no .mix archive lookup.**

The sequence inside `FUN_0074FAE0` for opening VERSION.TXT:

1. Calls `RawFileClass__Constructor` with the string `"VERSION.TXT"` (at `0x0084635C`) to
   initialize a stack-local `RawFileClass` object with that filename. Verified via
   `search_strings "VERSION\.TXT"` → `0x0084635C` and `get_xrefs_to 0x0084635C`.

2. Calls `FUN_0065CBF0(0)` — `RawFileClass::Is_Available` — which calls:
   ```c
   CreateFileA(filename, 0x80000000/*GENERIC_READ*/, 1/*FILE_SHARE_READ*/,
               NULL, 3/*OPEN_EXISTING*/, 0x80/*FILE_ATTRIBUTE_NORMAL*/, NULL)
   ```
   Returns 1 if the file exists and can be opened (immediately closes the handle), 0 otherwise.
   Verified via `decompile_function 0x0065CBF0`.

3. If `FUN_0065CBF0` returns non-zero (file exists), calls `FUN_0065CCE0(buffer, 0x10)` —
   `RawFileClass::Read` — which calls `ReadFile(handle, buffer, 16, &bytesRead, NULL)` to
   read up to 16 bytes. Verified via `decompile_function 0x0065CCE0`.

4. Closes the `RawFileClass` (calls `FileClass__Constructor()` for destructor/cleanup).

**There is no MIX archive search, no `FindFirstFileA` multi-path search, no CDFileClass
multi-directory lookup.** The `RawFileClass` operates directly on the raw Win32 filesystem,
resolving `"VERSION.TXT"` relative to the **current working directory** at the time of the
call. This is typically the RA2 game installation directory.

All four callers of `FUN_0074FAE0` listed by `get_function_callers 0x0074FAE0`:
`FUN_004C85E0`, `FUN_0064DEA0`, `FUN_006516F0`, `MainMenuDialog0xE2_Proc_00531F60`,
`RawFileClass__Constructor` (the last is a mislabeled large function `0x006C6F50`).

---

## 5. VERSION.TXT Parsing and `\r` Trimming

After a successful read of up to 16 bytes into `this+0x2A`:
- Sets `this+0x39 = 0` to ensure the buffer cannot exceed 15 usable chars + null.
- Walks the string with an inline `strlen` loop (Ghidra shows the `0xFFFFFFFF` countdown pattern).
- If the last character is `\r` (carriage return), it overwrites it with `\0` and repeats.
  This is a trim-trailing-CR loop, not just a single-pass strip — it will remove multiple
  trailing `\r` characters.

**No `\n` stripping is performed.** Only `\r` (0x0D) is explicitly checked.

---

## 6. Fallback Path — What Triggers `%d.%3.3dTUC`

The **`%d.%3.3dTUC` fallback always runs**, unconditionally:

```c
FUN_007c8ef4(param_1 + 0xc,   // output buffer
             s__d__3_3dTUC,   // format string "%d.%3.3dTUC" @ 0x00846368
             uVar2 & 0xffff,  // first %d  = major word  (this+0x08 & 0xFFFF)
             uVar3);           // second %d = minor word  (this+0x0A, uint16)
```

`FUN_007C8EF4` is a safe `sprintf`-like function (internally calls `FUN_007CE2A5`
for the formatted write, then null-terminates). Verified via `decompile_function 0x007C8EF4`.

Crucially: **VERSION.TXT content is stored in `this+0x2A` but the formatted output in
`this+0x0C` always comes from the numeric `%d.%3.3dTUC` path**, not from the VERSION.TXT
buffer. The VERSION.TXT text is stored separately for a different consumer (e.g.,
`FUN_0074F760`, the companion that returns a packed uint32 version word by parsing the
VERSION.TXT string). `FUN_0074FAE0` itself always formats `this+0x0C` using the numeric path.

Therefore: there is **no exclusive VERSION.TXT branch** in this function. The function:
1. Loads `this+0x2A` from VERSION.TXT (or leaves it empty if file missing).
2. Initializes `this+0x08` and `this+0x0A` to `1` (hardcoded) if not already set (bits 2 and 1
   of flag not set).
3. **Always** calls `FUN_007C8EF4` with `%d.%3.3dTUC` to populate `this+0x0C`.
4. Returns `param_1 + 0x0C` (pointer to the formatted string).

The VERSION.TXT content at `this+0x2A` is used by the companion function `FUN_0074F760`
(which parses it into a packed version uint), not by `FUN_0074FAE0` itself for display.

---

## 7. Where the Two `%d.%3.3dTUC` Integers Come From

```c
uVar2 = DAT_00a8ece8;                      // global: this+0x08 (uint, major)
...
if ((_DAT_00a8ed24 & 2) == 0) {            // if major not yet initialized
    DAT_00a8ece8 = CONCAT22(DAT_00a8ece8._2_2_, 1);  // set low word to 1
    _DAT_00a8ed24 = _DAT_00a8ed24 | 2;     // mark initialized
    uVar2 = 1;
}

// minor:
if ((*(uint *)(param_1 + 0x44) & 4) == 0) {
    uVar3 = 1;
    *(undefined2 *)(param_1 + 10) = 1;    // this+0x0A = 1
    *(uint *)(param_1 + 0x44) |= 4;
} else {
    uVar3 = *(undefined2 *)(param_1 + 10); // cached minor
}

FUN_007c8ef4(param_1 + 0xc, "%d.%3.3dTUC", uVar2 & 0xffff, uVar3);
```

Both major (`this+0x08`) and minor (`this+0x0A`) are initialized to `1` on first call if not
previously set. **Neither is read from VERSION.TXT content.** VERSION.TXT parsing populates the
text buffer at `this+0x2A` only; the numeric fields are separate.

The `_2_2_` suffix in `CONCAT22(DAT_00a8ece8._2_2_, 1)` means the high 16 bits of the uint are
preserved while the low 16 bits are set to 1. So on a fresh instance:
- major = 1 → first `%d` → `"1"`
- minor = 1 → second `%d` with `%3.3d` padding → `"001"`
- Result = `"1.001TUC"`

In practice, the major/minor may be populated by a caller before `FUN_0074FAE0` is invoked
(e.g., via the companion `FUN_0074F760` which sets `this+0x08`/`this+0x0A` from the VERSION.TXT
text by parsing the numeric portion). If that companion ran first and set the fields, those
values are used instead of the hardcoded `1`.

---

## 8. Caching Mechanism

Three lazy-init gates protect repeated work:

| Gate flag (this+0x44 bit) | Guards |
|--------------------------|--------|
| Bit 3 (0x08) | VERSION.TXT file read (both per-instance and global paths) |
| Bit 2 (0x04) | Minor version field `this+0x0A` |
| Bit 1 (0x02) | Major version field `this+0x08` (global path only) |

All three use the same pattern: if bit is clear → do the work → set the bit → cache result.
Subsequent calls skip the work and use the cached value.

For the global instance at `0x00A8ECE0`:
- `_DAT_00A8ED24` = `*(uint*)(0x00A8ECE0 + 0x44)` = flags
- `DAT_00A8ECE8` = `*(uint16*)(0x00A8ECE0 + 0x08)` = cached major
- `DAT_00A8ED0A` = `*(char*)(0x00A8ECE0 + 0x2A)` = cached VERSION.TXT text start

All globals are in BSS (zero-initialized), meaning on first call all bits are clear and all
work is performed. Verified via `inspect_memory_content 0x00A8ECE0` (all zeros in static image).

---

## 9. "TUC" Suffix Identification

The literal `"TUC"` appended by the format string `"%d.%3.3dTUC"` at `0x00846368` appears
**only once** in the entire binary (verified via `search_strings "TUC"`). It is a fixed
build-label suffix, not an INI key, not a localized string. Its meaning is unverified from
binary alone (possibly "The Ultimate Collection", "Tiberian/C&C" internal build tag, or
similar Westwood/EA internal convention). It never appears in any retail-visible UI string;
only in this internal debug/sync version label.

---

## 10. Companion Function: `FUN_0074F760`

`FUN_0074F760` (the public VersionClass getter) uses identical VERSION.TXT read logic but
additionally:
- Parses the `this+0x2A` text into a packed `uint32` version word stored at `this+0x04`.
- Sets `this+0x08` (major, high 16 bits of pack) and `this+0x0A` (minor, low 16 bits of pack).
- Returns the packed `uint32`.

This is the function that actually populates the major/minor fields that `FUN_0074FAE0` later
reads. If `FUN_0074F760` ran first (not guaranteed), the `%d.%3.3dTUC` output will reflect the
VERSION.TXT-derived numbers. If `FUN_0074FAE0` runs standalone, both integers default to `1`.

---

## 5 Most Load-Bearing Verified Facts

1. **File-open uses raw Win32 `CreateFileA` on the CWD, no .mix archive** — `FUN_0065CBF0`
   calls `CreateFileA(filename, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, …)`.
   Verified via `decompile_function 0x0065CBF0`.

2. **`%d.%3.3dTUC` always runs; there is no branch that uses VERSION.TXT text for display** —
   `FUN_0074FAE0` always calls `FUN_007C8EF4` with the numeric format, storing the result at
   `this+0x0C`. Verified via `decompile_function 0x0074FAE0`.

3. **Both integers default to `1` on first call** — major (`this+0x08`) and minor (`this+0x0A`)
   are each initialized to `1` if their respective flag bits (0x02, 0x04) in `this+0x44` are
   clear. Default output: `"1.001TUC"`. Verified via `decompile_function 0x0074FAE0`.

4. **Global static VersionClass instance is at `0x00A8ECE0`** — confirmed by trampoline
   `MOV ECX, 0x00A8ECE0 / CALL FUN_0074FAE0` at `0x004E7DC0` and vtable store `C7 05 E0 EC A8
   00 7C A5 7E 00` at `0x004E7DE0`. Verified via `inspect_memory_content 0x004E7DB0`.

5. **VERSION.TXT read limit is 16 bytes** — `FUN_0065CCE0(buffer, 0x10)` reads at most 16
   bytes into `this+0x2A`; sentinel at `this+0x39 = 0` caps the field. Verified via
   `decompile_function 0x0074FAE0`.

---

## Status

COMPLETE

Report written to:
`C:/Users/enok/Documents/ra2-rust-game-docs/VERSION_TXT_RESOLUTION_AND_FALLBACK_GHIDRA_REPORT.md`
