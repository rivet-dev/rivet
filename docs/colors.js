// const { default: rivet } = require();

// See https://tailwindcss.com/docs/customizing-colors#generating-colors
// Generate colors at https://uicolors.app/create
module.exports = ({ theme }) => ({
  charcole: {
    // Recommended 950
    50: '#f6f6f6',
    100: '#e7e7e7',
    200: '#d1d1d1',
    300: '#b0b0b0',
    400: '#888888',
    500: '#6d6d6d',
    600: '#5d5d5d',
    700: '#4f4f4f',
    800: '#454545',
    900: '#3d3d3d',
    950: '#090909'
  },
  cream: {
    // Recommended 100
    50: '#fcf7f0',
    100: '#faf0e4',
    200: '#f1d4b7',
    300: '#e8b889',
    400: '#dd925a',
    500: '#d67539',
    600: '#c75f2f',
    700: '#a64a28',
    800: '#853d27',
    900: '#6b3323',
    950: '#3a1810'
  },
  red: {
    // Recommended 600
    50: '#fff0f0',
    100: '#ffdddd',
    200: '#ffc0c0',
    300: '#ff9494',
    400: '#ff5757',
    500: '#ff2323',
    600: '#ff0000',
    700: '#d70000',
    800: '#b10303',
    900: '#920a0a',
    950: '#500000'
  },
  orange: {
    // Recommended 600
    50: '#fff6ec',
    100: '#ffecd3',
    200: '#ffd4a5',
    300: '#ffb56d',
    400: '#ff8a32',
    500: '#ff690a',
    600: '#ff4f00',
    700: '#cc3602',
    800: '#a12b0b',
    900: '#82260c',
    950: '#461004'
  },
  yellow: {
    // Recommended 500
    50: '#fffdea',
    100: '#fff8c5',
    200: '#fff285',
    300: '#ffe446',
    400: '#ffd31b',
    500: '#ffb200',
    600: '#e28800',
    700: '#bb5f02',
    800: '#984908',
    900: '#7c3c0b',
    950: '#481e00'
  },
  green: {
    50: '#f2fbf2',
    100: '#e2f7e1',
    200: '#c4eec4',
    300: '#96e095',
    400: '#5fc95f',
    500: '#38a938',
    600: '#2a8f2a',
    700: '#247125',
    800: '#215a22',
    900: '#1d4a1e',
    950: '#0b280c'
  },
  blue: {
    // Recommended 500
    50: '#eff8ff',
    100: '#def0ff',
    200: '#b6e2ff',
    300: '#75cdff',
    400: '#2cb5ff',
    500: '#008ee3',
    600: '#007ad4',
    700: '#0061ab',
    800: '#00528d',
    900: '#064574',
    950: '#042b4d'
  },
  pink: {
    // Recommended 400
    50: '#fef1fb',
    100: '#fee5f9',
    200: '#ffcbf5',
    300: '#ffa1eb',
    400: '#ff54d6',
    500: '#fa3ac7',
    600: '#ea18a7',
    700: '#cc0a89',
    800: '#a80c70',
    900: '#8c0f5f',
    950: '#560137'
  },
  purple: {
    // Recommended 700
    50: '#f5f0ff',
    100: '#ede4ff',
    200: '#ddcdff',
    300: '#c5a6ff',
    400: '#ab73ff',
    500: '#943bff',
    600: '#8c14ff',
    700: '#7a00f3',
    800: '#6c01d6',
    900: '#5903af',
    950: '#360077'
  },
  wistful: {
    // Recommended 400
    50: '#f2f5fb',
    100: '#e8edf7',
    200: '#d5def0',
    300: '#bcc7e5',
    400: '#a1acd9',
    500: '#8891cc',
    600: '#7074bb',
    700: '#5e5fa4',
    800: '#4e5085',
    900: '#44476b',
    950: '#28293e'
  },

  // copied from '@rivet-gg/components/tailwind-base'.theme.extend.colors
  border: 'hsl(var(--border))',
  input: 'hsl(var(--input))',
  ring: 'hsl(var(--ring))',
  background: {
    DEFAULT: 'hsl(var(--background))',
    main: 'hsl(var(--background-main))'
  },
  foreground: 'hsl(var(--foreground))',
  primary: {
    DEFAULT: 'hsl(var(--primary))',
    foreground: 'hsl(var(--primary-foreground))'
  },
  secondary: {
    DEFAULT: 'hsl(var(--secondary))',
    foreground: 'hsl(var(--secondary-foreground))'
  },
  destructive: {
    DEFAULT: 'hsl(var(--destructive))',
    foreground: 'hsl(var(--destructive-foreground))'
  },
  warning: {
    DEFAULT: 'hsl(var(--warning))',
    foreground: 'hsl(var(--warning-foreground))'
  },
  muted: {
    DEFAULT: 'hsl(var(--muted))',
    foreground: 'hsl(var(--muted-foreground))',
    destructive: 'hsl(var(--muted-destructive))'
  },
  accent: {
    DEFAULT: 'hsl(var(--accent))',
    foreground: 'hsl(var(--accent-foreground))'
  },
  popover: {
    DEFAULT: 'hsl(var(--popover))',
    foreground: 'hsl(var(--popover-foreground))'
  },
  card: {
    DEFAULT: 'hsl(var(--card))',
    foreground: 'hsl(var(--card-foreground))'
  }
});
