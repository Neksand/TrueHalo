/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        halo: {
          50: "#fdf8f6",
          100: "#f2e8e5",
          200: "#eaddd7",
          300: "#e0c2b7",
          400: "#d49785",
          500: "#bf6c54",
          600: "#a9543e",
          glow: "#ffaa77",
          accent: "#ff6b4a",
        },
        surface: {
          deep: "#0d0e12",
          card: "#15171e",
          cardHover: "#1c1f29",
          border: "#282c37",
          subtle: "#1c1f2a",
        },
      },
      boxShadow: {
        'halo': '0 0 25px -5px rgba(255, 107, 74, 0.25)',
        'halo-sm': '0 0 12px -2px rgba(255, 107, 74, 0.35)',
        'halo-card': '0 4px 20px -2px rgba(0, 0, 0, 0.5), 0 0 10px 0 rgba(255, 107, 74, 0.08)',
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "Segoe UI",
          "Roboto",
          "Helvetica",
          "Arial",
          "sans-serif",
        ],
        mono: [
          "SF Mono",
          "Menlo",
          "Monaco",
          "Consolas",
          "Courier New",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
}
