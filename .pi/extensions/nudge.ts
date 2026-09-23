// nudge - when the model ends a turn with reasoning only (no text,
// no tool call), send "continue" so the session does not sit idle
// waiting for a person. At most NUDGES per session.

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const NUDGES = 3;
const MSG =
  "Continue. Your last turn ended after reasoning with no text and " +
  "no tool call. Carry on with the next step.";

export default function (pi: ExtensionAPI) {
  let used = 0;

  pi.on("turn_end", async (event, ctx) => {
    if (used >= NUDGES) return;
    const m = event.message;
    if (!m || m.role !== "assistant") return;
    const parts = Array.isArray(m.content) ? m.content : [];
    const acted = parts.some(
      (p: { type?: string; text?: string }) =>
        p.type === "toolCall" ||
        (p.type === "text" && (p.text ?? "").trim().length > 0),
    );
    if (acted) return;
    if (event.toolResults && event.toolResults.length > 0) return;
    used += 1;
    if (ctx.hasUI) ctx.ui.notify(`nudge ${used}/${NUDGES}`, "warning");
    pi.sendUserMessage(MSG, { deliverAs: "followUp" });
  });
}
