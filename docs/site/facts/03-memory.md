# Facts: memory

One Buffer = one GPU allocation with a host view at the same
virtual address. Sequence (from libhsakmt fmm.c, verified by
unit 2):
1. Reserve VA: mmap(NULL, size, PROT_NONE,
   MAP_PRIVATE|MAP_ANONYMOUS|MAP_NORESERVE).
2. ALLOC_MEMORY_OF_GPU (nr 0x16, 40 bytes): va_addr = that
   address, size, gpu_id, flags. Returns a handle and an
   mmap_offset.
3. MAP_MEMORY_TO_GPU (nr 0x18): handle plus an array of gpu_ids.
4. Host view: mmap(va, size, PROT_READ|PROT_WRITE,
   MAP_SHARED|MAP_FIXED, render_fd, mmap_offset). MAP_FIXED
   replaces the reservation. CPU and GPU now share one address.
Teardown in reverse: munmap, UNMAP, FREE. A Drop does this.

Flags that matter (bit positions): GTT 1<<1, WRITABLE 1<<31,
EXECUTABLE 1<<30, COHERENT 1<<26.

COHERENT means fine-grained: the CP's own reads, atomics, and
completion path see the same bytes the CPU writes. Every buffer
the command processor touches must have it: ring, read and
write pointers, kernel object, kernarg, signal. Without it a
kernel ran but the CP never saw completion (unit 4).

Coarse (no COHERENT) is for data the kernels read and write.
Measured: streaming loads run at 222 GB/s fine and 225 GB/s
coarse, so the flag costs nothing here; data stays coarse because
768 MiB of fine data hangs before its first store (open item).

Verification of unit 2: fill 1 MiB with its own indices, munmap,
mmap again, read back; five runs; GTT accounting equal before
and after, which proves the frees. A forced verify failure still
tears down.

The kernel rule that bit later: every queue buffer must be the
exact start and exact size of its own allocation, so one big
buffer sliced up does not work for queue structures.
