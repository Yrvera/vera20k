"""Original Theme WAV duration blocks and retained-or-pending queries.

The active B8 callback6B6300 reads names/durations from the Theme catalog.
7207F0 reaches408610's WAV descriptor calculation,408560's complete duration
helper, and7208D3's seconds store. File I/O/chunk parsing is supplied here;
all rate/duration outputs execute original instructions. No playback decoding.
"""

from pathlib import Path
import hashlib
import struct

from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import (
    UC_X86_REG_EAX, UC_X86_REG_EBX, UC_X86_REG_ECX, UC_X86_REG_EDX,
    UC_X86_REG_ESI, UC_X86_REG_EBP, UC_X86_REG_ESP, UC_X86_REG_FPCW,
)
from tools.native_oracle import (
    configured_gamemd, finish_vectors, load_image, provenance, run_checked,
    STACK_BASE, STACK_SIZE, SCRATCH, SCRATCH_SIZE, RET_MAGIC,
)
from tools.sidebar_oracle.stock import mix, mix_hash


STEMS = ("BrainFre", "Drok", "Deceiver", "PhatAtta", "Bully", "Defend",
         "Tactics", "TranceLV")


def emulator():
    uc = Uc(UC_ARCH_X86, UC_MODE_32)
    load_image(uc)
    uc.mem_map(STACK_BASE, STACK_SIZE)
    uc.mem_map(SCRATCH, SCRATCH_SIZE)
    uc.mem_map(RET_MAGIC, 0x1000)
    # These small integer FILD/FST values are exact under either rounding mode.
    uc.reg_write(UC_X86_REG_FPCW, 0x027F)
    uc.reg_write(UC_X86_REG_ESP, STACK_BASE + 0x80000)
    return uc


def native_metadata(kind, name, header, fmt, data_bytes, sha256=None):
    uc = emulator()
    uc.mem_write(SCRATCH, fmt.ljust(32, b"\0"))
    uc.mem_write(SCRATCH + 256, b"\0" * 32)
    uc.reg_write(UC_X86_REG_ESI, SCRATCH)
    uc.reg_write(UC_X86_REG_EBX, SCRATCH + 256)
    run_checked(uc, 0x408751, 0x4087C6, count=200,
                required_addresses=[0x40878C, 0x4087B6])
    descriptor = struct.unpack("<8I", uc.mem_read(SCRATCH + 256, 32))
    uc.reg_write(UC_X86_REG_ECX, SCRATCH + 256)
    uc.reg_write(UC_X86_REG_EDX, data_bytes)
    uc.mem_write(STACK_BASE + 0x80000, struct.pack("<I", RET_MAGIC))
    run_checked(uc, 0x408560, RET_MAGIC, count=1000,
                required_addresses=[0x408575, 0x408585])
    milliseconds = uc.reg_read(UC_X86_REG_EAX) | (uc.reg_read(UC_X86_REG_EDX) << 32)
    uc.reg_write(UC_X86_REG_EBP, SCRATCH + 512)
    uc.mem_write(SCRATCH + 512, struct.pack("<I", SCRATCH + 1024))
    uc.reg_write(UC_X86_REG_ESP, STACK_BASE + 0x80000)
    run_checked(uc, 0x7208D3, 0x7208F2, count=1000,
                required_addresses=[0x7208DC, 0x7208EC])
    seconds = struct.unpack("<f", uc.mem_read(SCRATCH + 1024 + 0x284, 4))[0]
    tag, channels, rate, average, alignment, bits = struct.unpack_from("<HHIIHH", fmt)
    result = dict(kind=kind, name=name, header_hex=header.hex(),
                  format_tag=tag, channels=channels, sample_rate=rate,
                  header_average_bytes_per_second=average, block_align=alignment,
                  bits_per_sample=bits, data_bytes=data_bytes,
                  native_bytes_per_second=descriptor[5], milliseconds=milliseconds,
                  seconds=int(seconds), seconds_f32_hex=struct.pack("<f", seconds).hex())
    if sha256 is not None:
        result["archive"] = "thememd.mix"
        result["wav_sha256"] = sha256
    return result


def retail_metadata():
    archive = mix((configured_gamemd().parent / "thememd.mix").read_bytes())
    result = []
    for stem in STEMS:
        data = archive[mix_hash(stem + ".wav")]
        if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
            raise RuntimeError("Unexpected retail WAV header")
        offset, fmt = 12, None
        while offset + 8 <= len(data):
            tag, size = struct.unpack_from("<4sI", data, offset)
            if tag == b"fmt ":
                fmt = data[offset + 8:offset + 8 + size]
            elif tag == b"data":
                if fmt is None or offset + 8 + size > len(data):
                    raise RuntimeError("Unexpected retail WAV chunk order/length")
                result.append(native_metadata("retail", stem, data[:offset + 8],
                                             fmt, size, hashlib.sha256(data).hexdigest()))
                break
            offset += 8 + ((size + 1) & ~1)
        else:
            raise RuntimeError("Retail WAV has no data chunk")
    return result


def current_song_cases():
    cases = []
    for active, retained, pending in ((5, 6, 7), (5, -1, 7), (-1, -1, -3), (-1, -1, -1)):
        result = dict(active=active, retained=retained, pending=pending)
        for label, begin, end in (("launcher", 0x55FBE5, 0x55FBF4),
                                  ("sound", 0x6B6918, 0x6B6927)):
            uc = emulator()
            uc.mem_write(0xA83D10, struct.pack("<3i", active, retained, pending))
            run_checked(uc, begin, end, count=20)
            result[label] = struct.unpack("<i", struct.pack("<I", uc.reg_read(UC_X86_REG_EAX)))[0]
        cases.append(result)
    return cases


def generate():
    cases = retail_metadata()
    for rate in (22050, 44100):
        for channels in (1, 2):
            for bits in (8, 16):
                alignment = channels * bits // 8
                average = rate * alignment
                fmt = struct.pack("<HHIIHH", 1, channels, rate, average, alignment, bits)
                for delta in (-1, 0):
                    size = average * 60 + delta
                    header = (b"RIFF" + struct.pack("<I", size + 36) + b"WAVEfmt "
                              + struct.pack("<I", 16) + fmt + b"data" + struct.pack("<I", size))
                    name = f"pcm_{rate}_{channels}ch_{bits}bit_60sec{delta:+d}byte"
                    cases.append(native_metadata("pcm", name, header, fmt, size))
    if len(cases) != 24:
        raise RuntimeError("Incomplete ordinary metadata cases")
    return dict(source="unicorn/gamemd.exe", cases=cases,
                current_song_cases=current_song_cases())


if __name__ == "__main__":
    finish_vectors(generate, Path(__file__).with_suffix(".json"),
                   provenance=lambda: provenance(
        scope="Eight original YR music WAV metadata inputs, sixteen PCM seconds-boundary cases, four retained-or-pending cases for each original caller",
        assumptions=[
            "Retail inputs explicitly thememd.mix; mount precedence independently inspected with asset browser, not emulated",
            "Python supplies fmt pointer ESI, zeroed descriptor EBX, declared data length EDX, valid stack and destination; original RIFF file I/O/chunk walk not executed",
            "PCM positive mono/stereo 8/16-bit at22050/44100Hz; bytes immediately before/at60seconds, no overflow/malformed/zero-rate scope",
            "Original duration helper and its CRT arithmetic execute completely; descriptor rate and whole-second store execute bounded original blocks",
            "Supplied x87 control027F; scoped stored integer seconds are exactly representable and independent of rounding",
            "Current-song comparisons supply three global slots and stop before Queue; no playback/output or complete dialog execution",
        ],
        substitutions=["File I/O/chunk parsing and allocations replaced by supplied original fmt/data metadata and writable scratch; no native instructions patched"],
        entry_points={"catalog": 0x7207F0, "descriptor": 0x408751,
                      "milliseconds": 0x408560, "seconds": 0x7208D3,
                      "launcher_current": 0x55FBE5, "sound_current": 0x6B6918},
    ))
