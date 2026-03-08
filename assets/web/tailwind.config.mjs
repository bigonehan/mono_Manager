/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        border: "hsl(20 14% 83%)",
        input: "hsl(20 14% 83%)",
        ring: "hsl(160 67% 34%)",
        background: "hsl(36 60% 97%)",
        foreground: "hsl(160 31% 14%)",
        primary: {
          DEFAULT: "hsl(160 67% 34%)",
          foreground: "hsl(0 0% 100%)"
        },
        secondary: {
          DEFAULT: "hsl(160 28% 92%)",
          foreground: "hsl(160 31% 14%)"
        },
        muted: {
          DEFAULT: "hsl(36 18% 91%)",
          foreground: "hsl(160 13% 35%)"
        },
        accent: {
          DEFAULT: "hsl(20 76% 54%)",
          foreground: "hsl(0 0% 100%)"
        },
        destructive: {
          DEFAULT: "hsl(6 78% 57%)",
          foreground: "hsl(0 0% 100%)"
        },
        card: {
          DEFAULT: "hsl(0 0% 100%)",
          foreground: "hsl(160 31% 14%)"
        }
      },
      borderRadius: {
        lg: "0.75rem",
        md: "0.5rem",
        sm: "0.375rem"
      }
    }
  },
  plugins: []
};
