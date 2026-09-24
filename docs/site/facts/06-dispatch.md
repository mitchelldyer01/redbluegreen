# Facts: dispatch

A dispatch is four objects and an order of writes.

Kernel object: one 4 KiB buffer, COHERENT|EXECUTABLE. Bytes 0..64
hold the 64-byte kernel descriptor the assembler built from
.amdhsa_ directives; code starts at byte 256. One field is
patched at load: the i64 at offset 16, kernel_code_entry_byte
_offset, set to 256 (the assembler leaves it as a relocation).
Hand-encoding the descriptor's register fields is where a wrong
bit becomes a reset, so the assembler does it.

Kernarg: a 4 KiB COHERENT buffer. The descriptor asks for the
kernarg pointer in s[0:1].

Signal: 64 bytes, zero-filled, kind 1 (user) at offset 0, value
at 8. Set to 1; the CP decrements to 0 on retire. Polled.

Packet, 64 bytes: header u16, setup u16, workgroup x y z u16,
reserved, grid x y z u32, private and group segment sizes u32,
kernel_object u64, kernarg_address u64, reserved u64,
completion_signal u64. Header 0x1502 = KERNEL_DISPATCH | barrier
| acquire system | release system. Setup 1 = one dimension.

Order (ROCr does exactly this): body first; header last as one
u32 store after a release fence; write pointer = index + 1; then
the doorbell = the packet's own index, not index + 1 and not the
ring slot. The doorbell write is the one place in the program
that starts hardware; it is commented as such.

The first kernel, store42, is nine instructions: load the target
address from the kernarg pointer, store 42, wait for the store,
end. 36 bytes of code. Assembled by tools/asm (llvm-mc, then
llvm-objcopy extracts .text and the 64-byte .rodata descriptor).
The bytes are committed, so a clone without LLVM still builds.

Result: target[0] became 42 on the first run. The signal did not
reach 0. That is the next page.
