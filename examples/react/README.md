# Rivet Actors x React

This is a [Next.js](https://nextjs.org) project bootstrapped with [`create-next-app`](https://nextjs.org/docs/app/api-reference/cli/create-next-app), showcasing the use of by [Rivet Actors](https://rivet.gg).

## Overview

### Simple Chat

It's a simple chat implementation using Rivet's `Actor` API. It demonstrates how to use the `Actor` API to create a chat application. The chat application is a simple chat room where users can send messages and see messages from other users in real-time. **Does not include any authentication or authorization.**

## Getting Started

1. Go through the [Initial Setup](https://rivet.gg/docs/setup) documentation to get a basic understanding of how it works.
2. Create `.env` file in the root of the project and add the following environment variables:
   ```bash
    NEXT_PUBLIC_ACTOR_MANAGER_URL=YOUR ACTOR MANAGER URL FROM INITIAL SETUP
   ```
3. Follow instructions in the [Actors ReadMe](./actor/readme.md) to setup and deploy the actors.
4. Run the development server `npm run dev`
