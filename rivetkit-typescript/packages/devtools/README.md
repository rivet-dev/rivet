# RivetKit DevTools


## Contributing

To contribute to the RivetKit DevTools package, please follow these steps:

1. Set up assets server for the `dist` folder:
    ```bash
    pnpm dlx serve dist
    ```

2. Set your `CUSTOM_RIVETKIT_DEVTOOLS_URL` environment variable to point to the assets server (default is `http://localhost:3000`):
    ```bash
    export CUSTOM_RIVETKIT_DEVTOOLS_URL=http://localhost:5000
    ```

    This will ensure that the RivetKit will use local devtool assets instead of fetching them from the CDN.

3. In another terminal, run the development build:
    ```bash
    pnpm dev
    ```

    or run the production build:
    ```bash
    pnpm build
    ```
