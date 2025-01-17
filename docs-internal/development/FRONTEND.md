# Frontend Development

## Overview

The "Frontend" part of this repository consist of:
- Hub (hub.rivet.gg) ([frontend/apps/hub](https://github.com/rivet-gg/rivet/tree/main/frontend/apps/hub))
- Site (rivet.gg) ([site](https://github.com/rivet-gg/rivet/tree/main/site))
- Actors SDK ([sdks/actor](https://github.com/rivet-gg/rivet/tree/main/sdks/actor))

For more information check README.md in the desired directory.

## Development

### Prerequisites
- Node.js (>=18.19)
- Yarn (use [Corepack](https://nodejs.org/api/corepack.html))


### Setup

1. Clone the repository
2. Install dependencies
    ```bash
    yarn
    ```
3. Start the development server
    ```bash
    yarn start
    ```

### Build
1. Build the project
    ```bash
    yarn build
    ```
    1. You can define what app should be built using the `--filter` flag
        ```bash
        yarn build --filter=./site
        ```

### Code Quality

Don't worry about code quality, we got you covered! We use https://autofix.ci/ to automatically fix and format your code.

