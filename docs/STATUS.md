# Status

## Position

Unit 1 of docs/roadmap.md is done, review fixes applied. The binary
`rbg` takes the subcommand `probe` as its first argument. It opens
/dev/kfd and /dev/dri/renderD128, calls GET_VERSION, and on request
calls ACQUIRE_VM. Any other first argument prints the usage and
exits 2. No crates, no unsafe beyond the hand-declared ioctl and
open, which now use c_char. Code: src/main.rs.

## Verified on this machine, 2026-09-21

1. `cargo fmt --check` prints nothing. `cargo clippy --all-targets`
   ends with no warnings. `awk 'length > 80' src/main.rs` prints
   nothing.
2. `./target/debug/rbg probe` exits 0 and prints:

```
kfd version: 1.22
gpu_id: 3750
gfx_target_version: 110501
```

3. `./target/debug/rbg probe --acquire` adds `acquire_vm: ok` and
   exits 0.
4. `./target/debug/rbg --acquire probe` prints the usage line and
   exits 2.
5. `tools/doclint docs/*.md docs/prompts/*.md` exits 0.

`target` is a local link to the shared build dir set in
~/.cargo/config.toml, so the paths above work as written.

## Stale

None.

## Exact next step

Unit 2: own memory. Add `rbg mem` that ALLOCs a GTT buffer,
MAP_MEMORY_TO_GPU, mmaps the host view, writes a pattern, reads it
back. Verify the pattern survives a munmap and a fresh mmap.
