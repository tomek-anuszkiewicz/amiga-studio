#!/usr/bin/env python3
"""
iff_to_png.py - Amiga IFF ILBM Image to PNG Converter

Converts Commodore Amiga IFF (Interchange File Format) ILBM (InterLeaved BitMap)
graphics files (standard indexed 1-8 bitplanes, EHB, and HAM) to standard PNG format.
"""

import sys
import struct
import argparse
from pathlib import Path
from typing import Tuple, List, Optional

try:
    from PIL import Image
except ImportError:
    Image = None

def decompress_byterun1(data: bytes, expected_length: int) -> bytes:
    """
    Decompresses Amiga ByteRun1 RLE data.
    """
    out = bytearray()
    i = 0
    n_data = len(data)
    
    while i < n_data and len(out) < expected_length:
        b = data[i]
        i += 1
        
        if b <= 127:
            # Copy next b + 1 bytes literally
            count = b + 1
            out.extend(data[i:i + count])
            i += count
        elif b >= 129:
            # Repeat next byte (257 - b) times
            count = 257 - b
            if i < n_data:
                val = data[i]
                i += 1
                out.extend([val] * count)
        else:
            # b == 128 (NOP)
            pass
            
    return bytes(out[:expected_length])

def parse_iff_ilbm(raw: bytes) -> Tuple[dict, List[Tuple[int, int, int]], bytes]:
    """
    Parses an IFF ILBM file and returns:
    - bmhd: dict containing width, height, planes, etc.
    - cmap: list of (R, G, B) tuples (0..255)
    - body: raw decompressed planar scanlines
    """
    if len(raw) < 12:
        raise ValueError("File too short to be an IFF ILBM file.")
        
    form_tag, total_size, file_type = struct.unpack(">4sI4s", raw[:12])
    if form_tag != b"FORM" or file_type not in (b"ILBM", b"PBM "):
        raise ValueError(f"Not a valid IFF ILBM/PBM file (found {form_tag} {file_type})")
        
    bmhd = {}
    cmap = []
    camg = 0
    body_data = b""
    
    idx = 12
    total_len = len(raw)
    
    while idx + 8 <= total_len:
        chunk_tag, chunk_len = struct.unpack(">4sI", raw[idx:idx + 8])
        idx += 8
        chunk_data = raw[idx:idx + chunk_len]
        # Chunks are padded to even byte offsets
        idx += (chunk_len + 1) & ~1
        
        if chunk_tag == b"BMHD":
            w, h, x, y, n_planes, masking, comp, pad, trans, x_aspect, y_aspect, page_w, page_h = struct.unpack(
                ">HHhhBBBBHBBhh", chunk_data[:20]
            )
            bmhd = {
                'w': w, 'h': h, 'x': x, 'y': y,
                'n_planes': n_planes,
                'masking': masking,
                'compression': comp,
                'trans_color': trans,
                'x_aspect': x_aspect,
                'y_aspect': y_aspect,
                'page_w': page_w,
                'page_h': page_h
            }
        elif chunk_tag == b"CMAP":
            num_colors = chunk_len // 3
            for c in range(num_colors):
                r = chunk_data[c * 3]
                g = chunk_data[c * 3 + 1]
                b = chunk_data[c * 3 + 2]
                cmap.append((r, g, b))
        elif chunk_tag == b"CAMG":
            if chunk_len >= 4:
                camg = struct.unpack(">I", chunk_data[:4])[0]
        elif chunk_tag == b"BODY":
            body_data = chunk_data
            
    if not bmhd:
        raise ValueError("BMHD chunk not found in IFF file.")
        
    # Unpack BODY if compressed
    w = bmhd['w']
    h = bmhd['h']
    planes = bmhd['n_planes']
    masking = bmhd['masking']
    # masking: 0 = none, 1 = has mask plane, 2 = transparent color, 3 = lasso
    plane_count = planes + (1 if masking == 1 else 0)
    bytes_per_row = ((w + 15) // 16) * 2
    total_body_len = bytes_per_row * plane_count * h
    
    if bmhd['compression'] == 1:
        decompressed_body = decompress_byterun1(body_data, total_body_len)
    else:
        decompressed_body = body_data[:total_body_len]
        
    bmhd['camg'] = camg
    bmhd['bytes_per_row'] = bytes_per_row
    bmhd['plane_count'] = plane_count
    
    # Generate default grayscale/system palette if CMAP is missing
    if not cmap:
        max_colors = 1 << planes
        for c in range(max_colors):
            val = int((c / max(max_colors - 1, 1)) * 255)
            cmap.append((val, val, val))
            
    return bmhd, cmap, decompressed_body

def planar_to_rgba(bmhd: dict, cmap: List[Tuple[int, int, int]], body: bytes) -> Tuple[int, int, bytes]:
    """
    Converts planar scanlines into 32-bit RGBA chunky pixels.
    """
    w = bmhd['w']
    h = bmhd['h']
    planes = bmhd['n_planes']
    masking = bmhd['masking']
    trans_color = bmhd.get('trans_color', -1)
    bytes_per_row = bmhd['bytes_per_row']
    plane_count = bmhd['plane_count']
    
    rgba = bytearray(w * h * 4)
    out_idx = 0
    
    # Check if Extra Half-Brite (EHB) mode
    # CAMG bit 7 (0x80) is EHB on standard OCS/ECS
    is_ehb = bool(bmhd.get('camg', 0) & 0x0080) or (planes == 6 and len(cmap) == 32)
    
    # Precompute EHB palette if applicable (32..63 are half brightness of 0..31)
    full_cmap = list(cmap)
    if is_ehb and len(full_cmap) >= 32:
        for i in range(32):
            r, g, b = full_cmap[i]
            full_cmap.append((r // 2, g // 2, b // 2))
            
    for y in range(h):
        line_offset = y * bytes_per_row * plane_count
        plane_offsets = [line_offset + p * bytes_per_row for p in range(planes)]
        mask_offset = line_offset + planes * bytes_per_row if masking == 1 else None
        
        for x in range(w):
            byte_idx = x >> 3
            bit_shift = 7 - (x & 7)
            
            color_index = 0
            for p in range(planes):
                p_byte = body[plane_offsets[p] + byte_idx]
                bit = (p_byte >> bit_shift) & 1
                color_index |= (bit << p)
                
            # Transparency calculation
            alpha = 255
            if masking == 1 and mask_offset is not None:
                m_byte = body[mask_offset + byte_idx]
                if not ((m_byte >> bit_shift) & 1):
                    alpha = 0
            elif masking == 2 and color_index == trans_color:
                alpha = 0
                
            if color_index < len(full_cmap):
                r, g, b = full_cmap[color_index]
            else:
                r, g, b = (0, 0, 0)
                
            rgba[out_idx] = r
            rgba[out_idx + 1] = g
            rgba[out_idx + 2] = b
            rgba[out_idx + 3] = alpha
            out_idx += 4
            
    return w, h, bytes(rgba)

def convert_iff_to_png(iff_path: Path, png_path: Path) -> None:
    if Image is None:
        raise RuntimeError("Pillow (PIL) library is required to save PNG images.")
        
    raw_bytes = iff_path.read_bytes()
    bmhd, cmap, body = parse_iff_ilbm(raw_bytes)
    w, h, rgba = planar_to_rgba(bmhd, cmap, body)
    
    img = Image.frombytes("RGBA", (w, h), rgba)
    
    # If aspect ratio is significantly non-square (e.g. PAL hires 2:1), optionally adjust
    # Amiga standard: xAspect=44, yAspect=52 (PAL hires: 22, 52 or 44, 44)
    png_path.parent.mkdir(parents=True, exist_ok=True)
    img.save(png_path, "PNG")
    print(f"Successfully converted {iff_path.name} ({w}x{h}, {bmhd['n_planes']} planes) -> {png_path}")

def main():
    parser = argparse.ArgumentParser(description="Convert Amiga IFF ILBM graphic files to PNG.")
    parser.add_argument("input", type=str, help="Path to input .iff file")
    parser.add_argument("output", type=str, help="Path to output .png file")
    args = parser.parse_args()
    
    in_path = Path(args.input)
    out_path = Path(args.output)
    
    if not in_path.exists():
        print(f"Error: Input file {in_path} does not exist.")
        sys.exit(1)
        
    convert_iff_to_png(in_path, out_path)

if __name__ == "__main__":
    main()
