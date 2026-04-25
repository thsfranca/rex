import { build, context } from "esbuild";

const watch = process.argv.includes("--watch");
const production = process.env.NODE_ENV === "production";

const options = {
  entryPoints: ["src/extension.ts"],
  bundle: true,
  outfile: "dist/extension.js",
  platform: "node",
  target: "node20",
  format: "cjs",
  sourcemap: !production,
  minify: production,
  external: ["vscode"],
  logLevel: "info",
};

if (watch) {
  const ctx = await context(options);
  await ctx.watch();
  console.log("[esbuild] watching extension host...");
} else {
  await build(options);
}
