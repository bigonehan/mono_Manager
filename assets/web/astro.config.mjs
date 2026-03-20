import { defineConfig } from "astro/config";
import node from "@astrojs/node";
import react from "@astrojs/react";
import path from "node:path";

export default defineConfig({
  output: "server",
  adapter: node({ mode: "standalone" }),
  integrations: [react()],
  server: {
    host: "127.0.0.1",
    port: 4175
  },
  vite: {
    resolve: {
      alias: {
        "@": path.resolve("./src")
      }
    }
  }
});
