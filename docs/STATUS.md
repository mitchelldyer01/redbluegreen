# Status

## Position

Unit 3b: done. `rbg queue` builds an AQL queue and a SIGNAL event
on top of `Kfd` and `Buffer`, prints their ids, and tears
everything down through Drop. Every kernel resource is a type
with Drop; every syscall returns Result. The EOP buffer fix for
the 2026-09-22 MES hangs is in, see docs/queue.md.

## Verified on this machine, 2026-09-22

1. Step 1, by the agent: `rbg probe` prints kfd version 1.22,
   gpu_id 3750, gfx 110501, exit 0. `rbg mem` verifies
   262144 values, exit 0. Both unchanged from unit 2.
2. Step 4, by the agent: `cargo fmt --check` passes,
   `cargo clippy --all-targets` has zero warnings, doclint
   passes, no line over 80 columns in docs or src.
3. Step 2, by a person, server stopped: five `rbg queue` runs
   each printed `queue_id: 0`,
   `doorbell_offset: 0xc3a9800000000000`, `event_id: 1`,
   `event_slot_index: 1`, `rptr: 0`, exit 0. Zero "GPU reset("
   in the kernel log over the window.
4. Step 3, by a person, server up: `rbg queue --cwsr-short`
   printed `rbg: ioctl CREATE_QUEUE: Invalid argument (os error
   22)`, exit 1. mem_info_gtt_used 35313598464 before,
   35313606656 after: 8 KiB of background drift, the same as
   five clean runs showed in unit 2. No leak.

## Stale

None. The create_queue field name is fixed in docs/queue.md.

## Exact next step

Unit 4: dispatch nothing. Assemble a kernel that writes one
constant to one address, dispatch one workgroup, wait on the
signal. Prompt not yet written. GPU steps stay with a person.
