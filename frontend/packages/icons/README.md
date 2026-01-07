<p align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./../../apps/docs/public/icon-text-white.svg">
        <img src="./../../apps/docs/public/icon-text-black.svg">
    </picture>  
</p>
<h1 align="center">Rivet Icons</h1>
<p align="center">
    <a href="https://rivet.gg/discord"><img src="https://img.shields.io/discord/822914074136018994"></a>
</p>

## ⚠️ Legal Notice

**This package is licensed exclusively for use in Rivet products and services.** Using this package in any other product, project, or commercial application is strictly prohibited and may constitute a violation of Font Awesome's Terms of Service and intellectual property rights.

This package includes icons from Font Awesome Pro, which are licensed to Rivet for use in Rivet products only. If you wish to use Font Awesome Pro icons in your own projects, you must obtain your own license from [Font Awesome](https://fontawesome.com/plans).

## Motivation

Rivet Icons are a set of SVG icons that are used in Rivet products. This package is built on top of great Font Awesome icons. Some icons used in our products are from the premium Font Awesome icon set. We've created a package that lets you use premium icons without having to buy a Font Awesome license.

All icons (including Pro icons) are pre-generated and committed to the repository, so this package works out-of-the-box with **no Font Awesome token required** for end users.


## Contributing

### Prerequisites

**For maintainers only:** Obtain a Font Awesome Pro license and set the `FONTAWESOME_PACKAGE_TOKEN` environment variable to your Font Awesome Pro token. This is only required when vendoring new icons.

### Adding new icons

1. Ensure you have a `FONTAWESOME_PACKAGE_TOKEN` environment variable set
2. Modify [scripts/generateManifest.js](scripts/generateManifest.js) to include new icons
3. Run `./scripts/generateManifest.js` to generate a new `manifest.json` file
    - If you're getting an error about missing packages, run `pnpm install` in the `src` folder first
4. Run `pnpm vendor` to generate icon files:
    - `src/index.gen.js`
    - `src/index.gen.ts`
    - `dist/index.js`
5. Commit all changes including the generated files

## Troubleshooting

### Icons not showing up

All icons (including Font Awesome Pro icons) are pre-generated and committed to this repository. If icons aren't showing:

1. Make sure you have the peer dependencies installed:
   - `@fortawesome/fontawesome-svg-core`
   - `@fortawesome/free-solid-svg-icons`
   - `@fortawesome/free-brands-svg-icons`
   - `@fortawesome/react-fontawesome`
   - `react` and `react-dom`

2. Check that you're importing icons correctly:
   ```tsx
   import { Icon, faCheckCircle } from "@rivet-gg/icons";

   <Icon icon={faCheckCircle} />
   ```

3. If the problem persists, contact us on [Discord](https://rivet.gg/discord)

### For Maintainers: `Could not find package @fortawesome/pro-solid-svg-icons`

This error occurs when running the vendor script without access to Font Awesome Pro packages. Make sure:

1. `FONTAWESOME_PACKAGE_TOKEN` environment variable is set
2. Run `pnpm install` in the `src` folder to install FA Pro packages
3. Run `pnpm vendor` again
