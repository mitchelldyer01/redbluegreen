# Facts: method

Who did what: a 27B local model (Qwen) running in the pi coding
agent wrote the implementation, one unit per one-page prompt. A
larger model acted as driver: planned units, verified facts in
kernel and ROCm source before each prompt, reviewed every handoff,
and bisected the hangs. The owner ran every GPU step by hand and
made every decision at a gate.

Doc rules (docs/conventions.md): 80 columns, 60 lines per doc,
story shape (why, what, next), no repeats across docs, one
bird's-eye page per repo. A shell-and-awk checker enforces the
first two. The same rules bind the agents' reports.

AGENTS.md rules that came from failures: read files by line
range and pipe build output through tail (context is the scarce
resource); never create a GPU queue from inside an agent session
(the agent's own model shares the GPU; a hang ends its session);
report a wrong doc fact as stale, never fix or trust it silently.

Prompts are thin launchers: facts verified by the driver, a goal,
a verification loop, a handoff that rewrites STATUS.md with one
of four states: done, in progress, ready for GPU test, blocked.
tools/drive loops the agent until done or until a person must
act. A pi extension hands off at 40k context tokens instead of
compacting, because prompt processing runs at about 200 tokens
per second and a long context costs minutes per turn. Another
extension nudges the model when a turn ends with reasoning only.

Local-model facts learned: the server's speculative decoder
emitted early end tokens under two slots (2 of 6 answers empty)
and none under one slot; one slot with the draft on was kept.

Rules that cost resets, one line each: EOP required; COHERENT on
CP buffers; 8 spare VGPRs; no mappings while another process is
busy; one run first. Recorded in docs/lessons.md with the date.

The bisect kernels are committed as the record: any future hang
gets bisected against the same set.

Two facts read from source were wrong for the hardware (EOP
optional, RUNTIME_ENABLE debugger-only). Reading finds
candidates; only a run settles them; the first run is one run.
