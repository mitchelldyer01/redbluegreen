# Facts: the queue

An AQL compute queue is five buffers and one ioctl.
- ring: 64 KiB, power of two, COHERENT|EXECUTABLE. Holds 64-byte
  packets. Every slot starts with header 1 (INVALID); the CP
  waits on an INVALID header and never runs one.
- write pointer: its own 4 KiB buffer, COHERENT. The kernel maps
  it into GART so the scheduler can poll it.
- read pointer: its own 4 KiB buffer, COHERENT. Hardware advances
  it as packets retire.
- CWSR area: 18 MiB here, coarse. Required. Size from sysfs
  cwsr_size plus a debug region; the kernel rejects a smaller
  size or a different ctl_stack_size. Its header is filled as
  libhsakmt fills it (debug offset and size, error payload
  address, event id).
- EOP buffer: 4 KiB, coarse, EXECUTABLE. Optional to the ioctl,
  required by the gfx11 MQD code, which programs it with no zero
  check. Without it the first REMOVE_QUEUE hung MES and reset
  the GPU, four times.

Kernel rule (kfd_queue.c, since 6.11): each buffer address must
be the start of its own allocation, and the allocation size must
equal the expected size exactly.

CREATE_QUEUE (nr 0x02, 96 bytes) returns queue_id and a
doorbell_offset. The high bits are a page offset, the low 13 bits
this queue's byte offset within an 8 KiB doorbell page mmapped
on the kfd fd. Writing a packet index to that u64 starts the
hardware. The CP also polls the write pointer when memory is
coherent, so a packet runs even without the doorbell; ring it
anyway, it is the contract.

Event: CREATE_EVENT (nr 0x08) returns an event id and a slot
index in a 32 KiB event page mmapped on the kfd fd. Not used for
waiting yet; completion is polled.

Failure path verified: a CWSR area 4 KiB too small is rejected
with EINVAL before any hardware queue exists, and GTT usage is
equal before and after, proving every Drop ran on the error path.

Field rename: the last CREATE_QUEUE field is pad in the 6.19
header on disk and metadata_ring_size in the running 7.0 kernel.
Zero is correct either way. The local model found this and
reported it as stale rather than editing the doc, as instructed.
