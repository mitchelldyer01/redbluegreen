# Status

## Position

Unit 5c: done. The 5b experiment knobs are out: --cwsr-exec,
--no-header, --reknock and --settle, with the cwsr_exec and
fill_header queue options and Queue::reknock. The CWSR header
is always filled and the CWSR buffer is always coarse; Kfd::new
still calls RUNTIME_ENABLE and both keep their comments.
--kernel takes vadd (default) or vadd-v4hang; the other bisect
names are gone, their kernel files stay on disk.
src/kernels/vadd.s is now vadd-v4hang.s, vadd2.s is now vadd.s
with its symbol renamed. docs/lessons.md rule 3 and the
Measured heading use the new names. birdseye needed no change:
it names only kernels/vadd and kernels/store42, both unchanged
in name.

## Verified on this machine, 2026-09-23 (real output)

1. cargo fmt --check: clean. cargo clippy --all-targets: zero
   warnings. No line over 80 columns in the changed files.
   tools/doclint docs/*.md: clean.
2. The step 2 grep over src/ and docs/ matches only the
   historical prompt files (unit-1-fixes, unit-5b, unit-5c)
   and this status page; nothing in src/ or the other docs.
   The spec's "prints nothing" is not literally reachable: the
   prompt files are history.
3. tools/asm vadd and tools/asm vadd-v4hang: "92 bytes" and
   "64 bytes" each; all four byte files cmp-identical to the
   renamed originals.
4. rbg add 0 and rbg add 0 --wait 9 --debug: both print
   "rbg: add: N, reps, wg out of range", exit 1. rbg probe:
   "kfd version: 1.22", "gpu_id: 3750",
   "gfx_target_version: 110501", exit 0. rbg mem:
   "verify: 262144 ok", exit 0.
5. ls kernels/: vadd.kd/.text and vadd-v4hang.kd/.text, no
   vadd2 files; src/kernels/ matches.

## Stale

None.

## Exact next step

1. Unit 5c is finished; nothing remains in it.
2. Unit 6: the first matmul, every buffer allocated once at
   start (docs/lessons.md rule 4). Prompt not yet written;
   write it under docs/prompts/ before starting.
3. Open from the Measured table: 768 MiB of fine (COHERENT)
   data hangs before its first store, no fault logged.
