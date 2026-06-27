import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

// https://vitest.dev/config/
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    globals: true,
    passWithNoTests: true,
  },
})
