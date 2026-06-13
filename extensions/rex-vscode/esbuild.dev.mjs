import { build, context } from "esbuild";

const watch = process.argv.includes("--watch");

const options = {
  entryPoints: ["dev/mockHost.ts"],
  bundle: true,
  outfile: "dev/mock-host.js",
  platform: "browser",
  target: "es2022",
  format: "iife",
  sourcemap: true,
  logLevel: "info",
};

if (watch) {
  const ctx = await context(options);
  await ctx.watch();
  console.log("[esbuild] watching dev harness mock...");
} else {
  await build(options);
}
