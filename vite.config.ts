import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [{ find: "@", replacement: path.resolve(__dirname, "src") }],
  },
  test: { include: ["src/**/*.test.ts"] },
  server: {
    // the backend only accepts its own origin
    proxy: {
      "/api": {
        target: "http://127.0.0.1:4242",
        changeOrigin: true,
        headers: { origin: "http://127.0.0.1:4242" },
      },
    },
  },
});
