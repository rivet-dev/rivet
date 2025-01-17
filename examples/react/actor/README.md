# Rivet Actors

Powered by [Rivet](https://rivet.gg).

## Overview

### Simple Chat

It's a simple chat implementation using Rivet's `Actor` API. It demonstrates how to use the `Actor` API to
create a chat application. The chat application is a simple chat room where users can send messages and see
messages from other users in real-time. **Does not include any authentication or authorization.**

## Prerequisites

- [Rivet CLI](https://rivet.gg/docs/setup)
- [Deno](https://deno.land/)

## File structure

- `rivet.json` Configuration file for deploying the actor
- `deno.json` Configuration file for dependencies for the actor
- `simple-chat.ts` Simple chat implementation

## Deploying

```sh
rivet login
rivet deploy
```
