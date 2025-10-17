import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true,
        // ✅ Prevent proxy from choking on big multipart streams
        timeout: 0,
        proxyTimeout: 0,
        ws: false,
      }
    }
  }
});

