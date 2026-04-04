import { defineConfig } from 'vite'
import { viteSingleFile } from 'vite-plugin-singlefile'
import wasm from "vite-plugin-wasm"

export default defineConfig({
  plugins: [wasm(), viteSingleFile()],
  build: {
    target: "esnext",
    assetsInlineLimit: 100000000,
    rollupOptions: {
      output: { inlineDynamicImports: true },
    },
  },
})
