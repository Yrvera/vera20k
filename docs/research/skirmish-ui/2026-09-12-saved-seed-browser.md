# Saved random-map browser

This increment addresses the ordinary pre-match Load, Save and Delete journeys
owned by `RunModalLoop` at `0x00558DD0`. It does not close the all-shell acceptance
inventory or demonstrate complete rendered parity. The user asked that ordinary
player flows take priority over rare edge cases.

## Native behavior established

Evidence is the original `gamemd.exe` (SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`), its active
MapSeed vtable at `0x007ED8E4`, original instructions, and retail `ra2md.csf`.
Independent read-only reviewers checked these mechanisms separately from the
implementation.

- Dialogs are Load `0xB7`, Save `0x2B4`, Delete `0x2B5`; MapSeed supplies
  `GUI:LoadMapMenu`, `GUI:SaveMapMenu`, `GUI:DeleteMapMenu`. Resource mission
  prompts remain genuine, but `0x00558F8A` hides static `0x40C` in ordinary
  pre-match mode. Session `+0x30D8` is set only by in-game suspension
  (`0x0069BAB0`) and cleared by `0x0069BB40`.
- Metadata callback `0x00597D60` rejects load failure and reserved names;
  accepted empty descriptions remain invalid records. `0x005596A0` adds Save's
  `TXT_EMPTY_SLOT` record, sorts all metadata, then filters invalid rows. New's
  timestamp comes from GetSystemTime and therefore has millisecond precision.
  File timestamps retain their original FILETIME precision.
- Comparator `0x00559D30` negates CompareFileTime, with no secondary key.
  Original CRT qsort `0x007C8B48` and short-sort `0x007C8C9C` have observable
  unstable equal-time order. Load reuses an unfiltered metadata index as a
  visible selection index, including its wrong/no-selection outcomes.
- Enumeration uses FindFirstFileA. The metadata reader opens the full name,
  then copies at most 32 ANSI bytes for later row actions (`0x00597E96`). These
  bytes stay separate from display strings. Raw availability `0x0065CBF0`
  probes GENERIC_READ, share READ, OPEN_EXISTING, NORMAL; actual read-open
  `0x0065CB50` uses share READ|WRITE and SEQUENTIAL_SCAN|NORMAL.
- New names follow `SAVE%04lX.SED`, drawing from thread CRT rand at
  `0x007CB4AA`. The app owns one persistent stream. Existing rows retain their
  filenames regardless of the edited description.
- Custom NewEdit `0x00614B30` keeps a UTF-16 caret, no selection range. Save's
  `0x00558B90` configures its 79-unit limit; programmatic assignment also obeys
  this limit (`0x007B78D0`). Save's
  single-byte typing restriction stays disabled; input units above `0x1F` are
  accepted subject to that limit. Save trims units at or below `0x20`.
- Empty text uses `TXT_MUSTENTER_DESCRIPTION` and OK, then explicitly focuses
  the edit. Existing physical availability triggers `TXT_CONFIRM_SAVE`, Yes/No.
  MapSeed Save `0x00597760` updates the working description before I/O and
  returns success on its nonnull-name path even when opening fails. Success
  acknowledges `GUI:MapSaved`, then closes. Failed writes remain diagnostic.
- The shared modal pump calls IsDialogMessageA (`0x005D4DDB`); shared message
  proc `0x005D36A0` maps initial-focus IDOK and IDCANCEL to result 1. Enter
  dismisses acknowledgments/warnings and cancels an initially focused
  confirmation. It must not fall through to the browser's Save edit.
- Delete uses `TXT_DELETE_FILE_QUERY` plus two newlines and the description,
  Yes/No. It ignores DeleteFileA's result, removes the row, selects the first
  remaining row and closes when empty. Re-entry refreshes actual disk state.
- Ordinary Load posts Generate with its one-shot reroll latch clear
  (`0x00596963`) and synchronizes controls (`0x00596E50`). Valid control
  readback preserves saved derived fields/seed, assigns height from width,
  stamps localized `TXT_RANDOM_MAP_DESCRIPTION`, and generates. Load list
  double-click activates this path; Save/Delete double-click does not.
- List columns start at inner x=2/255/315, width=249/56/remaining. GAME.FNT is
  17 px high; rows are 19 px. Overflow removes UTF-16 units and appends `...`.
  Date/time use the host Windows short date and time APIs separately.
- The scrollbar has a shared one-pixel border, 20 px outer width, 18 px inner
  width and 22 px arrows. Native `0x0061C818` computes a logarithmic thumb,
  truncates it, and enforces a 14 px minimum. Track clicks jump; thumb dragging
  centers on the pointer; arrows repeat after 500 ms and every 25 ms afterward.

## Architecture and implementation

`map/rmg/saved_seeds` owns metadata and persistence; browser state owns rows,
selection and the description edit; `app/shell_saved_seeds` orchestrates modal
transactions and generation. Raw UTF-16 descriptions survive SED persistence.
Display conversion is explicit. Byte filenames remain byte-oriented through
Windows file operations. The existing save-game timestamp formatter moved to
`util/native_file_time`, giving both consumers the same locale boundary without
creating a dependency from random maps to the save-game UI.

The browser takes input before its hidden parent dialogs. Its ordinary layout
uses the existing shell background, right rail, lower strip and button assets.
Prompt masks prevent underlying browser text from painting through the panel in
the renderer's separate sprite/text passes.

## Validation and coverage limits

Native executable comparisons are preserved in:

- `tools/storage_oracle/seed_order.py`: 36 supplied order/comparison-count cases.
- `tools/storage_oracle/crt_random.py`: 7 supplied seeds, 32 draws each.
- `tools/storage_oracle/saved_scrollbar.py`: 450 supplied height/range cases.
- The prior `sed_description` harness: 28 cold-reader cases and one cached
  diagnostic; raw UTF-16 persistence adds Rust regression coverage.

Commands use `python -m tools.storage_oracle.<name>` with the retail executable
configured through `VERA20K_GAMEMD_EXE`. Sidecars record binary identity,
substitutions and sample limits. Rust tests consume the native payloads.

The full Rust suite passed with **8,690 passed, 0 failed, 121 ignored** (20.58 s).
Clippy completed successfully with 1,150 repository warnings. These results cover
the candidate including the independent review's prompt-Enter correction.

An isolated production release run configured for 800x600 exercised the ordinary startup
route into RMG and the browser: blank validation and refocus, physical-key edit
and Enter Save, acknowledgment and return, existing selection, overwrite No
(unchanged file timestamp) and Yes (updated timestamp), Load by double-click with
regeneration, Delete No and Yes, last-row closure and availability refresh, and
arrow/drag scrolling across 24 fixture entries. The visible prompt and scrolled
browser were retained as 642x511 logical-pixel window captures for independent
review; capture dimensions include window chrome and are distinct from the
configured physical client resolution. These are Rust UI observations,
not native screenshot comparisons.

The final production release build (SHA-256
`af9e26073f00a6ccba70c0cd3616886832e88bd8dca788c16f607ac1c45ca50c`)
rechecked normal Generate → Save with localized `Random Map`, physical-key
editing, empty-warning Enter dismissal with restored edit focus, Enter Save,
and Enter acknowledgment returning to RMG with its preview intact. Initial
overwrite and delete confirmation Enter both canceled: the test file remained
245 bytes with unchanged SHA-256 and last-write timestamp. Local captures and
file-check results were retained for final independent review.

Fresh independent read-only review passed this increment after correcting
prompt Enter handling and checking the final runtime evidence. Independently
confirmed comments at `0x00558DD0`, `0x005596A0` and `0x00614B30` were saved to
the intended Ghidra program and read back; existing role labels were retained.

The first run inherited 640x480 from its copied RA2MD.INI and exposed parent
skirmish/chooser overlap. A subsequent independent static audit found retail
normally selects its separate 800x600 frontend pair, switching to the configured
game pair only for a match (`0x006BDAF9`, `0x00683DBB`). Rust's startup/lifecycle
sizing therefore needs a separate increment; the observed overlap does not
establish incorrect native 640 geometry. Test saves and settings live in an isolated local runtime
folder; user retail saves were not changed.

Remaining bounded limitations include unsampled x87 rounding edges, exotic INI
line-reader/BOM forms, OS IME delivery and exact caret phase, and malformed
control values absent from the valid combo lists. CRT history after tactical
sparkles or network reseeds is not yet integrated; the saved-browser stream
must not be described as a guaranteed retail first-save filename. Runtime file
failure/media-retry behavior is narrower than the complete native media loop.
These limits must not be turned into a full browser or all-shell parity claim.
