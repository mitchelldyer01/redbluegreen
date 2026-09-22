# Status

## Position

Unit 4: done. `rbg dispatch` runs kernels/store42 on one
workgroup: the target reads 42 and the completion signal reaches
0. The fix that made it work is COHERENT on every CP-visible
buffer, docs/dispatch.md. First code ran on this GPU 2026-09-22.

## Verified on this machine, 2026-09-22

1. By the agent: `tools/asm store42` byte-identical; `rbg probe`
   and `rbg mem` exit 0; fmt, clippy, doclint, 80 columns clean.
2. By a person, server stopped, after the COHERENT fix: five
   `rbg dispatch` runs each print `target[0]: 42`, `signal: 0`,
   `dispatch: ok`, exit 0. No reset, no flicker.
3. `rbg dispatch --no-doorbell` also prints 42, 0, ok, exit 0:
   the CP polls the write pointer. GTT 998191104 before,
   998203392 after: 12 KiB drift, no leak.
4. Before the fix, five runs: target 42 but signal 1,
   DESTROY_QUEUE timed out, GPU reset each time.

## Stale

None. One note: the unit 4 prompt says to cmp against
"the committed" kernel files, but kernels/, tools/asm and
src/kernels/ are still untracked from unit 3b.

## Exact next step

Unit 5: a kernel that matters. Elementwise add of two large
float buffers across many workgroups; measure bandwidth. Prompt
not yet written.
