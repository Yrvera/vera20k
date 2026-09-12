# Loose asset storage and shell persistence

Ordinary Skirmish Start and Options close failed to save `RA2MD.INI` on Windows
with error 1224: a user-mapped section was open. The process-owned `AssetManager`
indexed every regular file in the retail root by opening a lifetime memory
mapping, including the profile, saved seeds and generated previews. Its intended
lifetime across menus and matches made these write failures persistent.

## Acceptance and ownership

Keep the process asset manager alive while Skirmish and Options alternately
write the same existing profile. Both owners' latest values and unrelated file
content must survive. Existing saved seeds and generated preview files must be
overwritable, and subsequent direct readers must see the current bytes.

Preserve loose-before-MIX byte precedence, stable borrowed asset bytes, empty
file resolution, fallback to MIX when the loose payload cannot be read, and the
separate process-sticky CRC cache. Indexing and availability probes must not read
every archive/movie payload into memory.

`LooseAsset` now retains a path and a lazy owned byte snapshot. The first byte
lookup closes its file handle after reading. Borrowers therefore keep stable
storage without retaining a mapping of a file that another owner can truncate.
All byte lookup APIs share this boundary; a failed loose read falls through to
MIX. Archive storage and `LoadFileFromMIX` cache ownership remain unchanged.

This is a Rust storage policy, not an emulation of native raw-file caching.
The startup filename catalog remains fixed; each first read, including failure,
is retained for that manager's lifetime. `contains` checks catalog presence
without loading bytes and is not a guarantee that a future disk read succeeds.
Live asset hot reload and retry behavior are outside this increment.

Ordinary mutable consumers already read disk directly: Options profile and
Skirmish persistence, saved-seed browser and generation, generated preview
loading, and the ordinary map loader's disk-first branch. They do not consume
the asset snapshot as their mutable state authority. No per-writer retry or
mutable-file extension allowlist is introduced.

## Original evidence

Original `gamemd.exe` SHA-256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

- Options writer `0x005FAD10` constructs a stack CCFile at `0x005FAD22` with
  literal `RA2MD.INI` at `0x00826444`. It writes through `0x00474430` at
  `0x005FB007` and tears down the file object at `0x005FB027/0x005FB038`.
- The active launcher dialog loop calls that writer at `0x0055FD8E` after its
  primary/child routing. The in-game accepted Options path calls it at
  `0x004E1DAB`; independent review checked that caller's acceptance branch.
- The INI writer uses FilePipe: `0x007BA480` opens with mode 2 at
  `0x007BA4AC/0x007BA4B0`; its destructor `0x007BA420` conditionally closes a
  file it opened through virtual slot `+0x34`.
- CCFile open `0x00473D10` tests mode bit 2 at `0x00473D1D`, routing writes to
  `0x00473E39` and buffered/raw open instead of MIX resolution. Raw open
  `0x0065CB50` takes the mode-2 branch at `0x0065CB94`: write access, no sharing,
  create-always. This establishes short-lived physical write ownership; it
  does not establish equivalence of Rust's asset snapshot policy.

The original instructions, filename bytes and active callers were independently
checked. Existing labels served as leads. No native executable was modified.

## Validation state

The final library suite passed **8,698 tests, 0 failed, 121 ignored** (18.05 s).
Clippy completed successfully with 1,150 repository warnings (23.15 s).
Regressions exercise live-manager profile/seed writers, lazy stable loose
payloads with truncation, empty files, and failed-read fallback across byte APIs.
The first focused run exposed an out-of-range test seed (424242); using 42424
keeps this persistence test within the existing 16-bit normalization contract.
No seed behavior was changed.

Release SHA-256
`833bb2fae5cb4e2ad4f3233b0dee501fc1f761b83abdaafe4a5c052f8d282ba3`
passed isolated Windows production checks. Skirmish Start persisted Credits=6900;
the loaded match used that value. Game Controls close persisted ScrollRate=2.
After Abort returned to the shell, launcher acceptance persisted ScoreVolume=0.3.
The other owners' values and an unrelated sentinel section survived every write.
A fresh process restored those visible settings.

The saved-map browser overwrote an existing `SCROLL01.SED` (seed 42 to 56613)
and displayed Map Saved. Accepting the generated result replaced an existing
`RandMap.Sed` and nonempty `RandMap.img` fixture with the generated seed and
PCX-encoded preview. After restart the browser loaded the overwritten seed,
regenerated its preview and replaced both working files again. Captures, before/
after profile snapshots and file hashes were retained locally. The run's logs
contained no profile/seed/preview write failure or error 1224. These are production
persistence checks, not a native frame or random-map determinism comparison.

Fresh independent read-only criticism passed the evidence, design, final diff
and validation within this scope. Independently confirmed comments were added
at `0x005FAD10` and appended at `0x00473D10`, preserving existing comments and
labels. The program was saved and both annotation readbacks matched exactly.

This corrects an ordinary persistence obstruction. It does not certify all
shell visuals, launcher layout, native raw-file live reload, or the complete
game-start bootstrap. The existing Game Controls view also exposes missing
chrome/labels/buttons over tactical art; that ordinary in-game shell gap was
captured for subsequent rendering work.
