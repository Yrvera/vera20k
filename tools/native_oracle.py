"""Shared, bounded execution and reference-file workflow for retail YR oracles.

This is a CPU fixture runner, not a Windows loader or a game environment.
See tools/native_oracle.md for supported workflows and evidence limits.
Unicorn 2.1.4 API: https://github.com/unicorn-engine/unicorn/blob/2.1.4/include/unicorn/unicorn.h
"""

from __future__ import annotations

import argparse
from collections import deque
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import struct

import unicorn
from unicorn import Uc, UcError, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE, UC_QUERY_TIMEOUT
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_ESP,
    UC_X86_REG_EIP, UC_X86_REG_FPCW,
)

NATIVE_SHA256 = "1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c"
IMAGE_BASE = 0x00400000
# Preserve the original fixture mapping, including runtime globals in BSS.
IMAGE_SIZE = 0x00A00000
STACK_BASE, STACK_SIZE = 0x10000000, 0x00100000
SCRATCH, SCRATCH_SIZE = 0x20000000, 0x00010000
RET_MAGIC = 0x30000000
# Legacy RMG fixture contract: 53-bit precision, truncate. A caller must establish
# the appropriate ambient state for its own native entry; this is not universal.
NATIVE_FPCW = 0x0E7F


class OracleError(RuntimeError):
    """No trustworthy result was obtained; do not publish reference outputs."""


def configured_gamemd() -> Path:
    explicit = os.environ.get("VERA20K_GAMEMD_EXE")
    retail_dir = os.environ.get("RA2_DIR")
    if not explicit and not retail_dir:
        raise OracleError("Set VERA20K_GAMEMD_EXE or RA2_DIR to the original retail gamemd.exe")
    path = Path(explicit) if explicit else Path(retail_dir) / "gamemd.exe"
    path = path.expanduser().resolve()
    if not path.is_file():
        raise OracleError(f"Missing original executable: {path}")
    return path


@lru_cache(maxsize=2)
def _verified_image(path: Path) -> bytes:
    # Hash the same immutable bytes subsequently mapped, not a separate read.
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != NATIVE_SHA256:
        raise OracleError(f"Unsupported gamemd.exe SHA-256 {digest}; expected {NATIVE_SHA256}")
    return data


def image_bytes() -> bytes:
    return _verified_image(configured_gamemd())


def _sections(data: bytes):
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    machine, count = struct.unpack_from("<HH", data, pe + 4)
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    if (data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0" or machine != 0x14C
            or struct.unpack_from("<H", data, optional)[0] != 0x10B
            or struct.unpack_from("<I", data, optional + 28)[0] != IMAGE_BASE):
        raise OracleError("Expected the pinned PE32 x86 image at 0x00400000")
    for index in range(count):
        offset = optional + optional_size + index * 40
        virtual_size, rva, raw_size, raw_ptr = struct.unpack_from("<IIII", data, offset + 8)
        flags = struct.unpack_from("<I", data, offset + 36)[0]
        if rva + max(virtual_size, raw_size) > IMAGE_SIZE or raw_ptr + raw_size > len(data):
            raise OracleError("PE section exceeds the verified fixture mapping")
        yield rva, raw_ptr, raw_size, virtual_size, flags


def load_image(uc: Uc) -> None:
    """Map verified original PE sections, zero-fill gaps/BSS; no OS initialization.

    The mapping retains legacy RWX permissions for existing fixture hooks. That
    does not authorize treating patched code or supplied call results as native.
    """
    data = image_bytes()
    sections = list(_sections(data))
    uc.mem_map(IMAGE_BASE, IMAGE_SIZE)
    uc.mem_write(IMAGE_BASE, data[:0x1000])
    for rva, raw_ptr, raw_size, _, _ in sections:
        if raw_size:
            uc.mem_write(IMAGE_BASE + rva, data[raw_ptr:raw_ptr + raw_size])


def run_checked(uc: Uc, begin: int, end: int | tuple[int, ...], *,
                count: int = 5_000_000, timeout_us: int = 10_000_000,
                required_addresses=()) -> int:
    """Execute to a declared return/region boundary or fail with a short trace.

    Boundaries are reached BEFORE executing their instruction. Existing hooks may
    remain, but stopping anywhere else fails. No faults are swallowed. Unicorn's
    count/time limits stop emulation normally, so absence of UcError is not proof
    of completion (uc.c:987, unicorn.h uc_emu_start). This function owns exits for
    this run; custom ctl_set_exits are disabled in favor of these explicit ends.
    """
    ends = (end,) if isinstance(end, int) else tuple(end)
    if not ends or begin in ends or count <= 0 or timeout_us <= 0:
        raise ValueError("Use distinct entry/endpoints and positive instruction/time limits")
    required = set(required_addresses)
    if required.intersection(ends):
        raise ValueError("Required instruction addresses must precede the stop boundary")
    visited = set()
    trail = deque(maxlen=16)

    def observe(_uc, address, _size, _data):
        trail.append(address)
        if address in required:
            visited.add(address)
        if address in ends:
            _uc.emu_stop()

    uc.ctl_exits_enabled(False)
    hook = uc.hook_add(UC_HOOK_CODE, observe)
    try:
        try:
            uc.emu_start(begin, ends[0], timeout=timeout_us, count=count)
        except UcError as error:
            trace = ", ".join(f"0x{x:08X}" for x in trail)
            raise OracleError(f"Native execution 0x{begin:08X} faulted: {error}; trace: {trace}") from error
        pc = uc.reg_read(UC_X86_REG_EIP)
        if uc.query(UC_QUERY_TIMEOUT) or pc not in ends:
            trace = ", ".join(f"0x{x:08X}" for x in trail)
            raise OracleError(
                f"Incomplete execution from 0x{begin:08X}: stopped at 0x{pc:08X}; "
                f"expected {', '.join(f'0x{x:08X}' for x in ends)} "
                f"(limit or early stop); trace: {trace}")
        missing = required - visited
        if missing:
            raise OracleError(f"Required native instruction addresses not reached: {sorted(hex(x) for x in missing)}")
        return pc
    finally:
        uc.hook_del(hook)


def call(func: int, *, ecx=None, edx=None, stack_args=None, writes=None,
         dumps=None, capture_st0=False, fpcw=NATIVE_FPCW,
         timeout_instr=5_000_000, timeout_us=10_000_000, required_addresses=()) -> dict:
    """Run one function in fresh state. Results preserve the legacy harness schema.

    ECX/EDX and stack arguments are explicit calling-convention inputs. Writes
    supply fixture data; executable-section writes are rejected. ST0 capture
    stores binary64 through a six-byte FSTP return stub outside native memory;
    it is a declared observation conversion, not full x87 80-bit capture.
    """
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    sections = list(_sections(image_bytes()))
    if not any(flags & 0x20000000 and IMAGE_BASE + rva <= func < IMAGE_BASE + rva + raw
               for rva, _, raw, _, flags in sections):
        raise OracleError("call() entry must be an original native executable-section address")
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, SCRATCH_SIZE)
    uc.mem_map(RET_MAGIC, 0x1000)
    executable = [(IMAGE_BASE + rva, IMAGE_BASE + rva + max(raw, size))
                  for rva, _, raw, size, flags in sections if flags & 0x20000000]
    for address, blob in (writes or {}).items():
        if any(address < high and address + len(blob) > low for low, high in executable):
            raise OracleError("call() fixture writes cannot replace native executable instructions")
        uc.mem_write(address, blob)
    stop_at = RET_MAGIC
    st0_slot = RET_MAGIC + 0x100
    if capture_st0:
        uc.mem_write(RET_MAGIC, b"\xdd\x1d" + struct.pack("<I", st0_slot))
        stop_at += 6
    sp = STACK_BASE + STACK_SIZE - 0x1000
    for value in reversed(stack_args or []):
        sp -= 4
        uc.mem_write(sp, struct.pack("<I", value))
    sp -= 4
    uc.mem_write(sp, struct.pack("<I", RET_MAGIC))
    uc.reg_write(UC_X86_REG_ESP, sp)
    for register, value in [(UC_X86_REG_FPCW, fpcw), (UC_X86_REG_ECX, ecx), (UC_X86_REG_EDX, edx)]:
        if value is not None:
            uc.reg_write(register, value)
    required = set(required_addresses)
    if capture_st0:
        required.add(RET_MAGIC)
    run_checked(uc, func, stop_at, count=timeout_instr, timeout_us=timeout_us,
                required_addresses=required)
    result = {"eax": uc.reg_read(UC_X86_REG_EAX) & 0xFFFFFFFF, "dumps": {}}
    for name, (address, length) in (dumps or {}).items():
        result["dumps"][name] = bytes(uc.mem_read(address, length)).hex()
    if capture_st0:
        raw = bytes(uc.mem_read(st0_slot, 8))
        result.update(st0_bits=struct.unpack("<Q", raw)[0], st0=struct.unpack("<d", raw)[0])
    return result


def provenance(*, scope: str, assumptions: list[str], substitutions: list[str],
               entry_points: dict[str, int]) -> dict:
    """Record identity and claims separately from legacy vector payloads."""
    if not scope.strip() or not assumptions or not entry_points:
        raise ValueError("Declare scope, runtime assumptions, and native entry points")
    return {
        "schema_version": 1, "native_sha256": hashlib.sha256(image_bytes()).hexdigest(),
        "unicorn_binding": unicorn.__version__, "unicorn_core": list(unicorn.uc_version()),
        "scope": scope, "assumptions": assumptions, "substitutions": substitutions,
        "entry_points": {name: f"0x{value:08X}" for name, value in entry_points.items()},
    }


def _canonical(data) -> bytes:
    return json.dumps(data, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def first_difference(expected, actual, path="$", limit=180) -> str | None:
    """Locate the first mismatch without dumping whole model states."""
    if type(expected) is not type(actual):
        return f"{path}: expected {type(expected).__name__}, got {type(actual).__name__}"
    if isinstance(expected, dict):
        if expected.keys() != actual.keys():
            return f"{path}: missing keys {sorted(expected.keys() - actual.keys())}, extra keys {sorted(actual.keys() - expected.keys())}"
        for key in expected:
            if difference := first_difference(expected[key], actual[key], f"{path}.{key}"):
                return difference
    elif isinstance(expected, list):
        if len(expected) != len(actual):
            return f"{path}: expected {len(expected)} entries, got {len(actual)}"
        for index, (left, right) in enumerate(zip(expected, actual)):
            if difference := first_difference(left, right, f"{path}[{index}]"):
                return difference
    elif isinstance(expected, float) and struct.pack("<d", expected) != struct.pack("<d", actual):
        return f"{path}: expected {expected!r}, got {actual!r} (binary64 differs)"
    elif isinstance(expected, str) and expected != actual and max(len(expected), len(actual)) > limit:
        offset = next((i for i, (left, right) in enumerate(zip(expected, actual)) if left != right),
                      min(len(expected), len(actual)))
        start = max(0, offset - 24)
        return (f"{path}: first differing character {offset}; "
                f"expected {expected[start:offset + 40]!r}, got {actual[start:offset + 40]!r}")
    elif expected != actual:
        return f"{path}: expected {repr(expected)[:limit]}, got {repr(actual)[:limit]}"
    return None


def finish_vectors(data, default_path: Path, *, provenance: dict, argv=None) -> None:
    """Default: check without writing. --write deliberately replaces the reference.

    Existing payloads retain their Rust-facing schema. A .meta.json sidecar records
    provenance and the canonical payload hash. Old files without metadata can be
    compared, but their historical provenance is explicitly unknown.
    """
    parser = argparse.ArgumentParser(description="Compare native outputs with recorded reference data")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="compare only (default)")
    mode.add_argument("--write", action="store_true", help="explicitly write outputs and provenance")
    parser.add_argument("--output", type=Path, default=default_path)
    args = parser.parse_args(argv)
    data = data() if callable(data) else data
    provenance = provenance() if callable(provenance) else provenance
    # Normalize tuples before comparisons; reject NaN/Infinity in either workflow.
    normalized = json.loads(_canonical(data))
    metadata = dict(provenance, payload_sha256=hashlib.sha256(_canonical(normalized)).hexdigest())
    payload_text = json.dumps(normalized, indent=2, allow_nan=False) + "\n"
    metadata_text = json.dumps(metadata, indent=2, allow_nan=False) + "\n"
    target = args.output
    sidecar = target.with_suffix(".meta.json")
    if args.write:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload_text, encoding="utf-8")
        sidecar.write_text(metadata_text, encoding="utf-8")
        print(f"WROTE {target} and {sidecar}; review before accepting changed native references")
        return
    if not target.is_file():
        raise OracleError(f"Reference missing: {target}; use --write to deliberately create it")
    expected = json.loads(target.read_text(encoding="utf-8"))
    if difference := first_difference(expected, normalized):
        raise OracleError(f"Reference mismatch in {target}: {difference}")
    if sidecar.is_file():
        expected_metadata = json.loads(sidecar.read_text(encoding="utf-8"))
        if difference := first_difference(expected_metadata, metadata):
            raise OracleError(f"Provenance mismatch in {sidecar}: {difference}")
    else:
        print("NOTE: legacy reference has no provenance sidecar; historical environment is unknown")
    print(f"PASS {target}: native outputs match; no files written")
