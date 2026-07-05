#!/usr/bin/env node
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import looksSame from "looks-same";
import { findRepoRoot, loadConfig } from "./config.js";
import { ciede2000, parseCssColor } from "./color.js";
import {
  closeSession,
  getSession,
  gotoScenario,
  openSession,
} from "./session.js";
import {
  pageCanvasHash,
  pageClick,
  pageClockStep,
  pageCssTokenAssert,
  pageEmulateReducedMotion,
  pageFocus,
  pageLayout,
  pageLocatorScreenshot,
  pagePress,
  pageScreenshot,
  pageSnapshotTree,
  pageType,
  pageWaitForSelector,
  pageWaitForText,
} from "./page.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = findRepoRoot(process.cwd());
const config = loadConfig(repoRoot);

const TOKEN_ENUM = [
  "--rex-text-primary",
  "--rex-text-secondary",
  "--rex-surface-base",
  "--rex-surface-raised",
  "--rex-surface-overlay",
  "--rex-surface-dimmed",
  "--rex-hairline-default",
  "--rex-hairline-focus",
  "--rex-status-success",
  "--rex-status-working",
  "--rex-status-error",
] as const;

const server = new Server(
  { name: "rex-ui-harness", version: "0.1.0" },
  { capabilities: { tools: {} } }
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [
    {
      name: "ui_open",
      description:
        "Launch Rex desktop session (Tauri + production dist + mock daemon).",
      inputSchema: {
        type: "object",
        properties: {
          mode: {
            type: "string",
            enum: ["desktop"],
            description: "desktop = Tauri + UDS daemon",
          },
        },
      },
    },
    {
      name: "ui_close",
      description: "Terminate the browser session.",
      inputSchema: { type: "object", properties: {} },
    },
    {
      name: "ui_goto_scenario",
      description: "Hydrate UI with a mock scenario (idle, streaming, approval_required, error).",
      inputSchema: {
        type: "object",
        properties: {
          scenario: {
            type: "string",
            enum: ["idle", "streaming", "approval_required", "error", "history-fetch"],
          },
        },
        required: ["scenario"],
      },
    },
    {
      name: "ui_snapshot",
      description: "Capture accessibility tree summary and optional screenshot.",
      inputSchema: {
        type: "object",
        properties: {
          screenshot: { type: "boolean", default: false },
        },
      },
    },
    {
      name: "ui_assert_token",
      description: "Assert computed color matches a semantic --rex-* token (CIEDE2000).",
      inputSchema: {
        type: "object",
        properties: {
          selector: { type: "string" },
          token: { type: "string", enum: [...TOKEN_ENUM] },
          property: {
            type: "string",
            enum: ["color", "background-color", "border-color"],
          },
          max_delta_e: { type: "number", default: 2.3 },
        },
        required: ["selector", "token", "property"],
      },
    },
    {
      name: "ui_clock_step",
      description: "Advance mocked Playwright animation clock.",
      inputSchema: {
        type: "object",
        properties: { duration_ms: { type: "number" } },
        required: ["duration_ms"],
      },
    },
    {
      name: "ui_assert_motion",
      description: "Validate region animation between clock steps.",
      inputSchema: {
        type: "object",
        properties: {
          region: { type: "string" },
          effect: { type: "string", enum: ["slide-in", "fade", "spring-scale"] },
          min_duration_ms: { type: "number" },
          max_duration_ms: { type: "number" },
        },
        required: ["region", "effect"],
      },
    },
    {
      name: "ui_assert_layout",
      description: "Validate flex/grid containment on a selector.",
      inputSchema: {
        type: "object",
        properties: {
          selector: { type: "string" },
          display: { type: "string" },
          flex_direction: { type: "string" },
          grid_template_columns: { type: "string" },
        },
        required: ["selector"],
      },
    },
    { name: "ui_click", description: "Click an element.", inputSchema: { type: "object", properties: { selector: { type: "string" } }, required: ["selector"] } },
    {
      name: "ui_send_keys",
      description: "Send keyboard input to focused element or page.",
      inputSchema: {
        type: "object",
        properties: {
          selector: { type: "string" },
          keys: { type: "string" },
        },
        required: ["keys"],
      },
    },
    {
      name: "ui_wait_for",
      description: "Wait for selector or text.",
      inputSchema: {
        type: "object",
        properties: {
          selector: { type: "string" },
          text: { type: "string" },
          timeout_ms: { type: "number", default: 10000 },
        },
      },
    },
    {
      name: "ui_record_motion",
      description: "Start or stop motion frame capture sequence.",
      inputSchema: {
        type: "object",
        properties: { action: { type: "string", enum: ["start", "stop"] } },
        required: ["action"],
      },
    },
    {
      name: "ui_assert_canvas",
      description: "Hash WebGL/canvas buffer to detect shader flux.",
      inputSchema: {
        type: "object",
        properties: {
          selector: { type: "string", default: "#ambient" },
          min_change_ratio: { type: "number", default: 0.001 },
        },
      },
    },
    {
      name: "ui_diff_baseline",
      description: "Compare screenshot to committed baseline PNG (looks-same).",
      inputSchema: {
        type: "object",
        properties: {
          name: { type: "string" },
          max_diff_pixels: { type: "number", default: 0 },
        },
        required: ["name"],
      },
    },
    {
      name: "ui_set_prefers_reduced_motion",
      description: "Toggle prefers-reduced-motion media query emulation.",
      inputSchema: {
        type: "object",
        properties: { enabled: { type: "boolean" } },
        required: ["enabled"],
      },
    },
  ],
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;
  try {
    const result = await handleTool(name, (args ?? {}) as Record<string, unknown>);
    return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { content: [{ type: "text", text: JSON.stringify({ error: message }) }], isError: true };
  }
});

async function handleTool(name: string, args: Record<string, unknown>): Promise<unknown> {
  switch (name) {
    case "ui_open": {
      const opened = await openSession(config, { mode: "desktop" });
      return {
        ok: true,
        mode: opened.mode,
        rex_root: config.rexRoot,
      };
    }
    case "ui_close":
      await closeSession();
      return { ok: true };
    case "ui_goto_scenario":
      await gotoScenario(String(args.scenario));
      return { ok: true, scenario: args.scenario };
    case "ui_snapshot": {
      const s = getSession();
      const tree = await pageSnapshotTree(s);
      const payload: Record<string, unknown> = { tree, mode: s.mode };
      if (args.screenshot) {
        payload.screenshot_base64 = (await pageScreenshot(s)).toString("base64");
      }
      return payload;
    }
    case "ui_assert_token": {
      const s = getSession();
      const selector = String(args.selector);
      const token = String(args.token);
      const property = String(args.property);
      const maxDelta = Number(args.max_delta_e ?? 2.3);
      const { actual, expected } = await pageCssTokenAssert(s, selector, token, property);
      const delta = ciede2000(parseCssColor(actual), parseCssColor(expected));
      const pass = delta <= maxDelta;
      return { pass, delta_e: delta, actual, expected, token, max_delta_e: maxDelta };
    }
    case "ui_clock_step": {
      const s = getSession();
      await pageClockStep(s, Number(args.duration_ms));
      return { ok: true, advanced_ms: args.duration_ms };
    }
    case "ui_assert_motion": {
      const s = getSession();
      const region = String(args.region);
      const before = await pageLocatorScreenshot(s, region);
      await pageClockStep(s, Number(args.min_duration_ms ?? 150));
      const mid = await pageLocatorScreenshot(s, region);
      await pageClockStep(s, Number(args.max_duration_ms ?? 350));
      const after = await pageLocatorScreenshot(s, region);
      const diff = !before.equals(mid) || !mid.equals(after);
      return { pass: diff, effect: args.effect, region };
    }
    case "ui_assert_layout": {
      const s = getSession();
      const selector = String(args.selector);
      const layout = await pageLayout(s, selector);
      const checks: Record<string, boolean> = {};
      if (args.display) checks.display = layout.display === args.display;
      if (args.flex_direction)
        checks.flex_direction = layout.flexDirection === args.flex_direction;
      if (args.grid_template_columns)
        checks.grid_template_columns = layout.gridTemplateColumns === args.grid_template_columns;
      const pass = Object.values(checks).every(Boolean);
      return { pass, layout, checks };
    }
    case "ui_click": {
      const s = getSession();
      await pageClick(s, String(args.selector));
      return { ok: true };
    }
    case "ui_send_keys": {
      const s = getSession();
      const keys = String(args.keys);
      if (args.selector) {
        await pageFocus(s, String(args.selector));
      }
      if (keys.includes("{Enter}")) {
        const text = keys.replaceAll("{Enter}", "");
        if (text) await pageType(s, text);
        await pagePress(s, "Enter");
      } else {
        await pageType(s, keys);
      }
      return { ok: true };
    }
    case "ui_wait_for": {
      const s = getSession();
      const timeout = Number(args.timeout_ms ?? 10000);
      if (args.selector) await pageWaitForSelector(s, String(args.selector), timeout);
      if (args.text) await pageWaitForText(s, String(args.text), timeout);
      return { ok: true };
    }
    case "ui_record_motion": {
      const s = getSession();
      if (args.action === "start") {
        s.recording = true;
        s.motionFrames = [];
        s.motionFrames.push(await pageScreenshot(s));
        return { ok: true, recording: true };
      }
      s.recording = false;
      return { ok: true, frame_count: s.motionFrames.length, recording: false };
    }
    case "ui_assert_canvas": {
      const s = getSession();
      const selector = String(args.selector ?? "#ambient");
      if (s.mode === "desktop") {
        return { pass: false, skipped: true, reason: "no canvas tier in desktop MVP" };
      }
      const hash1 = await pageCanvasHash(s, selector);
      await pageClockStep(s, 100);
      const hash2 = await pageCanvasHash(s, selector);
      const changed = hash1 !== hash2;
      return { pass: changed, hash1_preview: hash1?.slice(0, 32), hash2_preview: hash2?.slice(0, 32) };
    }
    case "ui_diff_baseline": {
      const s = getSession();
      const name = String(args.name);
      const baselinePath = path.join(config.baselineDir, `${name}.png`);
      const current = await pageScreenshot(s);
      if (!fs.existsSync(baselinePath)) {
        fs.mkdirSync(config.baselineDir, { recursive: true });
        fs.writeFileSync(baselinePath, current);
        return { pass: true, created_baseline: baselinePath };
      }
      const { equal, diffBounds, diffClusters } = await looksSame(baselinePath, current, {
        tolerance: 2,
        antialiasingTolerance: 3,
        shouldCluster: true,
      });
      return { pass: equal, diffBounds, diffClusters };
    }
    case "ui_set_prefers_reduced_motion": {
      const s = getSession();
      await pageEmulateReducedMotion(s, args.enabled === true);
      return { ok: true, reduced_motion: args.enabled };
    }
    default:
      throw new Error(`Unknown tool: ${name}`);
  }
}

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
