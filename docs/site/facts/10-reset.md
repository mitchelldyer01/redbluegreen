# Facts: the reset chain

Symptom: a queue created while a local model server was
generating never ran, and the GPU reset two to three seconds in.
Eight experiments, one reset at most each, in order:
1. Two rbg processes at once, 64M, 1000 reps: both finish.
2. Twelve rbg processes at once: all finish, bandwidth shared.
3. Server up but idle: 64M add at full speed.
4. Add started first, server generates mid-run, 20000 reps: all
   verify at half speed. So our queues survive being paused.
5. Queue created during generation: hang, reset. Same with the
   CWSR header filled, with RUNTIME_ENABLE, and with 1-element
   buffers (that one ran its first packet, then the reset came).
6. The kernel log for every case names the server's queue as the
   one the driver failed to evict, and the server as the process
   it failed to suspend.

Chain, from the driver source (amdgpu_vm_pt.c, amdgpu_amdkfd
_gpuvm.c, kfd_device_queue_manager.c):
- GPU page tables are allocated in VRAM on this chip; only "app
  APUs" use system memory.
- VRAM here is a 512 MiB carve-out, 468 MiB used by the desktop
  (terminals and a browser, from per-process fdinfo).
- A new GPU mapping needs page-table pages; TTM evicts another
  process's VRAM buffer to make room.
- A kfd buffer carries an eviction fence: evicting it means
  pausing that process's queues (REMOVE_QUEUE through MES).
- MES cannot pause the server's mid-kernel queue in time; the
  driver declares MES unrecoverable and resets the GPU. Both
  processes die.

Why the candidate fixes were wrong: the context save header, the
executable flag on the save area, and RUNTIME_ENABLE all concern
pausing our queue. Our queue was never the one being paused.

Rule 4: allocate every GPU buffer at start, never inside a loop,
never while another process is busy. A long-lived training
process that maps once is safe on a shared desktop, because our
own queues survive pauses.

Reset history: 1 on Sep 21, 31 on Sep 22, none in the 30 days
before. Every reset was ours. Scheduler quanta from the source:
10 ms per process, 1 ms per gang; the twelve-process pass proves
our waves survive that switching.
