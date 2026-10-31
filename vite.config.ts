import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2021", chunkSizeWarningLimit: 900 },
  test: { environment: "node", include: ["src/**/*.test.{ts,tsx}"] },
} as any);
