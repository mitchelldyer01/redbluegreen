# Facts: completion

Two faults stood between "the kernel ran" and "the CP said so".
Both were accepted by the kernel and refused by the hardware, and
the hardware refuses by hanging, which costs a GPU reset.

Fault 1, unit 3b: no EOP buffer. The ioctl treats it as optional
("not required for all ASICs"). The gfx11 MQD code programs its
address and size into the hardware queue descriptor with no zero
check. A queue built without one hangs MES on REMOVE_QUEUE at
destroy time. Four resets, each killing the model server sharing
the GPU. Fix: a 4 KiB coarse EXECUTABLE buffer, as libhsakmt
allocates.

Fault 2, unit 4: memory type. The kernel ran (target = 42) but
the completion signal stayed 1 and the queue could not be
removed. No page fault. Every buffer the CP touches in ROCr is
allocated fine-grained (COHERENT). Ours had none. The CP's own
reads and atomics did not see the same bytes the CPU wrote. Fix:
COHERENT on ring, both pointers, kernel object, kernarg, signal.
Five resets before the fix; five clean runs after.

Then the no-doorbell run, which had been a "fact" with the stale
mapping: with coherent memory the CP polls the write pointer and
a packet runs without the doorbell. The earlier "only the
doorbell starts it" was an artifact of non-coherent memory.

Lesson, in the owner's record: the kernel accepts many things the
hardware then refuses. From here every prompt that touches the CP
mirrors what ROCr allocates, not what the ioctl permits, and the
first GPU run of a new kernel is always one run.

Sources read to find the fixes: kfd_mqd_manager_v11.c (EOP),
ROCr amd_aql_queue.cpp and libhsakmt fmm.c (COHERENT on the
system allocator), kfd_queue.c (exact-size rule).

Numbers: rbg dispatch, five runs, target 42, signal 0, exit 0, no
reset; GTT drift 12 KiB over the runs, no leak.
