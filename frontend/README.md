<p align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="./apps/docs/public/icon-text-white.svg">
        <img src="./apps/docs/public/icon-text-black.svg">
    </picture>
</p>
<h1 align="center">Rivet Hub</h1>
<p align="center">
    <a href="https://rivet.gg/discord"><img src="https://img.shields.io/discord/822914074136018994"></a>
</p>

_This is the source code for the [Rivet Hub](https://hub.rivet.gg) frontend_

**Also check out the [Rivet backend](https://github.com/rivet-gg/rivet) for issues, documentation, and more.**

## Requirements

-   [NodeJS](https://nodejs.org/en/) v16+
-   [Yarn](https://yarnpkg.com/)
-   [Git LFS](https://git-lfs.com/) (make sure this is installed before cloning repo)

## Getting started

Run the following command:

```bash
yarn start
```

This will open `http://localhost:5080` in your browser. By default, this will connect to Rivet's staging servers (https://staging2.gameinc.io).

## Environment variables configuration

1. Copy `apps/hub/.env.example` to `apps/hub/.env.development.local` and change variables if needed.

## Developing with self-hosted backend

> **Where do I self-host the backend?**
>
> See our backend repo [here](https://github.com/rivet-gg/rivet).

### API Endpoint Configuration 

To configure the hub to connect to your own server (for example, `mydomain.com`), update the `.env` file in `apps/hub` folder to include the following:

```
VITE_APP_API_URL=https://api.mydomain.com
```

### Backend configuration

If self-hosting your own backend, the default configuration assumes the hub will be hosted at `https://hub.{your domain}`. You may want to change the following namespace config parameters:

**`dns.hub_origin`** (default: `https://hub.{main domain}`)

This configures the origin that the API will build URLs for.

For example, `identity.getProfile` returns a URL to the hub profile in `identity.external.profile`.

**`rivet.api.hub_origin_regex`** (default: `^https://hub\\.{main domain}$`)

This configurs the CORS origin regex. This allows you to secure where requests can come from to your hub.

If you are running your own cluster for development, consider updating the CORS rules to include `localhost` similar to below.

## CORS

`staging2.gameinc.io` CORS is specially configured to have the following origin regex:

```regex
^https?://(hub\\.rivet\\.gg|[^\\.]+\\.rivet-hub\\.pages\\.dev|localhost:\\d+|192\\.168\\.\\d+\\.\\d+:\\d+|0\\.0\\.0\\.0\\.:\\d+)$
```

This lets you develop with the hub using localhost.

## Filing issues

All Hub issues are managed in the [main Rivet repository](https://github.com/rivet-gg/rivet).

## 📚 Documentation

### Technologies

- [React](https://reactjs.org/)
- [TanStack Router](https://tanstack.com/router)
- [TanStack Query](https://tanstack.com/query)
- [Tailwind CSS](https://tailwindcss.com/)
- [Storybook](https://storybook.js.org/)
- [TypeScript](https://www.typescriptlang.org/)
- [Yarn](https://yarnpkg.com/)
- [Biome](https://biomejs.dev/)
- [shadcn/ui](https://ui.shadcn.com/)
- [React Hook Form](https://react-hook-form.com/)
- [Zod](https://zod.dev/)
- [Turbo](https://turbo.build/)

### File structure

- `apps/` - Contains the main applications.

  - `hub/` - The Rivet Hub application.

    - `src/` - Contains the source code.
      - `components/` - Contains the components used in the application.
      - `contexts/` - Contains the React contexts
      - `forms/` - Contains the forms.
      - `layouts/` - Contains the layouts, used in routing.
      - `queries/` - Contains the TanStack queries and mutations configs.
      - `routes/` - Contains the routes.
      - `views/` - Contains the views.

  - `docs/` - The Storybook application.

- `packages/` - Contains the shared components
  - `components/` - Contains the shared components.
  - `icons/` - Contains icons used by Rivet project, [read more here](packages/icons/README.md).

### Routing

The routing is done using the [TanStack Router](https://tanstack.com/router). The routes are defined in the `apps/hub/src/routes` folder. For more information, check the [TanStack Router documentation](https://tanstack.com/router). The project is using TanStack's Router Vite integration for dynamically generating route types. This means that all parameters and query parameters are typed and validated.

#### Routes

To define a route, create a new file in the `apps/hub/src/routes` folder. The route file should export a `Route` constant. See already existing routes for examples.

#### Layouts

To define a reusable layout, you can create a layout component in the `apps/hub/src/layouts` folder and use it in the route definition. See already existing layouts for examples.

### Data fetching

The data fetching is done using the [TanStack Query](https://tanstack.com/query). The queries are defined in the `apps/hub/src/queries` folder. For more information, check the [TanStack Query documentation](https://tanstack.com/query).

#### Query and mutation configs

All query and mutation configs are grouped by the business logic they are related to. For example, all queries related to the games are in the `games.ts` file. Configs define the query key, the query function, and other query options, like the query cache time or the query selector. All queries are cached by default in the local storage. To adjust the cache time, consult the [TanStack Query documentation](https://tanstack.com/query).

### Forms

The forms are created using the [React Hook Form](https://react-hook-form.com/) library. The form schemas are defined using the [Zod](https://zod.dev/) library. The forms are defined in the `apps/hub/src/forms` folder. For more information, check the [React Hook Form documentation](https://react-hook-form.com/) and the [Zod documentation](https://zod.dev/). For convenience, the form factory is used to create the forms. The form factory is defined in the `apps/hub/lib/create-schema-form.tsx` file.

### Reusable components (across the Rivet products)

The reusable components are defined in the `packages/components/src` folder. The components are grouped by the business logic they are related to. For example, all components related to the games are in the `games` folder. The components are using the [shadcn/ui](https://ui.shadcn.com/) library for styling. The atom components (like buttons, inputs, etc.) are defined in the `packages/components/src/ui`. Atom components represent the smallest components that can be used in the application, like buttons and inputs. Please keep the components as small and reusable as possible.

### Hub components

The components used in the Rivet Hub are defined in the `apps/hub/src/components` folder. The components are grouped by the business logic they are related to. For example, all components related to the games are in the `games` folder. The components are using the same technology stack as the reusable components. The components here are more specific to the Rivet Hub and are not meant to be reused across the Rivet products.

#### Views

Views are special components that are used in the routes, following MVC pattern. They are defined in the `apps/hub/src/views` folder. The views are using the components and the queries to display the data. The views are not meant to be reused across the Rivet products.

### Configuration

The project is designed to be built once and used in multiple environments. UI package uses a React context to determine environment configuration. Hub app, on the other hand, uses an HTML element to fetch the configuration and pass it to the UI elements. To configure the project put the following element in the `public/index.html` file:

```html
<script type="application/json" id="RIVET_CONFIG">
  { "apiUrl": "YOUR API URL", "assetsUrl": "YOUR ASSETS URL" }
</script>
```

and fill it with the appropriate values.

However, if you want to just deploy the project normally, supply the environment variables:

```bash
VITE_APP_API_URL=YOUR
VITE_APP_ASSETS_URL=VALUES
```

when building the project.

### Troubleshooting

#### `Failed to resolve import "./some_file.js" from "../../packages/components/dist/index.js". Does the file exist?` error

This is caused by the race condition between the Vite server (responsible for building Hub) and the Rollup build (responsible for building UI). To fix this, stop the Vite server remove `node_modules` from the `hub` directory, and run the Vite server again.

### `[postcss] Cannot find module '/your/path/to/project/node_modules/@rivet-gg/components/dist/index.cjs'` error

See the solution for the previous error.

### Unknown issue with icons, most of the icons are squares or not showing

See troubleshooting section for [@rivet-gg/icons package](packages/icons/README.md#troubleshooting).

## 🏗️ Contributing

1. Look for any issue that describes something that needs to be done - or, if
   you're willing to add a new feature, create a new issue with an appropriate
   description.
2. Submit your pull request.
3. Rivet team will review your changes.
4. Don't forget to join [Rivet's Discord](https://rivet.gg/discord) to hang out
   with the devs, or to pairprogram together!
