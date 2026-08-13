import { resolve } from "node:path"
import tailwindcss from "@tailwindcss/vite"
import { tanstackRouter } from "@tanstack/router-plugin/vite"
import react from "@vitejs/plugin-react"
import { defineConfig, externalizeDepsPlugin } from "electron-vite"

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: resolve("src/desktop/main/index.ts"),
      },
    },
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: resolve("src/desktop/preload/index.ts"),
      },
    },
  },
  renderer: {
    root: resolve("src/desktop/renderer"),
    resolve: {
      alias: {
        "@": resolve("src/desktop/renderer/src"),
      },
    },
    plugins: [
      tanstackRouter({
        target: "react",
        routesDirectory: resolve("src/desktop/renderer/src/routes"),
        generatedRouteTree: resolve("src/desktop/renderer/src/routeTree.gen.ts"),
      }),
      react(),
      tailwindcss(),
    ],
    build: {
      rollupOptions: {
        input: {
          index: resolve("src/desktop/renderer/index.html"),
        },
      },
    },
  },
})
