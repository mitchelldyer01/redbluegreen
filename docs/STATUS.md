# Status

## Position

Unit 5b: done. The pause was never the fault. Creating GPU
mappings while another process is mid-kernel evicts that
process's buffers out of the 512 MiB VRAM carve-out (page tables
live there), which pauses its queues, which MES cannot do in
time; the driver then resets the GPU. Rule 4 in docs/lessons.md.
Kfd::new now calls RUNTIME_ENABLE as ROCr does; Queue::new fills
the CWSR header as libhsakmt does. Neither was the fix; both are
kept because ROCr does them. Bisect switches remain on `rbg add`
(--cwsr-exec --no-header --reknock --settle --wait --debug).

## Verified on this machine, 2026-09-22

1. Host checks clean; probe and mem unchanged.
2. Two rbg processes at once, 64M, 1000 reps each: both verify.
3. Twelve rbg processes at once, 16M, 300 reps each: all verify.
4. Server up and idle: 64M add verifies at 224 GB/s.
5. Add started first, server generates mid-run, 20000 reps: all
   verify, half speed while shared.
6. Queue created while the server generates: never runs, GPU
   reset two to three seconds in, kernel log names the server's
   queue as the one the driver failed to evict. Same with the
   header fill, with RUNTIME_ENABLE, and with 1-element buffers.
7. VRAM 468 of 512 MiB used, by alacritty and brave (fdinfo).
   Page tables go to VRAM on this chip (amdgpu_vm_pt.c v7.0).
   Reset log: 1 on Sep 21, 31 on Sep 22, none in 30 days before.

## Stale

None.

## Exact next step

1. Small cleanup unit: remove --cwsr-exec, --no-header,
   --reknock, --settle from `rbg add`; keep --wait and --debug.
2. Unit 6: matmul. Prompt not yet written.
