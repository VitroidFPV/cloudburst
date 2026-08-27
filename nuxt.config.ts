export default defineNuxtConfig({
  compatibilityDate: '2026-08-27',
  ssr: false,
  telemetry: false,
  devtools: { enabled: false },
  modules: ['@nuxt/ui'],
  css: ['~/assets/css/main.css'],
  devServer: {
    host: '127.0.0.1',
    port: 3000,
  },
  vite: {
    clearScreen: false,
    server: {
      strictPort: true,
    },
  },
  app: {
    head: {
      title: 'Cloudburst',
      meta: [
        { name: 'description', content: 'A focused desktop interface for qBittorrent.' },
      ],
    },
  },
})
