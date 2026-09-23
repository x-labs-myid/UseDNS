/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#17243a",
        navy: "#10264a",
        azure: "#087be5",
        cyan: "#20c8ea",
        paper: "#ffffff",
        line: "#dceaf7",
      },
      boxShadow: {
        note: "0 18px 50px -28px rgba(16, 70, 130, 0.35)",
        float: "0 20px 60px -30px rgba(8, 123, 229, 0.42)",
      },
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
        note: ["'Segoe Print'", "'Bradley Hand'", "cursive"],
      },
    },
  },
  plugins: [],
};
