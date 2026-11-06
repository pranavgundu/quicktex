"""Regenerate the compact English trie and compressed built-in glyph outlines.
Run from any directory with Python's standard library. Source licenses stay in
hyph-en-us.tex and the existing glyph metadata/license sections.
"""
from pathlib import Path
import re
import argparse
import struct
import zlib

parser = argparse.ArgumentParser()
parser.add_argument('--chunk-bytes', type=int, default=262144)
parser.add_argument('--first-chunk-bytes', type=int)
args = parser.parse_args()
first_chunk = args.first_chunk_bytes if args.first_chunk_bytes is not None else min(65536, args.chunk_bytes)
if args.chunk_bytes <= 0 or first_chunk <= 0:
    parser.error('chunk byte targets must be positive')

root = Path(__file__).resolve().parent
source = '\n'.join(line.split('%')[0] for line in (root / 'hyph-en-us.tex').read_text().splitlines())
patterns = re.search(r'\\patterns\{(.*?)\}', source, re.S)[1].split()
nodes = [{'children': {}, 'values': b''}]
for pattern in patterns:
    letters, values = [], [0]
    for char in pattern:
        if char.isdigit():
            values[-1] = int(char)
        else:
            letters.append(char)
            values.append(0)
    index = 0
    for char in letters:
        if char not in nodes[index]['children']:
            nodes[index]['children'][char] = len(nodes)
            nodes.append({'children': {}, 'values': b''})
        index = nodes[index]['children'][char]
    nodes[index]['values'] = bytes(values)
records, weights, offsets = [], bytearray(), {b'': 0}
for node in nodes:
    children = sorted(node['children'].items())
    values = node['values']
    if values not in offsets:
        offsets[values] = len(weights)
        weights.extend(values)
    records.append([children[0][1] if children else 0, 0, offsets[values], 0, len(values)])
for node in nodes:
    children = sorted(node['children'].items())
    for at, (char, index) in enumerate(children):
        records[index][1] = children[at + 1][1] if at + 1 < len(children) else 0
        records[index][3] = ord(char)
assert len(nodes) < 65536 and len(weights) < 65536
output = struct.pack('<I', len(nodes)) + b''.join(struct.pack('<HHHBB', *record) for record in records) + weights
(root / 'hyph-en-us.bin').write_bytes(output)
glyphs = root.parent / 'src' / 'glyphs.bin'
# Keep independently decodable outline blocks. Small documents only open the
# blocks containing their glyphs, avoiding a full-font inflate at startup.
# Keep first-use text inflation small; larger remaining blocks share outlines.
metadata = (root.parent / 'src' / 'data.rs').read_text()
spans = sorted(set((int(a), int(b)) for a, b in re.findall(r'Glyph\{off:(\d+),len:(\d+)', metadata) if int(b)))
boundaries = [0]
covered = 0
for offset, length in spans:
    target = first_chunk if len(boundaries) == 1 else args.chunk_bytes
    if offset >= covered and covered - boundaries[-1] >= target:
        boundaries.append(covered)
    covered = max(covered, offset + length)
boundaries.append(len(glyphs.read_bytes()))
raw = glyphs.read_bytes()
packed, blocks = bytearray(), []
for start, end in zip(boundaries, boundaries[1:]):
    # The outlines contain small numeric deltas. FILTERED avoids spending
    # matches on noisy short runs; the zlib format and block boundaries stay
    # unchanged, and every original outline byte must survive the round trip.
    compressor = zlib.compressobj(9, zlib.DEFLATED, zlib.MAX_WBITS, 9, zlib.Z_FILTERED)
    block = compressor.compress(raw[start:end]) + compressor.flush()
    assert zlib.decompress(block) == raw[start:end]
    blocks.append((start, len(packed), len(block)))
    packed.extend(block)
for offset, length in spans:
    assert any(start <= offset and offset + length <= end for start, end in zip(boundaries, boundaries[1:]))
index = []
for offset in range(0, len(raw), 1024):
    index.append(max(i for i, start in enumerate(boundaries[:-1]) if start <= offset))
if len(blocks) > 256:
    parser.error('--chunk-bytes produces too many blocks for the u8 path index')
(root.parent / 'src' / 'glyphs.z').write_bytes(packed)
# Emit ordinary Rust literals for reproducibility without another generator dependency.
generated = '#[rustfmt::skip]\npub const PATH_CHUNKS: &[(usize,usize,usize)] = &[' + ','.join(f'({a},{b},{c})' for a,b,c in blocks) + '];\n'
generated += '#[rustfmt::skip]\npub const PATH_INDEX: &[u8] = &[' + ','.join(map(str,index)) + '];\n'
(root.parent / 'src' / 'glyph_data.rs').write_text(generated)
