# Rivet Website

[rivet.dev](https://rivet.dev)

## Project structure

```
public/           Static assets
  fonts/
  icons/          Favicons
  promo/          Assets used for promotional marketing
scripts/
src/
  content/        This is where all MDX content lives.
  authors/
  components/     Reusable components
  generated/      Content generated from the rivet-dev/engine repo with scripts/generate*.js
  lib/            Helper libraries used at build time
  mdx/            "
  pages/          MDX & JSX content to serve as pages
  styles/         Static stylesheets (seldom used)
```

## Developing

```bash
cd site
pnpm install
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser to view the website.

## License

This site template is a commercial product and is licensed under the [Tailwind UI license](https://tailwindui.com/license).
