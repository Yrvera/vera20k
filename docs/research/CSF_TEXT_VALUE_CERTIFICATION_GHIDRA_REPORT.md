# CSF Text-Value Certification + Load-Time Whitespace Normalization — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe) + corpus-proven + FIX IMPLEMENTED
**Goal:** upgrade CSF display-text VALUES from ratchet-only to a citable
certification. Found and fixed a real player-visible parser gap in the process.

## Verdict

**The original engine does not display CSF strings as stored on disk — its
loader applies a whitespace-normalization pass to every string at load time.
213 of 9,690 retail strings (mission briefings, tooltips, skirmish UI text)
are changed by it.** Our decoder previously returned the raw decoded text —
a real drift (e.g. `Tip:ThumbClosed` renders as
`"Click to show\nadvanced commands"` in gamemd but carried
`"Click to show \n advanced commands"` in our table). The normalization is
now implemented in `src/assets/csf_file.rs::normalize_whitespace` and every
retail string is certified by `certify_csf_text_values`
(tests/retail_goldens/certify_structural.rs): parser output ==
independently-restated NOT-decode + normalization from the raw bytes, for
all labels of both retail CSFs (9,687 unique labels).

**Trigger frequency:** briefing screens (all 3 campaign brief families),
several shell tooltips and WDT territory strings — visible whenever those
screens show; the padding spaces would have rendered as misaligned line
starts/ends in our engine.

## 1. Native loader behavior (verified)

`CSF_ParseLabelStringChunks @ 0x00734990` (verified via
`decompile_function 0x00734990`); header read in
`CSF_LoadHeaderAndLanguage @ 0x007346A0` (24-byte header consumed before the
record loop).

Per label record:
- Accepts label magic `0x4C424C20` (" LBL") ONLY; the loop keeps reading
  records until a 4-byte read fails or the magic mismatches — the header's
  label count does NOT drive the loop. (Our parser drives by header count;
  `certify_csf_structural` already proves count == records == EOF on retail
  files, so both terminate identically.)
- Accepts string magics `0x53545220` (" STR") and `0x53545257` ("WSTR" —
  value-with-extra) ONLY. Our parser accepts three additional legacy
  spellings — more lenient, no retail effect (corpus contains only the two).
- NOT-decode: `*p = ~*p` per UTF-16 unit (equivalent to our per-byte NOT).
  The loop stops at a pre-NOT zero unit; retail strings contain none
  (corpus walk covers every record exactly).
- **Whitespace normalization** (the finding), applied to the decoded UTF-16
  buffer before storing:
  ```
  prev = 0; at_line_start = true
  for each unit c:
    c == 0x20:      copy only if prev != 0x20 and !at_line_start
                    (skipped spaces do not update prev/at_line_start)
    c == 0x0A/0x09: if prev == 0x20 remove last copied unit;
                    copy c; at_line_start = true
    else:           copy c; at_line_start = false
  after loop: if prev == 0x20 remove last copied unit
  ```
  Effects: consecutive spaces collapse to one; leading spaces (string start
  or after \n/\t) dropped; a space immediately before \n/\t dropped; one
  trailing space trimmed.
- "W" records additionally carry a length-prefixed ASCII extra blob (sound
  cue name), stored separately by the engine; our parser skips it (framing
  identical, content unused by `get`).

## 2. Fix implemented (this session)

`src/assets/csf_file.rs`: `decode_csf_string` now applies
`normalize_whitespace` (exact restatement of the loop above over UTF-16
units). Unit test `load_time_whitespace_normalization_matches_engine`; corpus
certification `certify_csf_text_values`. CSF ratchet digests re-baselined
(rollup + ra2md.csf named digest) — reason: intended decoder fix to match
native load-time normalization.

## Confidence axes

| Claim | Content | Identity | Binding |
|---|---|---|---|
| Normalization algorithm | HIGH (decompiled loop, restated 1:1) | HIGH (`CSF_ParseLabelStringChunks @ 0x00734990`) | HIGH (reached from the CSF load path; the two retail CSFs parse through it) |
| Magic strictness (1 label + 2 string spellings) | HIGH | HIGH | HIGH |
| 213-string retail impact | machine-measured (corpus pass) | — | — |
| Full text-value equality after fix | machine-proven (`certify_csf_text_values`, 9,687 labels) | — | — |

## Notes / non-goals

- The engine stores all pair values per label; our parser keeps the first.
  Retail labels are single-pair (raw walk consumes every record exactly), so
  no observable difference.
- Label-name handling (uppercase lookup) unchanged and already covered by
  `certify_csf_structural`.
