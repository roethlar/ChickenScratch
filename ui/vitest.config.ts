import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Test-only config: kept separate from vite.config.ts so the app build
// (and its manualChunks tuning) is untouched by the test harness.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
