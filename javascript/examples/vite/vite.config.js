import { defineConfig } from 'vite'
export default defineConfig({
  build: {
    target: 'esnext',
  },
  optimizeDeps: { // https://github.com/vitejs/vite/issues/13756
    exclude: ['evmole'],
  },
})
