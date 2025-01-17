"use client";
import { ActorHandle, Client } from "@rivet-gg/actor-client";
import { createContext, use, useEffect, useState } from "react";

const ActorClientContext = createContext<Client | null>(null);

export function ActorClientProvider({
  client,
  children,
}: {
  children: React.ReactNode;
  client: Client;
}) {
  return (
    <ActorClientContext.Provider value={client}>
      {children}
    </ActorClientContext.Provider>
  );
}

type ActorState<T> =
  | { isLoading: true }
  | { isLoading: false }
  | { isLoading: false; actor: ActorHandle<T> }
  | { isLoading: false; error: unknown };

export function useActor<T>(...params: Parameters<Client["get"]>) {
  const manager = use(ActorClientContext);
  if (!manager) {
    throw new Error("useActor must be used within an ActorManager");
  }

  const [state, setState] = useState<ActorState<T>>({ isLoading: false });

  async function create(...params: Parameters<Client["get"]>) {
    if (state.isLoading) {
      return;
    }
    dispose();
    setState({ isLoading: true });
    try {
      const actor = await manager?.get<T>(...params);
      if (!actor) {
        return setState({
          isLoading: false,
          error: new Error("Actor not found"),
        });
      }
      setState({ isLoading: false, actor });
    } catch (e) {
      setState({ isLoading: false, error: e });
    }
  }

  function dispose() {
    if ("actor" in state) {
      state.actor.dispose();
      setState({ isLoading: false });
    }

    setState({ isLoading: false });
  }

  useEffect(() => {
    create(...params);
    return () => {
      dispose();
    };
  }, []);

  return { create, dispose, ...state };
}
