#!/usr/bin/env python3
"""One-shot helper to atomize nes_shm access (hotfix3 A-3 minimal)."""
import re
import sys
from pathlib import Path

VIDEO_FIELDS = (
    "ncol", "nrow", "pitch", "xscale", "yscale", "xyRatio",
    "preScaler", "test",
)

WRITE_RE = re.compile(
    r"nes_shm->video\.(" + "|".join(VIDEO_FIELDS) + r")\s*=\s*([^;]+);"
)
ASSIGN_PAIR_RE = re.compile(
    r"nes_shm->render_count\s*=\s*nes_shm->blit_count\s*=\s*0\s*;"
)
COUNT_WRITE_RE = re.compile(
    r"nes_shm->(render_count|blit_count)\s*=\s*([^;]+);"
)
COUNT_INC_RE = re.compile(
    r"nes_shm->(render_count|blit_count)\s*\+\+\s*;"
)


def transform(src: str) -> str:
    src = ASSIGN_PAIR_RE.sub(
        "nes_shm->render_count.store(0, std::memory_order_relaxed); "
        "nes_shm->blit_count.store(0, std::memory_order_relaxed);",
        src,
    )
    src = WRITE_RE.sub(
        lambda m: (
            f"nes_shm->video.{m.group(1)}.store({m.group(2).strip()}, "
            f"std::memory_order_release);"
        ),
        src,
    )
    src = COUNT_WRITE_RE.sub(
        lambda m: (
            f"nes_shm->{m.group(1)}.store({m.group(2).strip()}, "
            f"std::memory_order_relaxed);"
        ),
        src,
    )
    src = COUNT_INC_RE.sub(
        lambda m: f"nes_shm->{m.group(1)}.fetch_add(1, std::memory_order_relaxed);",
        src,
    )
    return src


def transform_reads(src: str) -> str:
    field_alt = "|".join(VIDEO_FIELDS)
    pattern = re.compile(
        r"nes_shm->video\.(" + field_alt + r")"
        r"(?!\s*\.\s*(?:store|load|fetch_add|fetch_sub))"
        r"(?!\s*=[^=])"
    )
    return pattern.sub(
        lambda m: f"nes_shm->video.{m.group(1)}.load(std::memory_order_acquire)",
        src,
    )


def process_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    new = transform_reads(transform(text))
    if new == text:
        return False
    path.write_text(new, encoding="utf-8")
    return True


def main() -> int:
    changed = []
    for arg in sys.argv[1:]:
        p = Path(arg)
        if process_file(p):
            changed.append(str(p))
    for c in changed:
        print(f"updated: {c}")
    return 0


if __name__ == "__main__":
    sys.exit(main())