/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        void: "#0A0A0A",
        surface: "#141414",
        elevated: "#1A1A1A",
        console: "#080808",
        "surface-card": "#0a0a0c",
        mahakali: "#E11D48",
        "mahakali-hover": "#FF3B6A",
        "mahakali-glow": "rgba(225,29,72,0.22)",
        gold: "#C9A84C",
        ink: "#F5F0EB",
        body: "#A8A39E",
        muted: "#6B6661",
        "check-border": "#4A4540",
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        display: ["Playfair Display", "Fraunces", "serif"],
        mono: ["JetBrains Mono", "Geist Mono", "monospace"],
      },
      borderRadius: {
        xs: "4px",
        sm: "6px",
        md: "8px",
        lg: "12px",
        xl: "16px",
        pill: "9999px",
      },
    },
  },
  plugins: [],
};
