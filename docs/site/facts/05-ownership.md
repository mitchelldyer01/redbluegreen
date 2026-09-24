# Facts: ownership

Rust's Drop runs when a value goes out of scope. Every kernel
resource is a type with Drop, so teardown order is a property of
the types, not a discipline of the caller.

Tree, outermost first:
- Kfd: the two fds and the gpu_id. Drop closes both.
- Buffer: va, size, handle, flags, a view_mapped bit. Drop:
  munmap, UNMAP, FREE. On error inside Drop, print and go on;
  Drop cannot fail. as_slice_mut borrows through the Buffer, so a
  slice cannot outlive it. remap drops and retakes the view.
- Queue: owns ring, write_ptr, read_ptr, cwsr, eop Buffers and
  the doorbell mapping. Drop: DESTROY_QUEUE, munmap the doorbell,
  then the Buffers drop in declaration order.
- Event: event id and the event page. Drop: munmap, DESTROY.
- Kernel: a Buffer holding descriptor plus code.
- Signal: a 64-byte struct in its own Buffer.

Every syscall returns Result<T, Error>; Error carries the call
name and errno and prints as "call: message". main prints it and
exits 1. There is no die().

Why it mattered: unit 2's flat functions leaked a handle on any
early exit until the process ended. A leak of 1 MiB is nothing; a
training run keeps a corpus of gigabytes resident, and a GPU hang
on an APU takes the desktop with it. Ownership is the cheapest
guard, and the reason Rust was chosen over C.

Where it was tested: the too-small CWSR run (EINVAL from the
kernel, exit 1, GTT unchanged) and the forced verify failure in
unit 2. Both prove Drop ran on the error path.

Nesting gives order for free: a Queue drops its own fields first,
then its Buffers, so DESTROY_QUEUE always runs before the ring's
memory is freed.

Theme the owner named: "thematic approaches", ownership as the
unit's theme rather than a feature bolted on.
