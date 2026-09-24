# Facts: the floor

The floor is /dev/kfd, the compute driver's character device.
Every ioctl in the project goes to it. /dev/dri/renderD128 is
opened for two reasons only: its fd is handed to ACQUIRE_VM, and
buffer host views are mmapped through it (the kfd fd answers
ENODEV for buffers; it serves doorbells and the event page).

Ioctl encoding: dir<<30 | size<<16 | 'K'<<8 | nr. A wrong struct
size gives a wrong request number and EINVAL, so every struct is
repr(C) with a compile-time size assert.

Unit 1 sequence: read gpu_id from sysfs topology (skip nodes with
gpu_id 0); open both nodes; GET_VERSION (nr 0x01, 8 bytes, answers
1.22 on this kernel while the on-disk header says 1.18, so check
the major only); ACQUIRE_VM (nr 0x15, {drm_fd, gpu_id}).

RUNTIME_ENABLE (nr 0x25, 16 bytes) is called once per process
before any queue, as ROCr does. The kernel comment says MES relies
on it to clear stale process context. It was first dismissed as
"debugger only" in the docs; that was wrong.

Unavoidable dependencies: the amdgpu/kfd driver and its header;
an assembler for gfx1151 machine code (llvm-mc, build time only;
a hand encoder could replace it); libc for ioctl and mmap, or raw
syscalls later. Host language: Rust, chosen for exact struct
layout, hand-declared C functions, and ownership of mapped
memory. Bend 2 was considered and rejected: no AMD target and it
brings a runtime.

Verified-by-reading versus verified-by-running: the kernel header
and libhsakmt source were read for every ioctl, and each was then
proven by a run. Reading alone was wrong twice (EOP, COHERENT).
