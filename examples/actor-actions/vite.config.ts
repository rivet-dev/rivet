import react from "@vitejs/plugin-react";
import build from '@hono/vite-build/node'
import devServer from '@hono/vite-dev-server'
import { defineConfig } from "vite";

export default defineConfig(({ mode, command }) => {
  if (mode === 'client') {
    return {
      build: {
        rollupOptions: {
          input: ['./frontend/main.tsx'],
          output: {
            entryFileNames: 'static/main.js',
            chunkFileNames: 'static/assets/[name]-[hash].js',
            assetFileNames: 'static/assets/[name].[ext]',
          },
        },
        emptyOutDir: false,
        copyPublicDir: false,
      },
	  plugins: [react()],
    }
  } else {
    return {
      build: {
        minify: true,
        rollupOptions: {
          output: {
            entryFileNames: '_worker.js',
          },
        },
      },
      plugins: command === "build" ? [build({entry: './src/server.tsx'})] : [
        devServer({
          entry: './src/server.tsx',
        }),
      ],
    }
  }
})
