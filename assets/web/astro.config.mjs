import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import path from "node:path";

export default defineConfig({
  integrations: [react()],
  server: {
    host: "127.0.0.1",
    port: 4173
  },
  vite: {
    resolve: {
      alias: {
        "@": path.resolve("./src")
      }
    }
  }
});
