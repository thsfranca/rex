/* eslint-env node */
const eslint = require("@eslint/js");
const tseslint = require("typescript-eslint");
const globals = require("globals");

const sharedRules = {
  "@typescript-eslint/no-unused-vars": [
    "error",
    { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
  ],
  "@typescript-eslint/no-explicit-any": "error",
  "@typescript-eslint/consistent-type-imports": "error",
  "no-console": ["warn", { allow: ["warn", "error"] }],
};

module.exports = tseslint.config(
  {
    ignores: ["dist/**", "out/**", "out-webview/**", "node_modules/**"],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.ts"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: globals.node,
    },
    rules: sharedRules,
  },
  {
    files: ["webview/**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: globals.browser,
    },
    rules: sharedRules,
  },
  {
    files: ["**/*.test.ts"],
    rules: {
      "no-console": "off",
    },
  },
);
