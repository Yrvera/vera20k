# frontier-net-transport — IPX/UDP transport + connection manager

**Slug:** `frontier-net-transport`
**Status:** PROMOTED from catalog stub (was `_frontier.md` §E2 — UN-STUDIED) → full profile.
**Layer:** `net` (out-of-sim wire transport; below `frontier-net-eventqueue`).
**Tick / render plug point:** OUT-OF-SIM transport. Driven from the per-tick spine's
network-service step, NOT from `LogicClass::PerTickUpdate` object passes. See §4.
**Active in YR:** Conditional — see §6. The connection-manager + send/receive plumbing is
LIVE for LAN (g_GameMode==3) and WOL (g_GameMode==4). The IPX socket transport underneath
is **OS-dead on Vista+** (IPX protocol removed from Windows), and the WOL servers are
offline since 2004. Modem/serial (modes 1/2) is RA2/TS-legacy, effectively dead in YR.

---

## ⚠️ Verification status this session (READ FIRST)

**No live Ghidra instance was available this session.** `list_instances` returned zero
instances; the TCP fallback (`127.0.0.1:8089`) actively refused; `check_tools` reported
`decompile_function` / `get_function_by_address` as `not_found`. Per the project's
no-invented-facts discipline, **I did not re-verify the representative address live this
session and have not fabricated a verification call.**

Every address below is sourced from **existing Ghidra-verified corpus docs** — each of
those docs cites the exact `decompile_function` call made when the instance WAS live (those
citations are reproduced inline). The representative-address re-verification was downgraded
to **corroboration-by-adjacency** (§2). Anything not corroborated by a corpus doc is marked
**UNVERIFIED / YELLOW** and must be confirmed live before implementation.

Authority order remains binary → Ghidra → docs. This doc is at the **docs** tier with
**Ghidra-verified upstream**, one step weaker than a live re-verification.

---

## 1. Purpose

The wire-transport substrate beneath the lockstep event queue. Responsibilities:

- **Connection manager** — tracks the set of remote peers (the "IPX_Manager" / session
  object), per-peer retry parameters (RetryDelta, MaxRetries, RetryTimeout), per-peer
  response-time / RTT accumulators, and adapter availability.
- **Socket transport** — the IPX socket layer (LAN, modes 1–3) and the UDP/WOL layer
  (mode 4): bind, send, receive, packet framing, guaranteed vs unguaranteed delivery,
  retransmit.
- **Packet plumbing** — serialize an outgoing event/keepalive/sync packet to the chosen
  transport; ingest raw incoming packets and route them into `frontier-net-eventqueue`'s
  command ring buffer.

It does **not** schedule frames, compute MaxAhead/FrameSendRate, or execute events — that
is `frontier-net-eventqueue` (E1). This service is the byte pipe + peer bookkeeping; E1 is
the lockstep contract on top of it.

---

## 2. Representative function — re-verification result

**Stub claimed:** `IPXManagerClass__Constructor @ 0x005408F0` (representative); "other
transport AI entries unlabeled — locate via ConnManClass/ConnectionClass vtable before
study."

**Result:** I could **not live-re-verify** `0x005408F0` this session (no instance). It is,
however, **strongly corroborated by adjacency** to a band of live-verified IPX-Manager
functions that the corpus pins by string content:

| Address | Role | Corpus evidence (cites live `decompile_function`) |
|---|---|---|
| `0x00540A80` | network-adapter / IPX-driver availability check (returns 0 = no network) | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §3 — `FUN_005DB680` "calls `FUN_00540A80()` to check network availability" (verified via `decompile_function 0x005DB680`) |
| `0x00540c60` | IPX-Manager retry-parameter configurator; prints `"IPX_Manager: RetryDelta = %d"` and `"MaxAhead is %d"`, then propagates RetryDelta/MaxRetries/RetryTimeout to each peer struct | `MAXAHEAD_NETWORK_FRAME_BUDGET` §8 — verified via `decompile_function 0x00540c60` |
| `0x00540F90` | peer-count getter (`return *(ECX+0x44)` on the session object `0x00A8E9C0`) | `NETWORK_FRAME_SCHEDULING` §1/§2 — verified via `decompile_function 0x00540F90` + `get_assembly_context 0x0055D5E6` |
| `0x005410F0` | LAN/WOL send (modes 3/4): sends a packet to a remote peer | `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3, `NETWORK_FRAME_SCHEDULING` — verified via `decompile_function 0x00648710`, `0x0048D1E0` |
| `0x0053F200` | IPX send (modes 1/2) | `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3 |
| `0x00541820` | parse raw packet into the command queue | `NETWORK_FRAME_SCHEDULING` §7 — referenced from `FUN_0048D1E0` decompile |
| `0x005422D0` | "all remote players committed for this frame" predicate | `MP_SYNC_CRC_DESYNC_RECONNECT` §3.3, `DESYNC_DETECTION_MAINTICK_COMPARE` §5 — verified via `decompile_function 0x0048D1E0` |
| `0x00542520` | `Network_Keepalive` — per-peer RTT accumulator update (mode 4, every 8 frames) | `NETWORK_FRAME_SCHEDULING` §6 — verified via `decompile_function 0x00542520` |

`0x005408F0` sits **immediately before** `0x00540A80` and `0x00540c60` in the same
contiguous IPX-Manager code band (`0x00540xxx`–`0x00543xxx`). A constructor for the
IPX/connection manager living at the head of that band is consistent with the corpus, but
the **exact identity, signature, and "this is the IPXManagerClass constructor" claim are
UNVERIFIED this session** — treat as YELLOW until a live `decompile_function 0x005408F0` +
`get_function_callers 0x005408F0` confirms it.

**Better-anchored representative for the connection manager (corroborated):**
`0x00540c60` (the "IPX_Manager: RetryDelta = %d" configurator) is the most defensible
single representative function for this service — its identity is pinned by literal string
content in a live-verified decompile, not by adjacency. Use it as the anchor for the next
live study; expand to `0x005408F0` (constructor), `0x00540A80` (adapter check),
`0x00540F90` (peer count), and the send/receive pair (`0x005410F0`/`0x0053F200` +
`0x00541820`) from there.

---

## 3. What it owns (globals / structs)

All addresses below come from corpus docs that cite live `decompile_function` /
`get_assembly_context` / `list_globals` calls; not re-verified live this session.

### 3.1 The session / connection-manager object

| Global | Address | Meaning | Corpus evidence |
|---|---|---|---|
| Session object ("IPX_Manager") | `0x00A8E9C0` | The connection-manager singleton. `+0x44` = peer count; `+0x28` = peer list base; `+0x20`/`+0x1C` = response-time slots read by keepalive. | `NETWORK_FRAME_SCHEDULING` §2/§6 (`FUN_00540F90` reads `*(ECX+0x44)` on `0x00A8E9C0`; keepalive iterates `param_1+0x44` count, `param_1+0x28` list) |
| WOL plug-in object ptr | `0x00B45B5C` | WOL-SDK plug-in object; `Network_ServiceLoop` calls its vtable slot 2 (`(*(*DAT_00B45B5C + 8))()`) when `DAT_00B45B68 > 0`. Object type/content untraced. | `MP_SYNC_CRC_DESYNC_RECONNECT` §3.2/§9, `DESYNC_DETECTION_MAINTICK_COMPARE` §4 (verified via `decompile_function 0x0048D080`) |
| WOL plug-in count gate | `0x00B45B68` | `> 0` gates the plug-in vtable call above. | same |

### 3.2 Per-peer transport bookkeeping (fixed-size — SCALE FLAGS)

| Global | Address | Meaning | Cap | Corpus evidence |
|---|---|---|---|---|
| Per-peer keepalive/RTT struct array | `0x00A8B5B4` (stats variant `0x00A8B5C4`) | Response-time / RTT accumulators per peer, stride `0x68` (0x1A dwords). Updated by `Network_Keepalive`; dumped to `mpstats.txt`. | **8 peers** (`0x00A8B5C4..0x00A8B904` = 8 × 0x68) | `NETWORK_FRAME_SCHEDULING` §6, `MAXAHEAD_NETWORK_FRAME_BUDGET` §7/§10 (verified via `decompile_function 0x00542520`, `0x0048e0b0`) |
| Per-peer lag accumulator array | `0x00A8DB7C` | Max-lag snapshot per non-local peer, `int[7]`, stride 4. Read by the adaptive throttle in `Main_Tick`; zeroed on successful frame advance. | **7 non-local (= 8 total)** | `NETWORK_FRAME_SCHEDULING` §3 (verified via `get_assembly_context 0x0055D5D0`) |
| Peer ping-time array | `0x00B7790C` | Per-peer ping times, 8 entries (loop bound `< 0x00B7792C`). | **8 peers** | `MAXAHEAD_NETWORK_FRAME_BUDGET` §10 |
| Connected peer count | `0x00A8DA84` | Live peer count; loop bound across many transport/sync functions. Majority/kick threshold = count − 1. | — | `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3, §3.3 |
| Peer list base ptr | `0x00A8DA78` | Pointer array of peer records. | — | `NETWORK_FRAME_SCHEDULING` §13 (count at `0x00A8DA84`) |
| Retry params (RetryDelta etc.) | (peer struct fields, set by `0x00540c60`) | RetryDelta, MaxRetries, RetryTimeout propagated to each peer struct. | — | `MAXAHEAD_NETWORK_FRAME_BUDGET` §8 |

**These fixed-size 7/8-peer arrays are the scale ceiling for the 30-player target.** They
belong to this transport service (peer bookkeeping), distinct from E1's command ring
buffer. A 30-player port must replace each with a growable per-peer container.

---

## 4. Tick / spine plug point

This service is **out-of-sim transport** — it is driven from the per-tick spine but is not
a `LogicClass::PerTickUpdate` rung.

Reference spine: `LogicClass::PerTickUpdate @ 0x0055AFB0`, sole caller
`Main_Tick @ 0x0055D360`. The transport touch-points around the tick (all in
`NETWORK_FRAME_SCHEDULING` §10 tick-order, verified via `decompile_function 0x0055D360`):

- **Receive / route (post-PerTickUpdate, pre-frame-increment):**
  `Network_ServiceLoop @ 0x0048D080` → `FUN_0048D1E0` (modes 3/4) ingests raw packets and
  routes them into the command ring buffer (via `Process_NetworkMessages` + `0x00541820`).
  This is the **incoming** half of transport. It does NOT advance the frame counter.
- **Send (inside `FUN_0048D1E0`):** when all remotes have committed
  (`0x005422D0 == 0`), sends a frame-ready packet (type `0x29`) to each remote via
  `0x005410F0`; per-remote 120-tick (~2 s) resend deadline.
- **Keepalive (mode 4, every 8 frames):** `Network_Keepalive @ 0x00542520`, gated by
  `(g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4` — updates per-peer RTT.
- **Reconnect modal (on peer drop):** `FUN_00648710` runs a blocking reconnect dialog that
  keeps `Network_ServiceLoop` pumping while the lockstep stalls; sends keepalive (`0x27`,
  mode 4) and progress (`0x33`, every 0xB4 ms) packets via `0x005410F0` (modes 3/4) or
  `0x0053F200` (IPX, modes 1/2).

**Relation to E1:** the barrier `FUN_00648710`-gated advance and the execute-frame math
live in `frontier-net-eventqueue`. This service supplies the bytes E1 schedules and
consumes the bytes E1 produces. The boundary: **E1 owns "what frame does this event
execute"; E2 owns "get this packet to/from the peer."**

---

## 5. Edges

### 5.1 OUTGOING (this service depends on / calls into)

| → Service | Via symbol | Evidence |
|---|---|---|
| `frontier-net-eventqueue` | `0x00541820` (parse raw packet → command ring buffer), `0x005410F0`/`0x0053F200` (serialize outgoing event packet) | `NETWORK_FRAME_SCHEDULING` §7 (`FUN_0048D1E0` → `Process_NetworkMessages` + `0x00541820` into ring buffer `0x008B4204`); `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3. **Primary edge** — transport is the byte pipe under the event queue. |
| `random-scenario` | indirect — transport delivers the game-options packet carrying the RNG seed (`DAT_00A8ED94`); reorder/drop ⇒ desync | `RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE` §3 (LAN packet 0x65 → seed), §4 (WOL). Transport ordering is the determinism prerequisite. |
| `shell-dialog` (lobby host) | `Network_ServiceLoop`/peer state drives LAN-lobby + reconnect dialog controls | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §3 (LAN lobby `0x005DC350`), `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3 (reconnect dialog controls). |
| OS socket layer / WinSock / IPX driver | `0x00540A80` (adapter check), raw socket calls inside `0x005410F0`/`0x0053F200` | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §3 (`FUN_00540A80` adapter/IPX check). Below the 18 studied services — OS boundary. |

### 5.2 INCOMING (services that call into this one)

| ← Service | Via symbol | Evidence |
|---|---|---|
| `frontier-net-eventqueue` | calls transport send to push the outgoing packet; receives via the ring buffer transport fills | `NETWORK_FRAME_SCHEDULING` §4/§7. **Bidirectional primary edge** with E1. |
| `logicclass` (tick spine) | `Main_Tick` calls `Network_ServiceLoop @ 0x0048D080` and `Network_Keepalive @ 0x00542520` directly each frame (modes 3/4) | `NETWORK_FRAME_SCHEDULING` §6/§7/§10 (verified via `decompile_function 0x0055D360`). The spine is what drives the transport pump. |
| `shell-dialog` (LAN/WOL lobby + reconnect) | `0x005DB680` → `0x00540A80` to init/check the transport before opening the lobby; reconnect dialog `0x00648710` calls transport send | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §3, `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3. |
| `factory-house` / `techno-foot` (indirect) | their command events become packets that transport carries | via E1 — not a direct call edge into transport. |

### 5.3 Edge to the spine rung

Transport ties to the **post-PerTickUpdate network-service step of `Main_Tick`** (not a
lettered PerTickUpdate rung A–AB). In spine terms it is adjacent to the same
`Main_Tick`-level slot as `frontier-net-eventqueue`'s `Process_QueuedEvents` — the leading
"commands" stage of the Rust tick. The render-pass entry (`TacticalClass_Draw @ 0x006D3D10`)
is unrelated; transport has no render coupling.

---

## 6. Active-in-YR / TS-legacy ledger

| Path | Mode | Status | Evidence |
|---|---|---|---|
| Connection manager + peer bookkeeping (session obj `0x00A8E9C0`, retry params, RTT arrays) | 3 (LAN), 4 (WOL) | **LIVE in YR** (code reachable, no SpecialFlags gate) | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §4 ("Fully live in YR"); `NETWORK_FRAME_SCHEDULING` |
| LAN/WOL send `0x005410F0`, receive `0x0048D1E0`/`0x00541820`, keepalive `0x00542520` | 3, 4 | **LIVE in YR** (mode-4 features keepalive `0x27` etc. mode-4-gated) | `MP_SYNC_CRC_DESYNC_RECONNECT`, `NETWORK_FRAME_SCHEDULING` |
| IPX socket transport (underlying) | 1–3 | Code live, **OS-DEAD on Vista+** (IPX protocol removed from Windows; `0x00540A80`/`0x005DB680` return 0 → LAN bails to menu) | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §4 |
| WOL / UDP-internet transport | 4 | Code live, **servers offline since 2004** — login screen appears, never connects | `WWONLINE_NETWORK_BUTTON_CASES_2_3` §2/§4 |
| Modem / serial (NullModemClass), IPX send `0x0053F200` | 1, 2 | **RA2/TS-legacy**, effectively dead in YR; mode 2 not active in standard YR LAN/WOL | `DESYNC_DETECTION_MAINTICK_COMPARE` §9 ("mode 2 is not active in standard YR"), `MP_SYNC_CRC_DESYNC_RECONNECT` §5.3 |
| FogOfWar MaxAhead −10 transport reduction | any | **Not active in YR** (gated on SpecialFlags bit 12 = 0 in stock YR) | `MAXAHEAD_NETWORK_FRAME_BUDGET` §9 |

**Net:** the connection-manager + LAN/WOL packet plumbing is genuinely live in YR; the
*underlying socket transports* are all dead in practice (IPX OS-removed, WOL offline,
modem TS-legacy). For a from-scratch Rust port targeting real multiplayer, the transport
must be **rebuilt on modern UDP** — gamemd's IPX/WOL socket code is not a behavior to
reproduce, only the connection-manager semantics (peer tracking, retry params, keepalive
cadence, packet types, the receive→ring-buffer routing) are the parity contract.

---

## 7. Scale-limiting structures (30-player target)

| Structure | Address | Cap | Required change |
|---|---|---|---|
| Per-peer keepalive/RTT struct array | `0x00A8B5B4` / `0x00A8B5C4` | 8 (stride 0x68) | growable `Vec<PeerStats>` keyed by peer id |
| Per-peer lag accumulator | `0x00A8DB7C` | 7 non-local | `Vec<i32>` sized `peer_count − 1` |
| Per-peer ping-time array | `0x00B7790C` | 8 | growable per-peer |
| Peer list / count | `0x00A8DA78` / `0x00A8DA84` | unverified max | dynamic peer registry |
| Per-remote sequential polling in `FUN_0048D1E0` frame-gate | — | O(peers) per frame | parallel/event-driven ack collection (linear poll over 29 peers is a 30-player risk per `MP_SYNC_CRC_DESYNC_RECONNECT` §6) |

---

## 8. Remaining uncertainty (verify live before implementation)

1. **`0x005408F0` identity — YELLOW.** Not live-verified this session; corroborated only
   by adjacency. Run `decompile_function 0x005408F0` + `get_function_callers 0x005408F0`
   to confirm it is the IPXManagerClass/connection-manager constructor and capture its
   struct init (the `0x00A8E9C0` session object layout: `+0x44` count, `+0x28` list,
   `+0x1C`/`+0x20` response slots).
2. **`ConnManClass` / `ConnectionClass` / `NullModemClass` / `UDPInterfaceClass` vtables —
   UNVERIFIED.** The stub named these class families; none were located by address in the
   corpus this session. Locate via the session-object vtable and the send/receive call
   sites (`0x005410F0`, `0x0053F200`, `0x00541820`) before claiming the class taxonomy.
3. **WOL plug-in object at `0x00B45B5C` — type/vtable untraced** (`MP_SYNC_CRC` §9). Slot 2
   call content unknown; low priority (WOL offline).
4. **Packet-type catalog incomplete.** Confirmed types so far: `0x29` frame-ready,
   `0x27` WOL keepalive, `0x33` resend-progress (guaranteed), `0x5b` resend, `0x1C7`
   kick-vote, `0x65`/`0x67`/`0x6b` LAN game-options, `0x6C` CRC (mode 2). A full
   transport packet-type enumeration is a follow-up.
5. **Guaranteed vs unguaranteed delivery split.** `0x33` is noted "guaranteed"; the
   retransmit/ack mechanism (RetryDelta/MaxRetries/RetryTimeout from `0x00540c60`) governs
   it but the exact ack-tracking structure was not traced.

---

## 9. Sources

Corpus docs read this session (all Ghidra-verified upstream; each cites its own live
`decompile_function` calls):

- `docs/research/NETWORK_FRAME_SCHEDULING_GHIDRA_REPORT.md` — `Main_Tick`,
  `Network_ServiceLoop @ 0x0048D080`, `Network_Keepalive @ 0x00542520`, peer-count getter
  `0x00540F90`, session object `0x00A8E9C0`, lag array `0x00A8DB7C`.
- `docs/research/MP_SYNC_CRC_DESYNC_RECONNECT_GHIDRA_REPORT.md` — send funcs `0x005410F0`
  (LAN/WOL) / `0x0053F200` (IPX), commit predicate `0x005422D0`, `FUN_0048D1E0` frame-sync,
  reconnect `0x00648710`, packet types, peer count `0x00A8DA84`.
- `docs/research/MAXAHEAD_NETWORK_FRAME_BUDGET_GHIDRA_REPORT.md` — `IPX_Manager`
  configurator `0x00540c60` ("RetryDelta"/"MaxAhead is %d"), per-peer stats array
  `0x00A8B5C4` (8-cap), `mpstats.txt` dumper `0x0048e0b0`, scale flags.
- `docs/research/WWONLINE_NETWORK_BUTTON_CASES_2_3_GHIDRA_REPORT.md` — adapter check
  `0x00540A80`, LAN init `0x005DB680`, LAN lobby `0x005DC350`, active-in-YR / TS-legacy
  gating.
- `docs/research/DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md` — `Network_ServiceLoop`
  WOL plug-in `0x00B45B5C`, mode-2 IPX path, mode-2 dead-in-YR note.
- `docs/research/RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE_GHIDRA_REPORT.md` — seed packet
  (0x65 LAN / WOL), the transport→RNG ordering dependency.
- `docs/research/FRAMESENDRATE_COMMAND_CADENCE_GHIDRA_REPORT.md` — per-peer RTT array 8-cap,
  WOL FrameSendRate override from measured RTT.
- Reference: `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`; render entry
  `TacticalClass_Draw @ 0x006D3D10`.
- Stub: `docs/research/core-services-map/_frontier.md` §E2.

**Live Ghidra re-verification: NOT performed this session — no instance available.** Mark
all addresses as Ghidra-verified-via-corpus, not freshly re-verified. Next study must open
the live instance and confirm §8 items 1–2 first.
