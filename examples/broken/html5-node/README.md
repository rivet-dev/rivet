# Sandbox

## Quickstart Tutorial

View the documentation [here](https://rivet.gg/docs/html5/tutorials/quickstart).

## Prerequisites

- Rivet CLI
- NodeJS
- Open ports: 8080 (client), 7777 (game server), 6420 (Rivet)

## Instructions

### Develop

```
npm install
npm run dev
```

Open [http://127.0.0.1:8080](http://127.0.0.1:8080).

### Deploy

```
rivet login
rivet deploy prod
```

#### Connecting

In the client served by `npm run dev`:

1. Run `rivet backend get-endpoint prod` and copy this value to the _Endpoint_ field.
2. Run `rivet backend get-current-version prod` and copy this value to the _Game Version_ field.

