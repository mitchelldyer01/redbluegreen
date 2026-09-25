# Status

## Position

Unit 6: done. `rbg matmul M N K [--reps R]` computes C = A x B
with kernels/matmul (naive: one work-item per element, workgroup
256,1,1, packet setup 2, a loop over K), checks C on the CPU,
and prints the time and GFLOP/s. Every buffer and one signal per
rep is allocated once at start (lessons rule 4). Driver review
fixed two kernel bugs before the first run; see below.

## Verified on this machine, 2026-09-25 (real output)

1. fmt, clippy, doclint clean. tools/asm matmul: 188 bytes of
   code, kd 64 bytes. Kernel fixes before any run: A offset was
   (row + t) * 4, now (row * K + t) * 4; store had address and
   data swapped. Host side reviewed, ten checks, all confirmed.
2. GPU runs, server stopped, target/release/rbg, best of 5:
   `matmul 1 1 1`: 158 us, verify: 1 ok.
   `matmul 16 16 16`: 162 us, verify: 256 ok.
   `matmul 1024 1024 1024`: 6482 us, 331 GFLOP/s,
   verify: 1048576 ok.
   `matmul 4096 4096 4096 --wait 60`: 1123728 us, 122 GFLOP/s,
   verify: 262144 ok (64 sampled rows).
3. `journalctl -k --since "30 min ago" | grep -c "GPU reset("`
   printed 0.

## Stale

None.

## Exact next step

1. Unit 6b: the tiled matmul, 16 x 16 LDS tiles with a barrier,
   checked against `rbg matmul` on the GPU. Prompt not yet
   written; write it under docs/prompts/ before starting.
2. The naive kernel drops from 331 to 122 GFLOP/s between 1024
   and 4096; the working set leaves the cache. The tiled kernel
   is the fix; measure both at both sizes.
3. Open from unit 5: 768 MiB of fine (COHERENT) data hangs
   before its first store, no fault logged.
