# redbluegreen

## Why

redbluegreen is a GPU training toolchain for one machine, built from
the kernel interface up, with no GPU userspace stack under it.

The machine is an AMD Ryzen AI MAX+ 395 with a Radeon 8060S iGPU
(gfx1151, RDNA 3.5). It has 96 GiB of GTT out of 125 GiB of unified
RAM, kernel 7.0.10-arch1-1, amdgpu loaded, and no ROCm package
installed. The hardware is capacity-rich and bandwidth-poor, near
256 GB/s. Every design choice follows from that one fact: use wide
batches and read each weight few times per step.

The consumer is the sibling repo `david`: two small diffusion models,
a graph coupling model over discrete edges and a code span model.
Both must train in one night on this box.

## What

The layers, in dependency order. Each layer uses only the one below.

1. kfd and queue. Open the device, get a virtual address space,
   build a queue, ring a doorbell. See docs/floor.md.
2. memory. Allocate GPU memory, map it, hold a host view of it.
3. kernels. matmul, scatter and gather, softmax and attention over
   short sequences, elementwise, reductions.
4. autograd. A backward for each kernel. Hand-derived first. Add a
   tape only if the two models need one.
5. optimizer. One fused step across all parameters.
6. loop. Forward, backward, step, repeat.
7. data. The whole corpus stays resident in GPU memory, so no layer
   above reads the disk during a training run.

## What exists today

The binary `rbg` (src/main.rs, src/kfd.rs) talks to the kfd
driver: it prints the uapi version, gpu_id and gfx target, can
call ACQUIRE_VM, can allocate a 1 MiB GTT buffer, map it to the
GPU, and verify a pattern through the host view, and can build
an AQL compute queue and a SIGNAL event on top of it and tear
both down, and fill the CWSR header before CREATE_QUEUE (unit
5b), and can dispatch a kernel (kernels/store42, built by
`tools/asm` from src/kernels/) and wait on its signal, and can run
`rbg add N` to add two float buffers with kernels/vadd, time it,
and verify every element on the CPU, and can run
`rbg matmul M N K` to multiply two float matrices with
kernels/matmul, time it in GFLOP/s, and verify on the CPU.
Beyond it, the repo holds docs, `tools/doclint`, `tools/asm`,
`tools/drive`.

## What next

Read docs/lessons.md first. Then docs/floor.md for the interface,
docs/language.md for the host language, and docs/roadmap.md for the
first five build units. Units 1 to 6 are done; unit 6b is next.
