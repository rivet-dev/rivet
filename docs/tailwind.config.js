const plugin = require('tailwindcss/plugin');

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,mjs,ts,tsx,jsx,mdx,json}', './node_modules/@rivet-gg/components/**/*.{ts,tsx}'],
  darkMode: 'class',
  theme: {
    fontFamily: {
      sans: ['Open Sans', 'ui-sans-serif', 'system-ui'],
      display: ['Perfectly Nineties', 'ui-sans-serif', 'system-ui'],
      pixel: ['Silkscreen', 'ui-sans-serif', 'system-ui'],
      psychotic: ['Gloria Hallelujah', 'Open Sans', 'ui-sans-serif', 'system-ui'],
      mono: [
        'ui-monospace',
        'SFMono-Regular',
        'Menlo',
        'Monaco',
        'Consolas',
        'Liberation Mono',
        'Courier New',
        'monospace'
      ],
      v2: [
        'ui-sans-serif',
        'system-ui',
        'sans-serif',
        'Apple Color Emoji',
        'Segoe UI Emoji',
        'Segoe UI Symbol',
        'Noto Color Emoji'
      ]
    },
    fontSize: {
      '3xs': ['0.5rem', { lineHeight: '1.25rem' }],
      '2xs': ['0.75rem', { lineHeight: '1.25rem' }],
      xs: ['0.8125rem', { lineHeight: '1.5rem' }],
      sm: ['0.875rem', { lineHeight: '1.5rem' }],
      base: ['1rem', { lineHeight: '1.75rem' }],
      lg: ['1.125rem', { lineHeight: '1.75rem' }],
      xl: ['1.25rem', { lineHeight: '1.75rem' }],
      '2xl': ['1.5rem', { lineHeight: '2rem' }],
      '3xl': ['1.875rem', { lineHeight: '2.25rem' }],
      '4xl': ['2.25rem', { lineHeight: '2.5rem' }],
      '5xl': ['3rem', { lineHeight: '1' }],
      '6xl': ['3.75rem', { lineHeight: '1' }],
      '7xl': ['4.5rem', { lineHeight: '1' }],
      '8xl': ['6rem', { lineHeight: '1' }],
      '9xl': ['8rem', { lineHeight: '1' }]
    },
    typography: require('./typography'),
    extend: {
      aria: {
        'current-page': "current='page'"
      },
      colors: require('./colors'),
      maxHeight: {
        'tabs-content': 'calc(100vh - 6.5rem)',
        content: 'calc(100vh - 3.5rem)'
      },
      minHeight: {
        'tabs-content': 'calc(100vh - 6.5rem)'
      },
      boxShadow: {
        glow: '0 0 4px rgb(0 0 0 / 0.1)'
      },
      maxWidth: {
        lg: '33rem',
        '2xl': '40rem',
        '3xl': '50rem',
        '5xl': '66rem',
        aside: '16rem'
      },
      opacity: {
        1: '0.01',
        2.5: '0.025',
        7.5: '0.075',
        15: '0.15'
      },
      spacing: {
        'docs-navigation': 'var(--header-height, 6.5rem)'
      },
      backgroundImage: {
        'rainbow-gradient': 'linear-gradient(to right, violet, indigo, blue, green, yellow, orange, red)',
        'light-grain': 'var(--bg-light-grain)',
        'dark-grain': 'var(--bg-dark-grain)'
      },
      gridTemplateColumns: {
        'table-of-contents': '1fr 20rem',
        'two-sidebars': 'minmax(0, 20rem) 50rem minmax(0, 20rem)',
        sidebar: '20rem 1fr'
      },
      scrollMargin: {
        'header-offset': 'calc(var(--header-height) + 4rem)'
      }
    }
  },
  plugins: [
    require('@tailwindcss/typography'),
    require('@tailwindcss/forms'),
    require('tailwindcss-animate'),
    plugin(function ({ addBase, theme }) {
      addBase({
        h1: { fontFamily: theme('fontFamily.display') }
      });
    }),
    plugin(function ({ addUtilities, theme }) {
      addUtilities({
        '.drag-none': {
          'user-drag': 'none',
          '-webkit-user-drag': 'none',
          'user-select': 'none',
          '-moz-user-select': 'none',
          '-webkit-user-select': 'none',
          '-ms-user-select': 'none'
        }
      });
    })
  ]
};
