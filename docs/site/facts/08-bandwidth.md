# Facts: bandwidth

Kernel vadd: c[i] = a[i] + b[i], one work-item per element,
workgroup 256, 16 instructions, bounds check on n. kernarg: a, b,
c pointers and n. Grid = ceil(n / 256) * 256.

Timing: from just before the doorbell write to signal 0, best of
five reps, fresh signal per rep, buffers reused. Bandwidth = 12
bytes per element (two reads, one write) over the best time.

Measured 2026-09-22, model server stopped, coarse data:

| elements | best time | GB/s |
|---|---|---|
| 1 | 133 us | launch floor |
| 1,048,576 | 151 us | 83 |
| 16,777,216 | 924 us | 218 |
| 67,108,864 | 3,578 us | 225 |

Fine (COHERENT) data: 16,777,216 elements, 906 us, 222 GB/s. So
the flag costs nothing on streaming loads on this APU. 67M fine
elements (768 MiB) hangs before the first store, no fault logged;
open item; data stays coarse.

Theoretical bandwidth: 256 GB/s. Achieved: 88 percent.

The launch floor, 133 us, is the number david's batch size must
amortize: a training step must move enough bytes that 133 us is
small against it. At 225 GB/s, 133 us is about 30 MB.

Sharing: with two rbg processes at once the best times summed to
roughly the single-process bandwidth; with a model server
generating alongside, our reps ran at about half speed and all
verified. The hardware runs queues side by side; it does not
starve one for another.

Earlier docs guessed "about 256 GB/s" and "a few TFLOP/s". The
first is now measured. The second waits for a matmul (unit 6).
