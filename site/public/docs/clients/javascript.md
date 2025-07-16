# Node.js & Bun

The Rivet JavaScript client allows you to connect to and interact with actors from browser and Node.js applications.

## Quickstart

    Create a new Node.js project with TypeScript support:

      ```sh npm
      mkdir my-app
      cd my-app
      npm init -y
      npm pkg set type=module
      ```
      
      ```sh pnpm
      mkdir my-app
      cd my-app
      pnpm init
      pnpm pkg set type=module
      ```
      
      ```sh yarn
      mkdir my-app
      cd my-app
      yarn init -y
      yarn pkg set type=module
      ```
      
      ```sh bun
      mkdir my-app
      cd my-app
      bun init -y
      ```

    Install the Rivet client and Node.js platform packages:

      ```sh npm
      npm install @rivetkit/actor
      ```
      
      ```sh pnpm
      pnpm add @rivetkit/actor
      ```
      
      ```sh yarn
      yarn add @rivetkit/actor
      ```
      
      ```sh bun
      bun add @rivetkit/actor
      ```

    Create a file `src/client.ts` in your project to connect to your actor:

    ```typescript src/client.ts
    async function main() 

    main().catch(console.error);
    ```

    In a separate terminal, run your client code:

      ```sh npm
      npx tsx src/client.ts
      ```
      
      ```sh pnpm
      pnpm exec tsx src/client.ts
      ```
      
      ```sh yarn
      yarn tsx src/client.ts
      ```
      
      ```sh bun
      bun run src/client.ts
      ```

    You should see output like:
    ```
    Event: 5
    Action: 5
    ```

    Run it again to see the state update.

## Next Steps

For more information on communicating with actors, including event handling and RPC calls, see [Communicating with Actors](/docs/actors/communicating-with-actors).