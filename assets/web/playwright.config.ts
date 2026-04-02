import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  fullyParallel: false,
  use: {
    baseURL: "http://127.0.0.1:4175"
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 4175 --strictPort",
    port: 4175,
    reuseExistingServer: true,
    timeout: 60_000
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] }
    }
  ]
});
