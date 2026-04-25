import { build, context } from "esbuild";

const watch = process.argv.includes("--watch");
const production = process.env.NODE_ENV === "production";

const options = {
  entryPoints: ["webview/index.tsx"],
  bundle: true,
  outfile: "dist/webview.js",
  platform: "browser",
  target: "es2022",
  format: "iife",
  sourcemap: !production,
  minify: production,
  jsx: "automatic",
  loader: {
    ".css": "text",
    ".svg": "text",
  },
  logLevel: "info",
};

if (watch) {
  const ctx = await context(options);
  await ctx.watch();
  console.log("[esbuild] watching webview...");
} else {
  await build(options);
}
