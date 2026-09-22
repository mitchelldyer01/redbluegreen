# Status

## Position

Unit 5: done. `rbg add` measures 225 GB/s at 64M elements with
coarse data, 222 GB/s at 16M with fine data. docs/lessons.md has
the table, the three rules the unit paid for, and two open items:
the queue does not survive a pause, and 64M of fine data hangs.

## Verified on this machine, 2026-09-22, server stopped

1. Host checks clean; `rbg probe`, `rbg mem` unchanged.
2. `rbg add 1`, `1048576`, `16777216`, `67108864`: all verify,
   exit 0. Best times and GB/s in docs/lessons.md.
3. Bisect kernels store42, vload, sload, sdump, vbranch, v4probe,
   vadd2, vadd16 all verify at N = 1. vadd (v4, block 0) hangs.
4. `rbg add 16777216 --fine`: verify, 906 us, 222 GB/s.
   `rbg add 67108864 --fine`: hangs, server stopped, no fault.

## Stale

None.

## Exact next step

Unit 5b: survive a pause. Prompt: docs/prompts/unit-5b.md.
