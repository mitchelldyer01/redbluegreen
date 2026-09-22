# Status

## Position

Unit 2 of docs/roadmap.md is done. The binary `rbg` has the
subcommands `probe` and `mem`. The ioctl plumbing moved to
src/kfd.rs: open, ioctl, mmap and munmap are declared there, and
the ioctl structs and call wrappers live there. `rbg mem`
reserves a 1 MiB VA, ALLOCs a GTT buffer at that VA, maps it to
the GPU, mmaps the host view at the same VA, writes a u32 index
pattern, munmaps, mmaps again, and checks every value. Teardown
(munmap, UNMAP, FREE) runs even on a verify failure.

## Verified on this machine, 2026-09-21

1. `cargo fmt --check` and `cargo clippy --all-targets` (zero
   warnings) pass. `tools/doclint` and `awk 'length > 80'` over
   src/ pass.
2. `rbg probe` exits 0 and prints kfd version 1.22, gpu_id 3750,
   gfx_target_version 110501. `rbg probe --acquire` adds
   `acquire_vm: ok`.
3. `rbg mem` exits 0 and prints, for example:

```
handle: 16106127360000
mmap_offset: 5190475776
va: 0x7feb99027000
verify: 262144 ok
```

4. `rbg mem` five times in a row: all five exit 0.
5. With one expected value broken, it prints `verify: 1 of
   262144 values wrong`, exit 1, and the teardown still ran.
   The change is reverted.

## Stale

None. The host-view fd finding is now in docs/floor.md.

## Exact next step

Unit 3: build a queue. Allocate the ring, create the AQL compute
queue, map the doorbell page and the event signal page.
