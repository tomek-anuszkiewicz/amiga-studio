"""Build the mixed scene's 880 KiB bootable floppy image."""

import argparse
from pathlib import Path
import struct
import subprocess
import tempfile


SCENE = Path(__file__).resolve().parent
BOOT_BLOCK_SIZE = 1024
PAYLOAD_OFFSET = 0x500
PAYLOAD_SIZE = 1268
ADF_SIZE = 80 * 2 * 11 * 512
LONG_MASK = 0xFFFFFFFF


def boot_sum(block):
    """Return the Amiga boot-block end-around-carry sum."""
    total = 0
    for (word,) in struct.iter_unpack(">I", block):
        total += word
        total = (total & LONG_MASK) + (total >> 32)
    return total


def make_image(assembler):
    with tempfile.TemporaryDirectory() as temp:
        boot_binary = Path(temp) / "boot.bin"
        subprocess.run(
            [assembler, SCENE / "boot.as", "-o", boot_binary],
            check=True,
        )
        boot = boot_binary.read_bytes()
    payload = (SCENE / "program.bin").read_bytes()
    if len(boot) != PAYLOAD_OFFSET or boot[:8] != b"DOS\0\0\0\0\0":
        raise ValueError("Unexpected boot block or continuation layout")
    if len(payload) != PAYLOAD_SIZE:
        raise ValueError(f"Expected a {PAYLOAD_SIZE}-byte mixed-scene program")

    image = bytearray(ADF_SIZE)
    image[:PAYLOAD_OFFSET] = boot
    image[PAYLOAD_OFFSET : PAYLOAD_OFFSET + PAYLOAD_SIZE] = payload
    checksum = LONG_MASK ^ boot_sum(image[:BOOT_BLOCK_SIZE])
    struct.pack_into(">I", image, 4, checksum)
    return image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembler", required=True, type=Path)
    parser.add_argument("--output", type=Path, default=SCENE / "mixed_scene.adf")
    args = parser.parse_args()
    image = make_image(args.assembler)
    args.output.write_bytes(image)


if __name__ == "__main__":
    main()
