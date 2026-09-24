# Facts: the bisect

vadd hung on its first run: rptr 0, signal 1, c[0] 0. The packet
never retired and the kernel never reached its store. No fault in
the log. Every experiment cost one GPU reset, so each kernel was
built to answer one question. All run with one wave, one element,
the harness unchanged.

| kernel | what it isolates | result |
|---|---|---|
| store42 (wg 256) | 8 waves of a known-good kernel | pass |
| vadd (wg 1) | vadd code as one wave | hang |
| vadd (wg 1, fine data) | coarse vs fine data | hang |
| vload | one vector load, then store | pass |
| sload | vadd's 3 scalar loads incl. 128-bit | pass |
| sdump | dump s4..s7, s10, s2, v0 to memory | all correct |
| vbranch | vadd's exec-mask and branch, no loads | pass |
| vadd2 | vadd using v0..v3 only | pass |
| v4probe | vload with the value through v4 | pass |
| vadd16 | vadd bytes, descriptor asks 16 VGPRs | pass |

The only difference between vadd and vadd2 is two dwords: the
second load lands in v4 or in v0. Their descriptors are identical
because 4 and 5 registers encode to the same block (the assembler
encodes blocks of 8 for wave32). vadd16 is vadd's exact bytes with
the register field one block higher, and it runs.

Rule 3: ask the assembler for 8 more VGPRs than the code uses.
Block 0 does not give a wave five usable registers, whatever the
encoding says. v4probe passing alone means the failure needs a
second load in flight; the mechanism is not known, the rule is.

Method facts: sdump could not hang, so it ran first and showed
every register vadd depends on was correct (a, b, n, workgroup
id, lane id). The bisect tree was chosen to maximize information
per reset. On timeout the harness prints rptr, signal, and c[0]
so a hang still reports whether the packet retired and whether
the kernel stored.

Cost of this page: about eight resets.
