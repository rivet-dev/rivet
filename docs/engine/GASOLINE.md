# Gasoline

Gasoline (at engine/packages/gasoline) is the durable execution engine running most persistent things on Rivet Engine.

Gasoline consists of:
- Workflows - Similar to the concept of actors (not Rivet Actors) which can sleep (be removed from memory) when not in use
- Signals - Facilitates intercommunication between workflow <-> workflow and other services (such as api) -> workflow
- Messages - Ephemeral "fire-and-forget" communication between workflows -> other services
- Activities - Individual steps in a workflow, each can be individually retried upon failure and "replayed" instead of re-executed with every workflow run
- Operations - Thin wrappers around native rust functions. Provided for clean interop with the Gasoline ecosystem

