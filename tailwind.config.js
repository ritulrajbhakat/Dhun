/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        dhun: {
          dark: '#0a0b0e',
          surface: '#12141a',
          card: '#181b22',
          border: '#262a36',
          primary: '#e11d48',       // Vibrant DJ red/crimson
          accent: '#f43f5e',
          gold: '#f59e0b',          // Indian festive gold
          cyan: '#06b6d4',          // Deck A cyan
          orange: '#f97316',        // Deck B orange
          purple: '#a855f7',        // Deck C purple
          emerald: '#10b981',       // Deck D emerald / verified
        }
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      }
    },
  },
  plugins: [],
}

