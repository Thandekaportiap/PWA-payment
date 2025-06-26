// import { defineConfig } from 'vite'
// import react from '@vitejs/plugin-react-swc'

// // https://vite.dev/config/
// export default defineConfig({
//   plugins: [react()],
// })
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.ico', 'apple-touch-icon.png', 'masked-icon.svg'],
      manifest: {
        name: 'Payment PWA',
        short_name: 'PaymentPWA',
        description: 'Subscription Payment PWA with Stripe and Peach Payments',
        theme_color: '#ffffff',
        background_color: '#ffffff',
        display: 'standalone',
        
      }
    })
  ],
  server: {
    port: 3000
  }
})
