import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  // GitHub Pages serves from /latch/
  base: process.env.LATCH_PAGES ? "/latch/" : "/",
  define: {
    "process.env": {},
    global: "globalThis",
  },
  resolve: {
    alias: {
      buffer: "buffer",
    },
  },
  // @latch-labs/checkout is a file: workspace dependency (symlinked outside
  // node_modules) until its first npm release; include it in the CJS
  // transform and dep prebundle so named exports resolve.
  optimizeDeps: {
    include: ["@latch-labs/checkout"],
  },
  build: {
    commonjsOptions: {
      include: [/node_modules/, /clients\/ts\/packages\/checkout/],
    },
  },
});
