import { defineConfig } from "vite";
import path from "path";
import react from "@vitejs/plugin-react";
import tailwindcss from "tailwindcss";
import wasm from "vite-plugin-wasm";

// https://vite.dev/config/
export default defineConfig({
  esbuild: {
    supported: {
      "top-level-await": true,
    },
  },
  plugins: [react(), wasm()],
  css: {
    postcss: {
      plugins: [tailwindcss],
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    target: "es2020",
  },
  base: process.env.NODE_ENV === "production" ? "/<your-repo-name>/" : "/",
});
