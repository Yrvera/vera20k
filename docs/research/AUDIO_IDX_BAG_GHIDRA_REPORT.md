# Audio.idx / Audio.bag File Format — Ghidra Report

Reverse-engineered from `gamemd.exe` via Ghidra MCP. All findings verified from
decompiled code with address citations.

## 1. Audio System Initialization (0x00406b10)

The audio system is initialized in `FUN_00406b10` (audio system init). The sequence:

1. Initialize DirectSound driver (`FUN_00402940`, `FUN_00402af0`)
2. Set up default format: 22050 Hz, stereo, 16-bit PCM (`0x00406be7..0x00406c07`)
3. Create sound driver via `FUN_00402c70` → stored in `DAT_0087e728`
4. Allocate 16 DirectSound buffers via `FUN_00403530` (return value compared with 0x10)
5. Open the MIX archive containing audio data:
   - Try `AUDIOMD.MIX` first (string at `0x00816280`)
   - Fall back to `AUDIO.MIX` (string at `0x00816274`)
   - The MIX is opened via `CDFileClass__Constructor` at `0x005b3c20` (MIXFileClass ctor)
   - Stored in `DAT_0087e734`
6. Create AudioIndex from base name `"audio"` (string at `0x0081626c`):
   ```asm
   00406ce1: XOR EDX, EDX          ; param2 = NULL (no directory path)
   00406ce3: MOV ECX, 0x81626c     ; param1 = "audio"
   00406ce8: CALL 0x004011c0       ; AudioIndex constructor
   ```
   Stored in `DAT_0087e724`
7. Initialize event system via `FUN_00403ed0`

**Key globals:**
- `DAT_0087e724` — AudioIndex object pointer (the main idx/bag handler)
- `DAT_0087e728` — Sound driver object
- `DAT_0087e734` — MIX file class (AUDIOMD.MIX or AUDIO.MIX)
- `DAT_0087e2a0` — Audio enabled flag
- `DAT_0087e294` — DirectSound initialized flag


## 2. AudioIndex Constructor (0x004011c0)

**Signature:** `AudioIndex* __fastcall AudioIndex__Constructor(char* base_name, char* directory_path)`

The constructor:
1. Allocates `0x124` (292) bytes for the AudioIndex object, zero-fills it
2. Takes the `base_name` (e.g., `"audio"`), strips any file extension
3. Appends `.idx` (string at `0x00815e80`) → `"audio.idx"`
4. Opens `audio.idx` via CCFileClass (through the MIX file system)
5. Strips extension again, appends `.bag` (string at `0x00815e78`) → `"audio.bag"`
6. Creates a persistent CCFileClass for `audio.bag` via `operator_new(0x6c)` — stored at
   `AudioIndex[0x10C]` (kept open for random-access reads during gameplay)
7. Reads the 12-byte idx header
8. Reads all entries into a heap-allocated array
9. Sorts entries by name (for binary search)
10. If `directory_path` is provided, stores it and checks if it's a valid directory
    (for loose .wav file fallback)

**Critical: no "audio.idx" or "audio.bag" string literals exist in the binary.**
The filenames are constructed at runtime by concatenating the base name with `.idx`/`.bag`
extension strings.

### String data addresses
| Address      | String  |
|-------------|---------|
| `0x00815e74` | `\`     |
| `0x00815e78` | `.bag`  |
| `0x00815e80` | `.idx`  |
| `0x00815e88` | `Invalid` |
| `0x00815e90` | `.wav`  |
| `0x0081626c` | `audio` |
| `0x00816274` | `AUDIO.MIX` |
| `0x00816280` | `AUDIOMD.MIX` |

### AudioIndex object layout (0x124 = 292 bytes)
| Offset | Type       | Name              | Notes |
|--------|------------|-------------------|-------|
| 0x000  | `void*`    | entries           | Pointer to array of 0x24-byte entries |
| 0x004  | `int`      | entry_count       | Number of entries |
| 0x008  | `char[256]`| directory_path    | Optional loose file directory |
| 0x10C  | `void*`    | bag_file          | CCFileClass for the .bag file (persistent) |
| 0x110  | `void*`    | loose_file        | CCFileClass for a loose .wav file (temp) |
| 0x114  | `int`      | has_directory     | 1 if directory_path is a valid directory |
| 0x118  | `void*`    | active_file       | Currently active file handle for reading |
| 0x11C  | `int`      | remaining_bytes   | Bytes remaining to read from current sample |


## 3. IDX File Header Format (12 bytes)

Read at `0x00401316` via `FUN_00473b10(buffer, 0xc)`:

```
Offset  Size  Name     Notes
0x00    4     magic    Not validated by the engine (read but never checked)
0x04    4     version  Compared against 1 at 0x00401367
0x08    4     count    Number of index entries
```

**Assembly proof (header field access):**
```asm
00401367: CMP dword ptr [ESP + 0x18], 0x1   ; version == 1 ?
0040132B: MOV EDX, dword ptr [ESP + 0x1c]   ; count
0040132F: MOV dword ptr [EBP + 0x4], EDX    ; store count in AudioIndex[4]
```

The first 4 bytes (`magic` / `[ESP + 0x14]`) are read as part of the 12-byte block but
never accessed or validated. Based on Westwood convention, this is likely the total size
of the accompanying .bag file, but the engine ignores it completely.


## 4. IDX Entry Format (0x24 = 36 bytes per entry)

### Version 1 (on-disk: 0x20 = 32 bytes)
When `version == 1`, entries are read one at a time, 0x20 (32) bytes each.
After reading, the engine zeroes the dword at offset +0x20:

```asm
00401384: PUSH 0x20                          ; read 32 bytes per entry
00401386: PUSH EAX                           ; destination = entry base
0040138B: CALL 0x00473b10                    ; CCFileClass::Read
00401390: CMP EAX, 0x20                     ; verify 32 bytes read
0040139D: MOV dword ptr [ESI + ECX + 0x20], EBX  ; zero out +0x20 (EBX = 0)
```

### Version 2+ (on-disk: 0x24 = 36 bytes)
When `version != 1`, all entries are read in a single bulk read:

```asm
004014E5: LEA EDX, [EAX + EAX*0x8]          ; EDX = count * 9
004014EB: SHL EDX, 0x2                      ; EDX = count * 36
004014EF: PUSH EDX                          ; total bytes
004014F0: PUSH EAX                          ; destination
004014F5: CALL 0x00473b10                    ; CCFileClass::Read
004014FE: CMP EAX, ECX                      ; verify all bytes read
```

### In-memory entry layout (always 0x24 = 36 bytes)

| Offset | Size | Name        | Verified from                    |
|--------|------|-------------|----------------------------------|
| 0x00   | 16   | name        | Binary search comparison target  |
| 0x10   | 4    | offset      | `FUN_004016f0`: seek target in .bag |
| 0x14   | 4    | size        | `FUN_004016f0`: bytes to read    |
| 0x18   | 4    | sample_rate | `FUN_00401640`: `[ECX + 0x18]`   |
| 0x1C   | 4    | flags       | `FUN_00401640`: `[ECX + 0x1C]`   |
| 0x20   | 4    | chunk_size  | `FUN_00401640`: `[ECX + 0x20]`   |

**The name field is 16 bytes** (offsets 0x00–0x0F), holding a null-terminated ASCII string.
Maximum name length is 15 characters + null terminator. This is verified by the fact
that the `offset` field starts at exactly +0x10.

**Proof of field usage in FUN_00401640 (0x00401640):**
```asm
00401645: LEA ECX, [ECX + EAX*0x4]       ; ECX = entry_base + index * 36
00401652: MOV EDX, [ECX + 0x18]          ; sample_rate
00401658: MOV EDX, [ECX + 0x1c]          ; flags
00401668: MOV EDX, [ECX + 0x20]          ; chunk_size
```

**Proof of field usage in FUN_004016f0 (0x004016f0):**
```c
pcVar1 = *param_1 + param_2 * 0x24;     // entry = entries + index * 36
param_1[0x47] = *(pcVar1 + 0x14);       // AudioIndex.remaining_bytes = entry.size
seek(param_1[0x43], *(pcVar1 + 0x10));   // seek bag_file to entry.offset
```

### Flag bits (at entry offset +0x1C)

Verified from `FUN_00401640` disassembly:

| Bit | Mask | Meaning                              | Proof |
|-----|------|--------------------------------------|-------|
| 0   | 0x01 | Stereo (2 channels); unset = mono (1) | `0x0040165B: AND DL,0x1` → channels = flag_bit0 + 1 |
| 2   | 0x04 | 16-bit samples; unset = 8-bit        | `0x00401691: AND CL,0x4` → bits = flag_bit2 + 1 |
| 3   | 0x08 | IMA ADPCM compressed                 | `0x00401671: TEST DL,0x8` → compression=1, bits=2 |
| 1   | 0x02 | Unknown (set by WAV fallback, unused by idx reader) | Written in WAV fallback code at `0x004016f0` |

When bit 3 (IMA ADPCM) is set, the engine forces bits_per_sample to 2 (16-bit output)
regardless of bit 2, since IMA ADPCM always decodes to 16-bit PCM.

### The chunk_size field (+0x20)

In version 1 idx files, this field does not exist on disk and is zeroed in memory.
In version 2 idx files, it is read from disk.

In `FUN_00401640`, it is stored into the audio format struct at offset +0x18
(`param_3[6]`). In the WAV file parser (`FUN_00408610` at `0x00408610`), the equivalent
field is `wSamplesPerBlock` from the WAV fmt chunk header (bytes 12-13 of the fmt chunk
data, for IMA ADPCM format 0x11). This is the number of decoded PCM samples per ADPCM
block.

When this field is 0 (version 1 or uncompressed PCM), the engine does not use it.


## 5. Sorting and Binary Search

### Sorting (after loading)

Entries are sorted immediately after loading, at `0x004013AB`:
```asm
004013B1: PUSH 0x7c8d20     ; comparison function (_stricmp variant)
004013B6: PUSH 0x24          ; element size = 36 bytes
004013B8: PUSH ECX           ; count
004013B9: PUSH EDX           ; array base pointer
004013BA: CALL 0x007c8b48    ; qsort
```

`FUN_007c8b48` at `0x007c8b48` is the C runtime `qsort` implementation (iterative
quicksort with insertion sort for small partitions). The comparison function is the
same one used for binary search.

### Binary search (FUN_004015c0)

**Signature:** `int __fastcall AudioIndex__FindSample(AudioIndex* this, char* name)`

```asm
004015C3: PUSH 0x7c8d20     ; comparison function
004015C8: PUSH 0x24          ; element size = 36
004015CA: MOV EAX, [ESI+4]  ; count
004015CD: MOV ECX, [ESI]    ; entries base
004015D1: PUSH EDX           ; search key (sample name)
004015D2: CALL 0x007c8e25    ; bsearch
```

Returns the index (0-based) of the found entry, or `0xFFFFFFFF` (-1) if not found.

**Index calculation (division by 36):**
```asm
004015E2: MOV EAX, 0x38e38e39   ; magic multiplier for /36
004015E7: SUB ECX, EDX          ; byte offset from base
004015EA: MUL ECX               ; multiply by magic number
004015EC: MOV EAX, EDX          ; take high 32 bits
004015EE: SHR EAX, 0x3          ; shift right by 3 → index = offset / 36
```

### Comparison function (FUN_007c8d20 — _stricmp)

**Address:** `0x007c8d20`

This is a **case-insensitive** string comparison. The simple code path (non-multibyte
locale, which is the default):

```asm
007c8d40: MOV AL, [ESI]       ; byte from entry name (param_2)
007c8d42: INC ESI
007c8d43: MOV AH, [EDI]       ; byte from search key (param_1)
007c8d45: INC EDI
007c8d46: CMP AH, AL
007c8d48: JZ 0x007c8d3c       ; if equal, check for null terminator

; Convert both bytes to lowercase for comparison:
007c8d4a: SUB AL, 0x41        ; AL -= 'A'
007c8d4c: CMP AL, 0x1a        ; if AL < 26 (was uppercase)
007c8d4e: SBB CL, CL          ; CL = 0xFF if uppercase, 0 otherwise
007c8d50: AND CL, 0x20         ; CL = 0x20 if uppercase, 0 otherwise
007c8d53: ADD AL, CL           ; add 0x20 (lowercase offset) if was uppercase
007c8d55: ADD AL, 0x41         ; add 'A' back → now lowercase
```

Both bytes are converted to lowercase before comparison. The function compares byte-by-
byte until a null terminator or a mismatch is found. Returns 0 for match, negative if
param_1 < param_2, positive if param_1 > param_2.

**Iteration limit:** The outer loop counter starts at 0xFF (255), decremented each
iteration. This means comparison stops after at most 255 characters (far more than the
16-byte name field allows).

**Conclusion:** Sample name lookup is **case-insensitive**.


## 6. Sample Loading / Playback Path

### Opening a sample (FUN_004016f0)

**Address:** `0x004016f0`

Called with `(AudioIndex*, sample_index)`. Flow:

1. Compute entry pointer: `entry = entries + sample_index * 0x24`
2. Set `AudioIndex.active_file = AudioIndex.bag_file` (offset 0x118 = offset 0x10C)
3. Set `AudioIndex.remaining_bytes = entry.size` (from `entry + 0x14`)
4. Seek bag file to `entry.offset` (from `entry + 0x10`) via vtable call `Seek(offset, SEEK_SET)`
5. If seek position doesn't match expected offset, or size is 0, return failure
6. **Loose file fallback:** If `AudioIndex.has_directory` is set, try to find a `.wav` file
   on disk at `directory_path + entry_name + ".wav"`. If found, parse the WAV header
   (`FUN_00408610`), update the entry's flags/sample_rate from the WAV header, and use
   the loose file instead of the bag file.

### Reading sample data (FUN_004018c0)

**Address:** `0x004018c0`

```c
int AudioIndex__Read(AudioIndex* this, void* buffer, int requested_bytes) {
    if (this->active_file == NULL) return 0;
    if (this->remaining_bytes < requested_bytes)
        requested_bytes = this->remaining_bytes;
    if (requested_bytes > 0)
        bytes_read = this->active_file->Read(buffer, requested_bytes);
    this->remaining_bytes -= bytes_read;
    return bytes_read;
}
```

### Audio format info (FUN_00401640)

**Address:** `0x00401640`

Populates an audio format descriptor struct from an idx entry:

```c
void AudioIndex__GetFormat(AudioIndex* this, int index, AudioFormat* out) {
    entry = this->entries + index * 0x24;
    out->format_tag   = 4;                                // always 4
    out->sample_rate  = entry->sample_rate;               // from +0x18
    out->num_channels = (entry->flags & 1) ? 2 : 1;      // bit 0 = stereo
    out->chunk_size   = entry->chunk_size;                // from +0x20

    if (entry->flags & 8) {                               // bit 3 = IMA ADPCM
        out->compression = 1;                             // IMA ADPCM
        out->bits_per_sample = 2;                         // 16-bit output
    } else {
        out->compression = 0;                             // raw PCM
        out->bits_per_sample = (entry->flags & 4) ? 2 : 1; // bit 2 = 16-bit
    }
}
```


## 7. Audio Data Format in .bag

The .bag file contains raw audio sample data at the offsets specified by the .idx entries.

- **Uncompressed (flags bit 3 = 0):** Raw PCM data, either 8-bit unsigned or 16-bit
  signed, mono or stereo, at the specified sample rate. Read directly from the bag file
  at the given offset for the given size.

- **IMA ADPCM compressed (flags bit 3 = 1):** Standard IMA ADPCM encoded data. The engine
  contains a standard IMA ADPCM decoder at `FUN_0040acd0` (`0x0040acd0`) with:
  - Step size table at `0x00816558` (89 entries: 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19,
    21, 23, 25, 28, 31, ... 32767)
  - Index adjustment table at `0x00816518` (16 entries: -1,-1,-1,-1, 2,4,6,8 repeated)
  - Decodes 4-bit nibbles to 16-bit signed PCM samples
  - `chunk_size` field (entry +0x20) specifies samples per ADPCM block


## 8. VocClass::ReadINI — Sound Tokenizer (FUN_00750440)

**Address:** `0x00750440`

This function reads a VocClass entry from `sound(md).ini`. For the `Sounds=` key:

```c
CCINIClass__ReadString(section, "Sounds", "", buffer, 0x800);
token = strtok(buffer, " \t\n");    // delimiter at 0x00846570
while (token != NULL) {
    FUN_004064a0(this, token);       // add sample by name
    token = strtok(NULL, " \t\n");
}
```

**Tokenizer delimiter string** at `0x00846570`:
```
0x20 = space
0x09 = tab
0x0a = newline
0x00 = null terminator
```

**Other INI keys parsed by VocClass::ReadINI:**
- `Sounds` — whitespace-separated sample names (with $/#prefix stripping)
- `Volume` — double, default from `0x008464b4`
- `VShift` — int, default 0
- `MinVolume` — double, default from `0x008464b8`
- `Priority` — string, default "NORMAL"
- `Attack` — int, default 0
- `Decay` — int, default 0
- `Control` — whitespace-separated tokens
- `Limit` — int, default from `DAT_008464c4`
- `Range` — int, default from `DAT_008464c0`
- `Delay` — comma/whitespace-separated pair (min, max)
- `FShift` — comma/whitespace-separated pair

All tokenized fields (`Sounds`, `Control`, `Delay`, `FShift`) use the same delimiter
string `" \t\n"` via `strtok`.


## 9. $ and # Prefix Stripping (FUN_004064a0)

**Address:** `0x004064a0`

```c
int VocClass__AddSample(VocClass* this, char* name) {
    if (this->sample_count == 0x20) return 0;   // max 32 samples per VocClass
    if (DAT_0087e2a0 == 0 || DAT_0087e294 == 0) return 1;  // audio disabled

    // Strip leading $ and # characters
    while (*name == '$' || *name == '#') {
        name++;
    }

    int index = AudioIndex__FindSample(audio_index, name);
    if (index != -1) {
        this->sample_indices[this->sample_count] = index;
        this->sample_count++;
        return 1;
    }

    Debug_Printf("Missing sample %s\n", name);  // string at 0x00816218
    this->field_0xc = 0;
    return 1;
}
```

**Stripping behavior (assembly proof at 0x004064BC):**
```asm
004064bc: MOV AL, [EDX]         ; load current character
004064be: CMP AL, 0x24          ; compare with '$'
004064c0: JZ 0x004064c8         ; if '$', skip
004064c2: CMP AL, 0x23          ; compare with '#'
004064c4: JNZ 0x004064cc        ; if not '#', stop stripping
004064c6: JMP 0x004064c8
004064c8: INC EDX               ; advance past prefix character
004064c9: JMP 0x004064bc        ; loop
```

The loop strips **all leading `$` and `#` characters** (not just the first one). The
stripping is a prefix-only operation — characters in the middle or end of the name are
not affected.

**VocClass struct (partial):**
- `+0x0C`: unknown flag (cleared on missing sample)
- `+0xB4`: `int sample_indices[32]` — array of AudioIndex entry indices
- `+0x134`: `int sample_count` — number of samples (max 32 = 0x20)


## 10. Summary: Complete Data Flow

```
INI file
  → "Sounds=<names>"
  → strtok by " \t\n"
  → strip leading $/# from each name
  → binary search (case-insensitive) in sorted audio.idx entries
  → store entry index in VocClass.sample_indices[]

Playback:
  → pick sample index from VocClass
  → AudioIndex__OpenSample(index)
     → entry = entries[index]
     → seek bag_file to entry.offset (+0x10)
     → set remaining = entry.size (+0x14)
     → (or try loose .wav file fallback)
  → AudioIndex__GetFormat(index)
     → read sample_rate (+0x18), flags (+0x1C), chunk_size (+0x20)
  → AudioIndex__Read(buffer, bytes)
     → read from bag_file, decrement remaining
  → if IMA ADPCM: decode via FUN_0040acd0
  → feed PCM to DirectSound buffer
```


## 11. File Format Quick Reference

### audio.idx

```
Header (12 bytes):
  uint32 magic;         // unused by engine, likely bag file size
  uint32 version;       // 1 = old format (32-byte entries), 2 = new (36-byte)
  uint32 entry_count;

Entry[entry_count] (36 bytes each in v2, 32 bytes in v1):
  char   name[16];      // null-terminated, max 15 chars
  uint32 offset;        // byte offset into audio.bag
  uint32 size;          // byte count in audio.bag
  uint32 sample_rate;   // Hz (e.g., 22050)
  uint32 flags;         // bit 0=stereo, bit 2=16-bit, bit 3=IMA ADPCM
  uint32 chunk_size;    // IMA ADPCM samples-per-block (v2 only, 0 in v1)
```

### audio.bag

Raw concatenated audio sample data. Each sample is located at the offset and
size specified by its corresponding .idx entry. No additional headers or framing.

### Flag bits detail
```
Bit 0 (0x01): Stereo       → 2 channels if set, 1 channel if unset
Bit 1 (0x02): (internal)   → set by WAV fallback, not read from idx
Bit 2 (0x04): 16-bit       → 16-bit signed PCM if set, 8-bit unsigned if unset
Bit 3 (0x08): IMA ADPCM    → compressed; overrides bit 2 (always 16-bit output)
```

### Location
Both `audio.idx` and `audio.bag` reside inside `AUDIOMD.MIX` (YR) or `AUDIO.MIX`
(base RA2), which are top-level MIX archives in the game directory. The engine opens
whichever MIX exists (preferring `AUDIOMD.MIX`), then searches for `audio.idx` and
`audio.bag` through the standard MIX file system.


## Appendix: Key Addresses

| Address    | Function / Data                              |
|-----------|----------------------------------------------|
| 0x004011c0 | AudioIndex::Constructor                     |
| 0x004015c0 | AudioIndex::FindSample (binary search)      |
| 0x00401640 | AudioIndex::GetFormat                       |
| 0x004016f0 | AudioIndex::OpenSample                      |
| 0x004018c0 | AudioIndex::Read                            |
| 0x00401580 | AudioIndex::Destructor                      |
| 0x00401c00 | SampleTracker::LoadSample                   |
| 0x004064a0 | VocClass::AddSample ($ / # stripping)       |
| 0x00406b10 | Audio system init                           |
| 0x00406d40 | Audio system shutdown                       |
| 0x00408610 | WAV header parser (RIFF/WAVE/fmt/data)      |
| 0x0040acd0 | IMA ADPCM decode (single nibble)            |
| 0x00750440 | VocClass::ReadINI                           |
| 0x007c8b48 | qsort                                       |
| 0x007c8d20 | _stricmp (case-insensitive compare)         |
| 0x007c8e25 | bsearch                                     |
| 0x007514d0 | VocClass::FindByName                        |
| 0x00815e78 | ".bag" string                                |
| 0x00815e80 | ".idx" string                                |
| 0x0081626c | "audio" string                               |
| 0x00816274 | "AUDIO.MIX" string                           |
| 0x00816280 | "AUDIOMD.MIX" string                         |
| 0x00816518 | IMA ADPCM index adjustment table (16 int32s) |
| 0x00816558 | IMA ADPCM step size table (89 int32s)        |
| 0x0087e724 | Global AudioIndex pointer                    |
| 0x0087e728 | Global sound driver pointer                  |
| 0x0087e734 | Global MIX file class pointer                |
