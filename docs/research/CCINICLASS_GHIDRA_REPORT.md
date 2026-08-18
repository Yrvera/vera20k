# CCINIClass — Ghidra Research Report

**Primary addresses:** ReadString `0x00528A10`, ReadInt `0x005276D0`, ReadBool `0x005295F0`, ReadDouble `0x005283D0`
**Confidence:** HIGH (decompiled 15+ functions, verified from binary)
**Active in YR:** Yes — the core INI system used by all game data loading

> **2026-07-13 active-binary correction:**
> `disassemble_function(address="0x005283d0", program="gamemd.exe")` proves
> `ReadDouble` parses `%f` to f32, then always performs an f32→f64 spill/reload
> (`FLD dword` at `0x0052855d`, `FSTP qword` at `0x00528569`). If the source
> contains `%`, it reloads that f64, multiplies by the f64 0.01 constant, and
> performs a second f64 spill at `0x0052857a..0x00528584`; the return reloads the
> stored f64. `decompile_function(address="0x00528a10", program="gamemd.exe")`
> confirms `ReadString` uses the caller's byte capacity with `_strncpy`, forces
> `dst[size-1]=0`, then trims and measures the bounded result. These stores and
> per-call capacities are parity behavior, not disposable C ABI details.

## 1. Overview

CCINIClass is the engine's INI file reader. It provides type-safe accessor methods
(ReadInt, ReadBool, ReadString, ReadDouble, Read3Int, ReadMinMax, ReadCLSID, etc.)
that look up values by section name and key name. Internally it uses **CRC-32 hashes**
of section/key names for fast lookup via **binary search** on sorted arrays, with a
**one-entry cache** for the most recently accessed section.

CCINIClass extends INIClass (which provides the raw section/entry storage and linked-list
infrastructure) and adds the higher-level typed read methods.

## 2. Class Hierarchy and Layout

```
INIClass (base)
└── CCINIClass (adds typed readers, overrides vtable)
```

### INIClass Layout (at 0x00535AA0 constructor)

`param_1` is `undefined4 *` (4-byte stride). All offsets below are byte offsets.

| Offset | Size | Type | Name | Init Value | Purpose |
|--------|------|------|------|-----------|---------|
| 0x00 | 4 | void* | vtable | vtable__INIClass | Virtual function table |
| 0x04 | 4 | int | cached_section_name_crc | 0 | CRC of last-looked-up section name |
| 0x08 | 4 | int | cached_section_ptr | 0 | Pointer to cached INISection node |
| 0x0C | 4 | void* | GenericList_vtable | vtable__GenericList | Linked list of sections |
| 0x10 | 4 | void* | GenericNode_head_vtable | vtable__GenericNode | List head sentinel |
| 0x14 | 4 | void* | head_next | 0 | Head→next pointer |
| 0x18 | 4 | void* | head_prev | 0 | Head→prev pointer |
| 0x1C | 4 | void* | GenericNode_tail_vtable | vtable__GenericNode | List tail sentinel |
| 0x20 | 4 | void* | tail_next | 0 | Tail→next pointer |
| 0x24 | 4 | void* | tail_prev | 0 | Tail→prev pointer |
| 0x28 | 4 | void* | sorted_sections_array | 0 | Pointer to sorted CRC→INISection array |
| 0x2C | 4 | int | section_count | 0 | Number of sections |
| 0x30 | 4 | int | sorted_sections_capacity | 0 | Allocated capacity of sorted array |
| 0x34 | 1 | bool | sorted_flag | false | Whether the sorted array is valid |
| 0x38 | 4 | void* | last_find_result | 0 | Cached result of last FindSection |
| 0x3C | 4 | void* | field_0x3C | 0 | Unknown (possibly entry iteration cursor) |

**Total INIClass base size: ~0x40 bytes (64 bytes)**

### CCINIClass Extensions

CCINIClass adds a vtable override and a `Digest` field:

| Offset | Size | Type | Name | Purpose |
|--------|------|------|------|---------|
| 0x00 | 4 | void* | vtable | vtable__CCINIClass (overrides INIClass) |
| 0x40 | 1 | bool | has_digest | Whether SHA/CRC digest is enabled |

**Total CCINIClass size: ~0x44 bytes (68 bytes)**

## 3. Core Lookup Mechanism — CRC-Based Binary Search

**This is the key architectural insight.** The engine does NOT do string comparisons
for section and key lookups. Instead:

1. **Section/key names are CRC-32 hashed** via `CRCEngine__AddData(string, strlen(string))`
2. Sections are stored in a **sorted array** of `{CRC, INISection*}` pairs (8 bytes each)
3. Lookups use **binary search** on the sorted CRC array
4. A **one-entry cache** at `this+0x04`/`this+0x08` remembers the last accessed section

### Lookup Algorithm (ReadInt as example, at `0x005276D0`)

```
ReadInt(this, section_name, key_name, default_value):
    // Step 1: Find the section
    if section_name == this.cached_section_name:
        section = this.cached_section_ptr          // Cache hit
    else:
        section_crc = CRC32(section_name)
        section = BinarySearch(sorted_sections, section_crc)
        if not found: return default_value
        this.cached_section_name = section_name     // Update cache
        this.cached_section_ptr = section

    // Step 2: Find the entry within the section
    key_crc = CRC32(key_name)
    entry = BinarySearch(section.sorted_entries, key_crc)
    if not found: return default_value

    // Step 3: Parse the value string
    value_string = entry.value_ptr + 0x10          // String at entry node + 0x10
    return parse(value_string)
```

### Binary Search (at `0x0052B620` / `0x0052B4F0`)

Both FindSection and FindEntry use identical binary search logic:

```
BinarySearch(sorted_array, target_crc):
    base = sorted_array.data
    count = sorted_array.count

    // Lazy sort: if not sorted, sort first
    if not sorted_array.is_sorted:
        qsort(data, count, 8, compare_func)
        sorted_array.is_sorted = true

    // Standard binary search on 8-byte records (CRC at offset 0, ptr at offset 4)
    while count > 0:
        mid = count / 2
        mid_crc = *(base + mid * 8)
        if target_crc < mid_crc:
            count = mid
        elif target_crc == mid_crc:
            return base + mid * 8    // Found!
        else:
            base = base + (mid + 1) * 8
            count = count - 1 - mid
    return NULL                      // Not found
```

**Key detail:** The arrays are sorted **lazily** — only on first access after modification.
The `sorted_flag` at offset +0x34 (sections) or +0x30+3 (entries) tracks this.

### INISection Internal Structure

Each section node (found via binary search) has its own sorted entry array:

| Offset | Size | Purpose |
|--------|------|---------|
| 0x00 | 4 | Section name CRC |
| 0x04 | 4 | Pointer to section data node |
| → +0x00 | — | GenericNode linked list pointers |
| → +0x28 | 4 | Sorted entry array pointer |
| → +0x2C | — | (aliased) |
| → +0x30 | 4 | Entry count |
| → +0x34 | 1 | Entry sorted flag |
| → +0x3C | 4 | Cached last-found entry pointer |

### INIEntry Internal Structure

Each entry node (found via binary search within a section):

| Offset | Size | Purpose |
|--------|------|---------|
| 0x00 | 4 | Entry name CRC |
| 0x04 | 4 | Pointer to entry data node |
| → +0x10 | 4 | Pointer to value string (char*) |

## 4. Read Method Signatures and Behavior

### ReadString (`0x00528A10`)

```c
int __thiscall ReadString(
    CCINIClass* this,       // this pointer
    char* section,          // Section name (e.g., "HTNK")
    char* key,              // Key name (e.g., "Strength")
    char* default_value,    // Returned if key not found
    char* buffer,           // Output buffer
    size_t buffer_size      // Buffer capacity
) → returns string length
```

**Behavior:**
- Returns 0 if buffer is NULL or buffer_size < 2 or section is NULL or key is NULL
- Copies found value (or default) into buffer via `strncpy`
- Always null-terminates: `buffer[buffer_size - 1] = '\0'`
- Calls `strtrim()` on the result — **trims leading chars <= 0x20 and trailing chars < 0x21**
- Returns the strlen of the final trimmed string

### ReadInt (`0x005276D0`)

```c
int __thiscall ReadInt(
    CCINIClass* this,
    char* section,
    char* key,
    int default_value
) → returns parsed int or default
```

**Parsing rules (in order of precedence):**
1. If value starts with `$` → parse as hex via sscanf with `"$%x"` format
2. If value ends with `h` (case-insensitive) → parse as hex via sscanf with `"%xh"` format
3. Otherwise → parse as decimal integer via `atoi()`
4. If section or key is NULL → return default immediately

**Hex support is notable** — the original game supports `$FF` and `FFh` hex notation.

### ReadBool (`0x005295F0`)

```c
bool __thiscall ReadBool(
    CCINIClass* this,
    char* section,
    char* key,
    bool default_value
) → returns parsed bool or default
```

**Parsing:** Checks the **first character** (uppercased) of the value string:
- `'1'`, `'T'`, `'Y'` → return **true**
- `'0'`, `'F'`, `'N'` → return **false**
- Anything else → return **default**

This means `yes`, `Yes`, `YES`, `true`, `True`, `1` all work for true.
And `no`, `No`, `false`, `False`, `0` all work for false.

### ReadDouble (`0x005283D0`)

```c
double __thiscall ReadDouble(
    CCINIClass* this,
    char* section,
    char* key,
    double default_value
) → returns the stored parsed f64 or default
```

**Parsing:**
1. Parse value as f32 via sscanf with `"%f"` format.
2. Load that f32 and spill/reload it through a local f64 unconditionally.
3. Check if the source contains byte `%` (the `0x2525` argument is truncated by
   `strchr` to `0x25`); if yes, multiply the reloaded f64 by **0.01** and
   spill/reload f64 again.
4. Return the stored f64.

**Percentage handling:** any source containing `%` takes the scale branch:
`"50%"` and `"50%%"` both parse the f32 prefix 50 and return 0.5; `"100%"` and
`"100%%"` return 1.0. `strchr` receives `0x2525` but searches its low byte
`0x25`, so the predicate is contains-any-percent, not a required `%%` suffix.

**The 0.01 constant** is stored as a double at `0x007E3808` = `0x3F847AE147AE147B` = 0.01.

### Read3Int (`0x00529CA0`)

```c
void __thiscall Read3Int(
    CCINIClass* this,
    int* output,            // 3-element int array
    char* section,
    char* key,
    int* defaults           // 3-element int array of defaults
)
```

**Parsing:** Uses sscanf with `"%d,%d,%d"` format string (at `0x008189B0`).
Copies all 3 defaults if key not found.

### ReadMinMax (`0x00529880`)

```c
void __thiscall ReadMinMax(
    CCINIClass* this,
    int* output,            // 2-element int array (min, max)
    char* section,
    char* key,
    int* defaults           // 2-element int array of defaults
)
```

**Parsing:** Uses sscanf with `"%d,%d"` format string (at `0x0081C000`).

### ReadCLSID (`0x00527920`)

```c
void __thiscall ReadCLSID(
    CCINIClass* this,
    CLSID* output,          // 16-byte COM GUID
    char* section,
    char* key,
    CLSID default_value     // Passed by value (4 DWORDs)
)
```

**Parsing:** Converts value string to wide char, calls `CLSIDFromString()`.
Buffer is 128 chars. Used for locomotor CLSIDs.

### ReadSoundList (`0x00525430`)

```c
DynamicVectorClass<int>* __thiscall ReadSoundList(
    CCINIClass* this,       // (implicit)
    ... section, key
) → returns vector of sound indices
```

**Parsing:**
1. Calls ReadString into a 128-byte buffer
2. Tokenizes with `strtok(buffer, ",")`
3. For each token: calls `VocClass__FindPtrByName()` → `VocClass__FindIndexByPtr()`
4. Stores sound indices in a DynamicVectorClass (initial capacity 10)

### ReadSpeedType (`0x00476FC0`)

Pattern: `ReadString → FromName()` enum conversion using string table lookup.

### ReadMovementZone (`0x00474E40`)

Pattern: Linear scan through a string pointer table at `0x0081BA88` (13 entries: Normal,
Destroyer, Crusher, etc.), using case-insensitive `_stricmp`.

**MovementZone enum values** (from string table at `0x0081BA88`, 13 entries, 0x0081BA88–0x0081BABB):
```
0: Normal, 1: Crusher, 2: Destroyer, 3: AmphibiousDestroyer,
4: AmphibiousCrusher, 5: Amphibious, 6: Subterranean,
7: Infantry, 8: InfantryDestroyer, 9: Fly, 10: Water,
11: WaterBeach, 12: CrusherAll
```
Returns -1 if string not found.

### ReadAction (`0x00474EE0`)

Pattern: Linear scan through string pointer table at `0x007E4C50` (73 entries,
0x007E4C50–0x007E4D73). Returns 0 if not found (not -1).

### ReadLayer (`0x00477050`)

Pattern: `ReadString → Layer_From_Name()` enum conversion.

## 5. The strtrim Function (`0x00727CF0`)

Called on every ReadString result. Trims **both** ends:

```
strtrim(char* str):
    // 1. Skip leading whitespace (chars <= 0x20, i.e., space and control chars)
    find first char > 0x20
    if found offset > 0: memmove string left

    // 2. Trim trailing whitespace (chars < 0x21, i.e., space and control chars)
    from end of string, null-terminate at first non-whitespace + 1
```

**Notable:** Uses `<= 0x20` for leading trim and `< 0x21` for trailing — both effectively
trim space (0x20) and all control characters. Tabs, newlines, carriage returns all trimmed.

## 6. Section Name Caching

The CCINIClass maintains a **one-element cache** for section lookups:

- `this+0x04` = pointer to the **raw section name string** of the last successful lookup
- `this+0x08` = pointer to the **section data node** from the last lookup

On each Read* call:
- If `section_name == this+0x04` (pointer equality, NOT string compare!) → use cached section
- Otherwise → compute CRC, binary search, update cache

**Important:** The cache comparison is a **pointer comparison**, not a string comparison.
This means repeated calls with the same string literal will cache-hit, but calls with
different char* pointers to identical string content will NOT cache-hit (they'll still
work via binary search, just slower).

## 7. Integration Points

### Who creates CCINIClass instances?

| Usage | Address | Purpose |
|-------|---------|---------|
| Base constructor | `0x00535B30` | Allocates and initializes empty INIClass with GenericList/GenericNode infrastructure |
| Scenario load | `0x00599650` | Full scenario CCINIClass with lighting, ambient, theater |
| Random map gen | `0x005981F0` | RMG scenario constructor |

### Who calls the Read methods?

Every TypeClass has a `ReadINI` virtual method (vtable offset 0x64 typically) that calls
these CCINIClass methods. Key callers documented in READINI_FIELD_MAPS.md:

| TypeClass | ReadINI Address | Approx Keys |
|-----------|----------------|-------------|
| TechnoTypeClass | `0x00712170` | 200+ |
| WeaponTypeClass | `0x00772080` | 63 |
| WarheadTypeClass | `0x0075D590` | 43 |
| BulletTypeClass | `0x0046BEE0` | 42 |
| InfantryTypeClass | `0x005240A0` | 36+42 seqs |
| BuildingTypeClass | `0x006F32D0` | 200+ |
| AnimTypeClass | `0x00427D00` | ~55 |
| SuperWeaponTypeClass | `0x006CEA20` | 22 |

### Global INI instances

- `rules.ini` / `rulesmd.ini` → loaded into a global CCINIClass, merged (YR overrides RA2)
- `art.ini` / `artmd.ini` → loaded into a separate global CCINIClass
- Map files (`.map`) → each map has its own CCINIClass for `[Header]`, `[Basic]`, etc.

## 8. Implications for Our Rust Implementation

Our Rust INI parser at `src/rules/ini_parser.rs` uses `HashMap<String, IniSection>` with
case-insensitive string keys (lowercased). This is functionally equivalent to the
CRC-based lookup — both achieve O(1)-ish amortized lookup, both are case-insensitive
(CRC hashing treats same-case strings identically; our Rust code lowercases on insert).

**Key behavioral matches we should verify:**
1. **Hex integer support** — `$FF` and `FFh` notation. Our `get_i32()` may not handle this.
2. **Bool parsing** — first-char check only (T/Y/1 = true, F/N/0 = false). Our Rust code
   matches this with `get_bool()`.
3. **Percentage handling** — `%%` suffix divides by 100. Our `get_percent()` handles `%`.
4. **strtrim** — trims chars <= 0x20 from both ends. Our Rust `trim()` should match.
5. **ReadString buffer sizes** — original uses explicit byte capacities (128
   typical, 32 for enums). Rust must reproduce the relevant caller capacity,
   forced-NUL truncation, and post-truncation trim before downstream parsing;
   accepting the unbounded source is DRIFT for over-length values.
6. **Sound list tokenization** — splits on `,` via strtok. Matches our `get_list()`.
7. **CLSID/GUID parsing** — for locomotors. We may need this eventually.

**Potential divergences to investigate:**
- CRC hash collisions: two different section/key names could theoretically collide. The
  original engine would silently return the wrong value. Our HashMap approach is immune.
- Inline comment stripping: the original engine's `strtrim` does NOT strip inline `;`
  comments — that's handled elsewhere (during file load, not during Read*). We should
  verify our parser strips comments at load time, not at read time.

## 9. Open Questions

1. **INI file loading/parsing** (MEDIUM): How does the raw INI text get parsed into the
   section/entry data structures? The `INIClass::Load()` or equivalent function was not
   decompiled in this session. This would reveal comment handling, multi-line support,
   and other parsing quirks.

2. **CRC collision handling** (LOW): The binary search returns the first match. If two
   keys hash to the same CRC, the second one is effectively invisible. How common is this
   in practice? Probably never occurs with standard RA2/YR INI keys, but worth noting.

3. **INI write methods** (LOW): CCINIClass presumably has Write* methods for scenario
   editing. Not investigated as they're not relevant for our read-only usage.

4. **Section iteration** (MEDIUM): How does `GetEntryCount` / `GetEntryName` work for
   type registry iteration (e.g., `[InfantryTypes]` with `0=E1`, `1=GI`)? The linked
   list at offsets 0x0C–0x24 likely provides ordered iteration.

5. **MD file merging** (MEDIUM): How exactly does rulesmd.ini merge with rules.ini?
   Is it a simple section+key override, or more complex? Not investigated in binary.

## Sources

### Ghidra Functions Decompiled
- `0x00528A10` — CCINIClass::ReadString (full section/key lookup with strtrim)
- `0x005276D0` — CCINIClass::ReadInt (hex `$xx`/`xxh` support, atoi fallback)
- `0x005295F0` — CCINIClass::ReadBool (first-char T/Y/1 = true, F/N/0 = false)
- `0x005283D0` — CCINIClass::ReadDouble (`%f` parse, `%%` → multiply by 0.01)
- `0x00529CA0` — CCINIClass::Read3Int (`%d,%d,%d` sscanf)
- `0x00529880` — CCINIClass::ReadMinMax (`%d,%d` sscanf)
- `0x00527920` — CCINIClass::ReadCLSID (CLSIDFromString, 128-char buffer)
- `0x00525430` — CCINIClass::ReadSoundList (strtok on `,`, VocClass lookup)
- `0x00476FC0` — CCINIClass::ReadSpeedType (ReadString → enum conversion)
- `0x00474E40` — CCINIClass::ReadMovementZone (linear scan through 13-entry string table)
- `0x00474EE0` — CCINIClass::ReadAction (linear scan through 73-entry string table)
- `0x00477050` — CCINIClass::ReadLayer (ReadString → Layer_From_Name)
- `0x0052B620` — INIClass::FindSection (binary search on sorted CRC array)
- `0x0052B4F0` — INIClass::FindEntry (binary search on sorted CRC array)
- `0x0052B390` — INIClass::FindSectionCached (cache check + binary search)
- `0x00535B30` — CCINIClass::Constructor (initializes GenericList/Node infrastructure)
- `0x00535AA0` — INIClass::Constructor (base class init)
- `0x005256F0` — INIClass::Destructor (cleans up linked list, frees sorted array)
- `0x00727CF0` — strtrim (trim chars <= 0x20 from both ends)

### Format Strings Verified
- `0x00825BB8` = `"$%x"` (hex with `$` prefix)
- `0x00825BB4` = `"%xh"` (hex with `h` suffix)
- `0x00825BD8` = `"%f"` (float)
- `0x008189B0` = `"%d,%d,%d"` (3-int tuple)
- `0x0081C000` = `"%d,%d"` (min/max pair)

### Constants Verified
- `0x007E3808` = double 0.01 (percentage multiplier)
- `0x0081BA88` = MovementZone string table (13 entries)
- `0x007E4C50` = Action enum string table (73 entries)

### Ghidra Labels Created
- `INIClass__FindSection_BinarySearch` at `0x0052B620`
- `INIClass__FindEntry_BinarySearch` at `0x0052B4F0`
- `INIClass__FindSectionCached_BinarySearch` at `0x0052B390`
- `INIClass__Constructor_Init` at `0x00535AA0`
