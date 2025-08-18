import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";
import { componentTagger } from "lovable-tagger";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  server: {
    host: "::",
    port: 5173, // Changed from 8080 to avoid conflict with backend
    strictPort: false, // Allow fallback to next available port
    
    // Proxy configuration for development
    proxy: {
      // Proxy API requests to backend server
      '/api': {
        target: process.env.VITE_API_BASE_URL || 'http://localhost:8080',
        changeOrigin: true,
        secure: false, // Allow self-signed certificates in development
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, _res) => {
            console.log('Proxy error:', err);
          });
          proxy.on('proxyReq', (proxyReq, req, _res) => {
            console.log('Proxying request:', req.method, req.url);
          });
          proxy.on('proxyRes', (proxyRes, req, _res) => {
            console.log('Proxy response:', proxyRes.statusCode, req.url);
          });
        },
      },
      
      // Proxy authentication requests
      '/auth': {
        target: process.env.VITE_API_BASE_URL || 'http://localhost:8080',
        changeOrigin: true,
        secure: false,
      },
      
      // Proxy health check requests
      '/health': {
        target: process.env.VITE_API_BASE_URL || 'http://localhost:8080',
        changeOrigin: true,
        secure: false,
      },
      
      // Proxy WebSocket connections
      '/ws': {
        target: process.env.VITE_WS_BASE_URL || 'ws://localhost:8080',
        changeOrigin: true,
        ws: true, // Enable WebSocket proxying
        secure: false,
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, _res) => {
            console.log('WebSocket proxy error:', err);
          });
          proxy.on('open', (_proxySocket) => {
            console.log('WebSocket proxy connection opened');
          });
          proxy.on('close', (_res, _socket, _head) => {
            console.log('WebSocket proxy connection closed');
          });
        },
      },
    },
  },
  
  plugins: [
    react(),
    mode === 'development' &&
    componentTagger(),
  ].filter(Boolean),
  
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  
  // Build configuration
  build: {
    // Generate source maps for better debugging
    sourcemap: mode === 'development',
    
    // Optimize bundle size
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['react', 'react-dom'],
          ui: ['@radix-ui/react-dialog', '@radix-ui/react-dropdown-menu'],
        },
      },
    },
    
    // Set chunk size warning limit
    chunkSizeWarningLimit: 1000,
  },
  
  // Development specific optimizations
  optimizeDeps: {
    include: [
      'react',
      'react-dom',
      'react/jsx-runtime',
    ],
  },
  
  // Environment variable prefix
  envPrefix: 'VITE_',
  
  // Preview server configuration (for production builds)
  preview: {
    port: 4173,
    strictPort: false,
    host: "::",
  },
}));
