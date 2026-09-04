export default defineNuxtConfig({
  $development: {
    app: {
      head: {
        title: 'Cloudburst Dev',
      },
    },
    runtimeConfig: {
      public: {
        appName: 'Cloudburst Dev',
      },
    },
  },
  compatibilityDate: '2026-08-27',
  ssr: false,
  telemetry: false,
  devtools: { enabled: false },
  modules: ['@nuxt/eslint', '@nuxt/ui'],
  css: ['~/assets/css/main.css'],
  devServer: {
    host: '127.0.0.1',
    port: 3000,
  },
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
    },
  },
  ignore: ['**/src-tauri/**'],
  runtimeConfig: {
    public: {
      appName: 'Cloudburst',
    },
  },
  app: {
    head: {
      title: 'Cloudburst',
      meta: [
        { name: 'description', content: 'A focused desktop interface for qBittorrent.' },
      ],
      link: [
        { rel: 'icon', type: 'image/svg+xml', sizes: 'any', href: '/favicon.svg' },
      ],
    },
  },
})
