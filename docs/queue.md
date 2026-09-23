# The queue

## Why

docs/floor.md names the ioctls. This page holds the rules the
kernel enforces when a queue is built, read from kfd_queue.c and
kfd_chardev.c at v7.0, and from libhsakmt queues.c and events.c.

## Buffers the kernel checks

Every queue buffer must be a BO from ALLOC. The address must be
the BO's start, and the BO size must equal the expected size
exactly (kfd_queue_buffer_get). So each is its own allocation:

| buffer | size | note |
|---|---|---|
| ring | ring_size, a power of two, >= 1 KiB | flag EXECUTABLE too |
| write pointer | 4096 | its own BO |
| read pointer | 4096 | its own BO |
| CWSR area | ALIGN(cwsr_size + debug, 4096) | required |
| EOP buffer | 4096 | required on gfx11, see below |

CWSR sizes come from sysfs node properties: `cwsr_size` and
`ctl_stack_size`. Pass them as ctx_save_restore_size and
ctl_stack_size; the kernel rejects a smaller CWSR size or a
different stack size. debug = ALIGN(cu * 32 * 32, 64) with
cu = simd_count / simd_per_cu (kfd_topology.c). On this box:
cwsr_size 19185664, ctl_stack_size 16384, cu 40, so the CWSR BO
is 19226624 bytes. Zero-filled is enough to create the queue.
The EOP buffer is optional to the ioctl but not to the hardware:
kfd_mqd_manager_v11.c programs it into the MQD with no zero check.
A queue made without one hangs MES on REMOVE_QUEUE, and the driver
resets the whole GPU (seen 2026-09-22, four times).

## CREATE_QUEUE, nr 0x02, read+write, 96 bytes

ring_base u64, write_ptr u64, read_ptr u64, doorbell_offset u64
(out), ring_size u32, gpu_id u32, queue_type u32 (AQL = 2),
queue_percentage u32 (100), queue_priority u32 (7), queue_id u32
(out), eop_addr u64, eop_size u64, ctx_save_restore_addr u64,
ctx_save_restore_size u32, ctl_stack_size u32, sdma_engine_id
u32, metadata_ring_size u32 (0; `pad` before uapi 1.22).
DESTROY_QUEUE: nr 0x03, {queue_id u32, pad u32}.

## Doorbell

The returned doorbell_offset holds the page offset in its high
bits and this queue's byte offset in the low 13 bits. mmap 8 KiB
on the KFD fd at (offset & !0x1FFF). The doorbell is the u64 at
mapping + (offset & 0x1FFF). Writing the packet index there
starts the hardware.

## Events, CREATE_EVENT nr 0x08, read+write, 32 bytes

event_page_offset u64 (out on an APU; pass 0), event_trigger_data
u32 (out), event_type u32 (SIGNAL = 0), auto_reset u32, node_id
u32 (0), event_id u32 (out), event_slot_index u32 (out). mmap the
event page on the KFD fd at event_page_offset, 32 KiB. The slot
is the u64 at index event_slot_index. DESTROY_EVENT: nr 0x09,
{event_id u32, pad u32}. Call RUNTIME_ENABLE once before any queue.
