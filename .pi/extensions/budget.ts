// budget - hand off at a context budget instead of compacting.
//
// Why: the local model re-reads the whole prompt every turn at a
// few hundred tokens per second. Past ~40k tokens a turn costs
// minutes before output starts and the client times out. A fresh
// session with docs/STATUS.md as the resume point is cheaper than
// compaction and keeps the prompt cache warm.
//
// What: after each turn, if context usage passes the budget, send
// one steering message that asks for the handoff, then shut down
// once the agent is idle. `--budget N` overrides the default.

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const HANDOFF =
  "Stop the unit here. Do the handoff now: rewrite docs/STATUS.md " +
  "(position, verified with real output, stale, exact next step " +
  "so a fresh session can resume mid-unit), run tools/doclint on " +
  "it, and then stop. Do not start new work.";

export default function (pi: ExtensionAPI) {
  pi.registerFlag("budget", {
    description: "Context tokens before forced handoff",
    type: "number",
    default: 40_000,
  });

  let asked = false;

  pi.on("turn_end", async (_event, ctx) => {
    if (asked) return;
    const usage = ctx.getContextUsage();
    const budget = Number(pi.getFlag("budget"));
    if (!usage || usage.tokens < budget) return;
    asked = true;
    if (ctx.hasUI) {
      ctx.ui.notify(`budget: ${usage.tokens} tokens, handing off`, "warning");
    }
    pi.sendUserMessage(HANDOFF, { deliverAs: "followUp" });
  });

  pi.on("agent_end", async (_event, ctx) => {
    if (!asked) return;
    if (ctx.hasPendingMessages()) return;
    if (ctx.hasUI) ctx.ui.notify("budget: handoff done, shutting down", "info");
    ctx.shutdown();
  });
}
