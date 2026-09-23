# What the hardware taught us

## Why

The kernel accepts many things the hardware then refuses, and it
refuses by hanging, which costs a GPU reset. This page keeps every
such fact with the date it was paid for. Read it before any
kernel or queue change. docs/queue.md and docs/dispatch.md hold
the mechanics; this page holds the rules.

## Rules, each bought with at least one reset (2026-09-22)

1. EOP buffer. Optional in the ioctl, required by the gfx11 MQD.
   A queue without one hangs MES on REMOVE_QUEUE. 4 KiB, coarse.
2. COHERENT on every buffer the CP touches: ring, both pointers,
   kernel object, kernarg, signal. Without it a kernel runs but
   the CP never sees completion. EOP and CWSR stay coarse.
3. Registers. Ask the assembler for 8 more VGPRs than the code
   uses. vadd with `next_free_vgpr 5` hung on a load into v4; the
   same bytes with `next_free_vgpr 16` ran. Block 0 does not give
   a wave five usable registers, whatever the encoding says.
4. Do not map new GPU memory while another process is busy on
   the GPU. Page tables live in the 512 MiB VRAM carve-out here
   (amdgpu_vm_pt.c), and the desktop keeps it 90 percent full. A
   fresh mapping allocates page tables there, TTM evicts another
   process's buffer, that pauses its queues, and MES cannot pause
   a mid-kernel queue in time: GPU reset, both processes dead.
   Our own queues survive pauses (12 processes; a queue that ran
   through a whole server generation). Rule: allocate everything
   at start, never inside a loop, never with the server busy.
5. One run first. Five identical runs of a new kernel cost five
   resets and taught the same thing once.

## Facts that cost nothing but are easy to get wrong

- The CP polls the write pointer when memory is coherent; the
  doorbell is the contract, not the only trigger.
- The doorbell takes the packet's index, not its ring slot.
- The 128-bit scalar load, the exec-mask sequence, and the
  vector load path all work as the ISA says; each was proven
  alone with a bisect kernel under src/kernels/.

## Measured (vadd2, server stopped)

| elements | coarse | fine (COHERENT) |
|---|---|---|
| 1 | 133 us | |
| 1M | 151 us, 83 GB/s | |
| 16M | 924 us, 218 GB/s | 906 us, 222 GB/s |
| 64M | 3578 us, 225 GB/s | hangs, open item |

Theoretical is 256 GB/s. The 133 us is the floor for any dispatch
and sets the batch size david must use. Fine memory costs nothing
on streaming loads here, but 768 MiB of it hangs the wave before
its first store, with no fault logged. Data stays coarse.

## What next

Unit 6: the first matmul, with every buffer allocated once at
start. Open: 768 MiB of fine data hangs before its first store.
