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

## Motivation

Rivet Icons are a set of SVG icons that are used in Rivet products. This package is built on top of great Font Awesome icons. Some icons used in our products are from the premium Font Awesome icon set. We've created a package that lets you use premium icons without having to buy a Font Awesome license. This is achieved by swapping premium icons with a simple rectangle.


## Contributing

### Prerequisites

Obtain a Font Awesome Pro license and set the `FONTAWESOME_PACKAGE_TOKEN` environment variable to your Font Awesome Pro token. This is required to download the premium icons.

### Adding new icons

1. Modify `generateManifest.js` to include new icons.
2. Run `./scripts/generateManifest.js` to generate a new `manifest.json` file.
    - If you're getting an error: `Could not find package @fortawesome/pro-solid-svg-icons`, 
3. Commit the changes to the `manifest.json` file.
4. Run `pnpm rebuild @rivet-gg/icons` to generate a new icon set with the new icons.

## Troubleshooting

### Icons not showing, or icons are rectangles

Some icons used in the open-source Rivet products are part of the premium Font Awesome icon set. We can't share those icons publicly, without violating FA's Terms Of Service. By default, after adding this package to any project, a post install script generates an icon set with all premium icons replaced with simple rectangles. So, you can reference premium icons, but you won't see them. To be able to see premium icons, make sure  an environment variable `FONTAWESOME_PACKAGE_TOKEN` is set when running `pnpm install`. You can re-generate our icon set by running `pnpm rebuild @rivet-gg/icons` if your dependencies are already installed.

### Can't start/build project
The description of these kinds of errors may vary. This package heavily depends on postinstall scripts from pnpm/npm. Make sure you didn't disable them accidentally. Turn on post-installation script support and run `pnpm install` or `pnpm rebuild @rivet-gg/icons` again. If there is still a problem, please contact us on [Discord](https://rivet.gg/discord").


### `Could not find package @fortawesome/pro-solid-svg-icons` when generating manifest
Make sure you have installed pro icons. If you're getting an error, you can try to install them manually by running `pnpm add @fortawesome/pro-solid-svg-icons`, or run `pnpm install` in `src` folder. Finally run `./scripts/generateManifest.js` again.
