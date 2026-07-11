"use strict";

/**
 * Map StreamInferenceResponse chunks to renderer StreamEvent DTOs
 * (subset sufficient for mock hello + approvals).
 */
function mapGrpcChunk(chunk) {
  if (chunk.done) {
    return [{ kind: "done" }];
  }

  const event = String(chunk.event || "").trim();
  const events = [];

  if (event === "" || event === "chunk") {
    if (chunk.text) {
      events.push({ kind: "chunk", text: chunk.text });
    }
    return events;
  }

  if (event === "tool" && chunk.phase === "approval_required") {
    const detail = String(chunk.detail || "");
    let pathDetail = detail;
    let approvalToken = "";
    const split = detail.lastIndexOf("|");
    if (split >= 0) {
      const maybe = detail.slice(split + 1);
      if (maybe.startsWith("tap-")) {
        pathDetail = detail.slice(0, split);
        approvalToken = maybe;
      }
    }
    events.push({
      kind: "approvalRequired",
      toolCallId: chunk.tool_call_id || "",
      toolName: chunk.tool_name || "",
      detail: pathDetail,
      approvalToken,
    });
    return events;
  }

  if (event === "tool" || event === "step" || event === "activity") {
    if (chunk.phase) {
      const phase = normalizePhase(chunk.phase);
      if (phase) events.push({ kind: "phase", phase });
    }
    if (chunk.summary) {
      events.push({ kind: "message", text: chunk.summary });
    }
    return events;
  }

  if (event === "error") {
    events.push({
      kind: "error",
      code: chunk.phase || "stream_error",
      message: chunk.text || chunk.summary || "stream error",
    });
  }

  return events;
}

function normalizePhase(phase) {
  const p = String(phase || "").toLowerCase();
  if (
    p === "idle" ||
    p === "generating" ||
    p === "tool_running" ||
    p === "tool_approval" ||
    p === "terminal"
  ) {
    return p;
  }
  if (p === "approval_required") return "tool_approval";
  return null;
}

function toSystemStatus(res) {
  return {
    daemonVersion: res.daemon_version || "",
    uptimeSeconds: Number(res.uptime_seconds || 0),
    activeModelId: res.active_model_id || "",
    workspaceRoot: res.workspace_root || "",
    lifecycleState: res.lifecycle_state || "",
    idleSeconds: Number(res.idle_seconds || 0),
  };
}

module.exports = {
  mapGrpcChunk,
  toSystemStatus,
};
