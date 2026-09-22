# The dispatch

## Why

docs/queue.md builds the queue. This page holds the bytes that
make the hardware run one kernel: the packet, the descriptor, the
signal, and the doorbell. Sources: hsa.h and amd_hsa_signal.h in
the TheRock tree, AMDHSAKernelDescriptor.h, ROCr amd_aql_queue.cpp.

## Memory type: every buffer the CP touches is COHERENT (1<<26)

Ring, pointers, kernel object, kernarg, signal, target. ROCr
allocates all of these fine-grained. Without it the kernel ran
but the CP never saw completion: signal stayed 1, DESTROY_QUEUE
timed out, GPU reset (2026-09-22). EOP and CWSR stay coarse.

## The kernel object

One Buffer with flags GTT|WRITABLE|EXECUTABLE|COHERENT. Bytes 0..64 hold
the kernel descriptor from kernels/NAME.kd. The code from
kernels/NAME.text starts at byte 256. Patch the descriptor's i64
at offset 16 (kernel_code_entry_byte_offset) to 256: it is a
relocation the assembler leaves at zero. kernel_object in the
packet is the Buffer's va. tools/asm makes both files.

## The kernarg segment

A Buffer of 4 KiB. Bytes 0..8 hold the u64 address the kernel
writes to. The descriptor asks for the kernarg pointer in
s[0:1]; store42 loads that u64 and stores 42 through it.

## The signal, 64 bytes, in its own Buffer, zero-filled

kind i64 = 1 (AMD_SIGNAL_KIND_USER) at 0, value i64 at 8,
event_mailbox_ptr u64 at 16 (0: no event, we poll), event_id
u32 at 24, then timestamps and reserved. Set value to 1 before
the dispatch. The CP decrements it to 0 when the packet retires.

## The packet, 64 bytes, at ring slot index % (ring_size / 64)

header u16, setup u16, workgroup_size x y z u16, reserved u16,
grid_size x y z u32, private_segment_size u32,
group_segment_size u32, kernel_object u64, kernarg_address u64,
reserved u64, completion_signal u64 (the signal Buffer's va).
header = 2 (KERNEL_DISPATCH) | 1<<8 (barrier) | 2<<9 (acquire
system) | 2<<11 (release system) = 0x1502. setup = 1 (one
dimension). workgroup 1,1,1; grid 1,1,1; both segment sizes 0.

## The order of writes (ROCr does exactly this)

1. Before the first dispatch, write header 1 (INVALID) into every
   slot. The CP waits on an INVALID header; it never runs one.
2. Write the packet body. Write the header last, as one u32
   store (header | setup << 16), after a fence.
3. Store index + 1 into the write-pointer Buffer as u64.
4. Store index (the packet's own index, not index + 1) into the
   doorbell u64, the AQL rule on gfx9+. The CP also polls the
   write pointer: with coherent memory a packet runs with no
   doorbell (measured 2026-09-22). Ring it anyway.
5. Poll the signal value with a 5 s timeout. 0 means done.
