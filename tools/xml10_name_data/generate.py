# SPDX-License-Identifier: GPL-2.0-only
"""Generate historical XML name flags from separately licensed libxml2 definitions"""
import hashlib
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
CLASSES = ('xmlIsBaseChar', 'xmlIsIdeographic', 'xmlIsCombining', 'xmlIsDigit', 'xmlIsExtender')


def ranges(text):
    if type(text) is not str or len(text) > 16384 or not text.isascii():
        raise ValueError('Bounded ASCII definitions required')
    result = {}
    active = None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith('name '):
            if active is not None:
                raise ValueError('Nested class')
            active = line[5:]
            if active in CLASSES:
                if active in result:
                    raise ValueError('Duplicate class')
                result[active] = []
        elif line.startswith('end'):
            if active is None or line not in ('end', 'end ' + active):
                raise ValueError('Mismatched class end')
            active = None
        elif active in CLASSES and line:
            match = re.fullmatch(r'ur (0x[0-9A-Fa-f]+)(?:\.\.(0x[0-9A-Fa-f]+))?', line)
            if match is None:
                raise ValueError('Selected class requires numeric ranges')
            low = int(match[1], 16)
            high = int(match[2], 16) if match[2] else low
            if not 0 <= low <= high <= 65535:
                raise ValueError('BMP range required')
            result[active].append((low, high))
    if active is not None or set(result) != set(CLASSES):
        raise ValueError('Incomplete selected definitions')
    for pairs in result.values():
        pairs.sort()
        if not pairs or len(pairs) > 256 or any(a[1] >= b[0] for a, b in zip(pairs, pairs[1:])):
            raise ValueError('Empty, excessive or overlapping ranges')
    return result


def generate(text):
    classes = {}
    for name, pairs in ranges(text).items():
        flags = bytearray(65536)
        for low, high in pairs:
            flags[low:high + 1] = bytes([1]) * (high - low + 1)
        classes[name] = flags
    output = bytearray(65536)
    for codepoint in range(65536):
        start = (classes['xmlIsBaseChar'][codepoint] or classes['xmlIsIdeographic'][codepoint]
                 or codepoint in (0x3a, 0x5f))
        rest = (start or classes['xmlIsCombining'][codepoint] or classes['xmlIsDigit'][codepoint]
                or classes['xmlIsExtender'][codepoint] or codepoint in (0x2d, 0x2e))
        output[codepoint] = int(bool(start)) | (int(bool(rest)) << 1)
    return bytes(output)


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit('Usage: generate.py OUTPUT')
    source = HERE / 'upstream/chvalid.def'
    with source.open('rb') as stream:
        body = stream.read(16385)
    if len(body) > 16384:
        raise ValueError('Definition input exceeds bound')
    candidate = generate(body.decode('ascii'))
    with Path(sys.argv[1]).open('xb') as stream:
        stream.write(candidate)
    print(hashlib.sha256(candidate).hexdigest())
