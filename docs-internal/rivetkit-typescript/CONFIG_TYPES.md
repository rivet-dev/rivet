# Config Types

## Inferred types 

- All types must be included in `ActorTypes` so the user can hardcode types

- If using input parameters for inferring types, they must be raw parameters. They also must be the last parameter. e.g.:

    ```typescript
    // DO NOT DO THIS:
    // It's hard for users to infer TConnParams because they would have to import & use an extra type
    onAuth: (opts: OnAuthOpts<TConnParams>) => TAuthData,

    // DO NOT DO THIS:
    // If you only want to access `opts`, you'll have to also define `params`
    onAuth: (params: TConnParams, opts: OnAuthOpts) => TAuthData,

    // DO THIS:
    // This allows you to not accept `params` and only access `opts`
    onAuth: (opts: OnAuthOpts, params: TConnParams) => TAuthData,
    ```

- When inferring via return data, you must use a union. e.g.:

    ```typescript
    { state: TState } | { createState: () => TState } | undefined
    ```

