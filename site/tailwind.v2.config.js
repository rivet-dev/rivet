// Used by the new design

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{ts,tsx,jsx,js,mdx,md}', './node_modules/@rivet-gg/components/**/*.{ts,tsx}'],
  theme: {
    extend: {
      gridTemplateColumns: {
        docs: 'minmax(0, 20rem) 65ch minmax(0, 20rem)'
      },
      typography: {
        DEFAULT: {
          css: {
            '--tw-prose-invert-body': 'hsl(var(--foreground))',
            '--tw-prose-invert-headings': 'hsl(var(--foreground))',
            '--tw-prose-invert-lead': 'hsl(var(--foreground))',
            '--tw-prose-invert-links': 'hsl(var(--foreground))',
            '--tw-prose-invert-bold': 'hsl(var(--foreground))',
            '--tw-prose-invert-counters': 'hsl(var(--foreground))',
            '--tw-prose-invert-bullets': 'hsl(var(--foreground))',
            '--tw-prose-invert-hr': 'hsl(var(--border))',
            '--tw-prose-invert-quotes': 'hsl(var(--foreground))',
            '--tw-prose-invert-quote-borders': 'hsl(var(--border))',
            '--tw-prose-invert-captions': 'hsl(var(--foreground))',
            '--tw-prose-invert-code': 'hsl(var(--foreground))',
            '--tw-prose-invert-pre-code': 'hsl(var(--foreground))',
            '--tw-prose-invert-pre-bg': 'rgb(0 0 0 / 50%)',
            '--tw-prose-invert-th-borders': 'hsl(var(--border))',
            '--tw-prose-invert-td-borders': 'hsl(var(--border))'
          }
        }
      },
      spacing: {
        header: 'var(--header-height, 3.5rem)'
      },
      scrollMargin: {
        header: 'calc(var(--header-height, 3.5rem) + 1rem)'
      },
      maxHeight: {
        content: 'calc(100vh - var(--header-height, 3.5rem))'
      }
    }
  },
  presets: [require('@rivet-gg/components/tailwind-base')],
  plugins: [require('@tailwindcss/typography')]
};
