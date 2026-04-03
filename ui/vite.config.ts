import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const SITE = process.env.VITE_SITE ?? "exit-test.localhost";
const BACKEND = process.env.VITE_BACKEND ?? "http://127.0.0.1:8000";

/** Inject the correct Host header on every proxied request so Spotledger
 *  routes to the right site (it routes by hostname). */
function withSiteHost() {
  return {
    configure: (proxy: import("http-proxy").Server) => {
      proxy.on("proxyReq", (proxyReq: import("http").ClientRequest) => {
        proxyReq.setHeader("Host", SITE);
      });
    },
  };
}

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        "form-host": "form-host.html",
      },
    },
  },
  server: {
    port: 8080,
    watch: { usePolling: true, interval: 500 },
    proxy: {
      // Spotledger API — all /api/* calls need the Host header injected
      "/api": {
        target: BACKEND,
        changeOrigin: false,
        ...withSiteHost(),
      },
      // Frappe compiled assets — form-host.html loads libs/desk/form bundles
      // from here; they are compiled by bench and served by Spotledger
      "/assets": {
        target: BACKEND,
        changeOrigin: false,
        ...withSiteHost(),
      },
    },
  },
});
