#!/usr/bin/env python3
"""Minimal `strings -n N` replacement (ASCII + UTF-16LE), with file offsets."""
import sys


def runs(data, minimum):
    out = []
    i = 0
    n = len(data)
    while i < n:
        b = data[i]
        if 32 <= b < 127 or b in (9,):
            start = i
            while i < n and (32 <= data[i] < 127 or data[i] in (9,)):
                i += 1
            if i - start >= minimum:
                out.append((start, data[start:i].decode("latin-1")))
        else:
            i += 1
    return out


def utf16_runs(data, minimum):
    out = []
    i = 0
    n = len(data)
    while i + 1 < n:
        lo, hi = data[i], data[i + 1]
        if hi == 0 and (32 <= lo < 127):
            start = i
            chars = []
            while i + 1 < n and data[i + 1] == 0 and 32 <= data[i] < 127:
                chars.append(chr(data[i]))
                i += 2
            if len(chars) >= minimum:
                out.append((start, "".join(chars)))
        else:
            i += 2
    return out


def main():
    if len(sys.argv) < 3:
        print("usage: strings_extract.py <binary> <output.txt> [min]", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    minimum = int(sys.argv[3]) if len(sys.argv) > 3 else 6
    data = open(src, "rb").read()
    with open(dst, "w", encoding="utf-8", errors="replace") as f:
        f.write(f"===== {src} (ASCII, min={minimum}) =====\n")
        for off, s in runs(data, minimum):
            f.write(f"{off:08x}  {s}\n")
        f.write(f"\n===== {src} (UTF-16LE, min={minimum}) =====\n")
        for off, s in utf16_runs(data, minimum):
            f.write(f"{off:08x}  {s}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
