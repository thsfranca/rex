import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    globals: false,
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      include: ["src/**/*.ts"],
      exclude: ["src/**/*.test.ts", "src/types/**", "src/extension.ts"],
    },
    alias: {
      vscode: new URL("./src/test/vscodeStub.ts", import.meta.url).pathname,
    },
  },
});
