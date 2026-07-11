"use strict";

const { createRexClient, unary, grpc } = require("./grpcClient.cjs");
const { mapGrpcChunk } = require("./streamMap.cjs");

function newHarnessSessionId() {
  return `hs-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
}

function metadataFor(traceId, harnessSessionId) {
  const md = new grpc.Metadata();
  if (traceId) md.set("x-rex-trace-id", traceId);
  if (harnessSessionId) md.set("x-rex-harness-session-id", harnessSessionId);
  return md;
}

/**
 * StreamInference → onEvent(StreamEventDto) fan-out.
 */
function submitPromptStream(prompt, mode, onEvent) {
  const harnessSessionId = newHarnessSessionId();
  const traceId = `web-${harnessSessionId}`;
  const client = createRexClient();
  const md = metadataFor(traceId, harnessSessionId);

  const call = client.StreamInference(
    {
      prompt,
      model: "",
      mode: mode || "agent",
      approval_id: "",
      continue_token: "",
    },
    md,
  );

  return new Promise((resolve, reject) => {
    let settled = false;
    const finish = (err) => {
      if (settled) return;
      settled = true;
      try {
        client.close?.();
      } catch {
        // ignore
      }
      if (err) reject(err);
      else resolve(harnessSessionId);
    };

    call.on("data", (chunk) => {
      for (const evt of mapGrpcChunk(chunk)) {
        onEvent(evt);
      }
    });
    call.on("error", (err) => {
      onEvent({
        kind: "error",
        code: "stream_error",
        message: err?.message || String(err),
      });
      finish(err);
    });
    call.on("end", () => finish(null));
  });
}

async function fetchSessionEvents(harnessSessionId, opts = {}) {
  const client = createRexClient();
  try {
    const md = metadataFor(null, harnessSessionId);
    const res = await unary(
      client,
      "FetchSessionEvents",
      {
        before_sequence: opts.beforeSequence || 0,
        after_sequence: opts.afterSequence || 0,
        limit: opts.limit || 100,
      },
      md,
    );
    return {
      events: (res.events || []).map((e) => ({
        sequence: Number(e.sequence || 0),
        event: e.event || "",
        text: e.text || "",
        turnId: e.turn_id || "",
        toolName: e.tool_name || "",
        phase: e.phase || "",
      })),
      hasMoreBefore: Boolean(res.has_more_before),
      hasMoreAfter: Boolean(res.has_more_after),
      headSequence: Number(res.head_sequence || 0),
    };
  } finally {
    client.close?.();
  }
}

async function respondToToolApproval(
  approvalToken,
  approved,
  toolCallId,
  harnessSessionId,
) {
  const client = createRexClient();
  try {
    const md = metadataFor(null, harnessSessionId);
    const res = await unary(
      client,
      "RespondToToolApproval",
      {
        approval_token: approvalToken,
        approved: Boolean(approved),
        tool_call_id: toolCallId || "",
      },
      md,
    );
    return { ok: Boolean(res.ok), error: res.error || "" };
  } finally {
    client.close?.();
  }
}

module.exports = {
  submitPromptStream,
  fetchSessionEvents,
  respondToToolApproval,
  newHarnessSessionId,
};
