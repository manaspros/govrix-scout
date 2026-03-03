import type { Config } from 'tailwindcss'

export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        primary: '#6366f1',
        'primary-light': '#818cf8',
        'primary-dark': '#4f46e5',
        danger: '#ef4444',
        warning: '#f59e0b',
        success: '#10b981',
        chart: {
          indigo: '#6366f1',
          violet: '#8b5cf6',
          sky: '#0ea5e9',
          emerald: '#10b981',
          amber: '#f59e0b',
          rose: '#f43f5e',
          slate: '#64748b',
        },
      },
      fontFamily: {
        display: ['Inter', 'sans-serif'],
        sans: ['Inter', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
    },
  },
  plugins: [],
} satisfies Config
