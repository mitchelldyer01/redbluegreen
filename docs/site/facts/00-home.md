# Facts: home

Voice (owner's words, use as the opening, first person):
"I want to train and work with models more closely, but my
hardware has some serious limitations. Support is full of gaps and
it is not obvious how the off-the-shelf software is meant to work;
I am constantly pinning versions and community forks." So the
toolchain is built from the kernel interface up, with no GPU
userspace under it. Tone: dry lab notebook. The site exists so the
owner can re-derive the reasoning behind each decision a year on.

The project: redbluegreen, a GPU training toolchain for one
machine, an AMD Strix Halo APU. Consumer: david, two small
diffusion models (a graph coupling model over discrete edges and a
code span model), which must train in one night on this box.

Layers, dependency order (docs/birdseye.md): kfd and queue;
memory; kernels; autograd; optimizer; loop; data resident in GPU
memory. Units 1 to 5b built the first two layers and the first
kernels. Nothing above layer 3 exists yet.

Units, one line each:
1 talk to the driver (GET_VERSION, ACQUIRE_VM)
2 own memory (ALLOC, MAP, host view)
3 own the queue (Drop types, AQL queue, event)
4 dispatch nothing (one kernel writes 42)
5 a kernel that matters (vector add, 225 GB/s)
5b survive a pause (root cause: page tables in a full carve-out)

Cost: 31 GPU resets on 2026-09-22, each a rule now written down.
Owner's line on that: "thirty one ways not to make a lightbulb".
Zero external dependencies at run time. Build time: llvm-mc and
llvm-objcopy to assemble kernels; rustc. No crates.
Method: a 27B local model (Qwen) in the pi coding agent did the
implementation from one-page prompts; the owner ran GPU steps.
