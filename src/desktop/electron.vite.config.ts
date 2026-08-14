import { resolve } from "node:path"
import tailwindcss from "@tailwindcss/vite"
import { tanstackRouter } from "@tanstack/router-plugin/vite"
import react from "@vitejs/plugin-react"
import { defineConfig, externalizeDepsPlugin } from "electron-vite"

const desktopRoot = resolve("src/desktop")
const rendererRoot = resolve(desktopRoot, "renderer")
const rendererSource = resolve(rendererRoot, "src")

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: resolve(desktopRoot, "main/index.ts"),
      },
    },
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    build: {
      rollupOptions: {
        input: resolve(desktopRoot, "preload/index.ts"),
      },
    },
  },
  renderer: {
    root: rendererRoot,
    resolve: {
      alias: {
        "@": rendererSource,
      },
    },
    plugins: [
      tanstackRouter({
        target: "react",
        routesDirectory: resolve(rendererSource, "routes"),
        generatedRouteTree: resolve(rendererSource, "routeTree.gen.ts"),
      }),
      react(),
      tailwindcss(),
    ],
    build: {
      rollupOptions: {
        input: {
          index: resolve(rendererRoot, "index.html"),
          overlay: resolve(rendererRoot, "overlay.html"),
        },
      },
    },
  },
})
