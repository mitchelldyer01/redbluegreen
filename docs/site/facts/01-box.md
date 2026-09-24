# Facts: the box

AMD Ryzen AI MAX+ 395, Radeon 8060S iGPU, gfx1151 (RDNA 3.5,
Strix Halo). 32 CPU threads, 125 GiB unified RAM. Kernel 7.0,
amdgpu driver. No NVIDIA hardware; a first roadmap written by a
model said "no GPU" because nvidia-smi failed. That was wrong.

Memory as the GPU sees it:
- GTT: 96 GiB of system RAM the GPU may map (kernel parameter).
- VRAM carve-out: 512 MiB, set by firmware. The desktop keeps
  it about 90 percent full (terminals and a browser). GPU page
  tables are allocated here on this chip (amdgpu_vm_pt.c: only
  "app APUs" put them in system memory).
- Bandwidth: theoretical 256 GB/s. Measured 225 GB/s streaming.
- One unified pool: a CPU pointer and a GPU pointer can be the
  same virtual address (unit 2).

Consequence: capacity-rich, bandwidth-poor. A 27B model fits and
runs slowly; a small model trains at memory speed. Design rules
that follow: wide batches, few weight reads per step, the whole
corpus resident in GPU memory, CPU work pipelined with the GPU.

Node facts from sysfs: gpu_id read at run time (3750 today, not
stable across boots); gfx_target_version 110501; 80 SIMDs, 2 per
CU, 40 CUs; wave32; cwsr_size 19185664; ctl_stack_size 16384.

Software present: ROCm PyTorch works in a venv, and a ROCm
llama.cpp server runs a 27B model on the same GPU. redbluegreen
uses none of that at run time; it reads ROCm's source (libhsakmt,
ROCr) as reference for what the hardware expects.
