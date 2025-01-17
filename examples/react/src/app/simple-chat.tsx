"use client";
import { ActorHandle } from "@rivet-gg/actor-client";
import { useEffect, useState, useSyncExternalStore } from "react";
import type { default as ChatActor } from "../../actor/simple-chat";
import { useActor } from "./use-actor";

type ChatHandle = ActorHandle<ChatActor>;

class ChatStore {
  #handle: ChatHandle;
  #messages: string[] = [];

  #listeners: (() => void)[] = [];

  constructor(handle: ChatHandle) {
    this.#handle = handle;

    this.fetchMessages().then(() => {
      this.#update();
    });
  }

  getMessages = () => {
    return this.#messages;
  };

  sendMessage = async (message: string) => {
    this.#messages = await this.#handle.sendMessage(message);
    this.#update();
  };

  fetchMessages = async () => {
    this.#messages = await this.#handle.getMessages();
  };

  subscribe = (cb: () => void) => {
    this.#listeners.push(cb);
    const unsub = this.#handle.on("newMessage", (msgs: string[]) => {
      this.#messages = msgs;
      this.#update?.();
    });

    return () => {
      unsub();
      this.#listeners = this.#listeners.filter((l) => l !== cb);
    };
  };

  #update = () => {
    this.#listeners?.forEach((cb) => cb());
  };
}

function useChatMessages(store: ChatStore) {
  return useSyncExternalStore(store.subscribe, store.getMessages);
}

export function SimpleChat() {
  const state = useActor<ChatActor>({ name: "simple-chat" });

  if (!("actor" in state) || state.isLoading) {
    if ("error" in state) {
      return <div>Error while loading actor, see console for more details</div>;
    }
    return <div>Loading...</div>;
  }

  return (
    <>
      <ChatContent actor={state.actor} />
    </>
  );
}

function ChatContent({ actor }: { actor: ChatHandle }) {
  const [store] = useState(() => new ChatStore(actor));
  const messages = useChatMessages(store);

  return (
    <>
      <ChatMessageForm onSubmit={store.sendMessage} />
      <ChatMessages messages={messages} />
    </>
  );
}

function ChatMessages({ messages }: { messages: string[] }) {
  return (
    <div>
      {messages.map((msg, i) => (
        <div key={i}>{msg}</div>
      ))}
    </div>
  );
}

function ChatMessageForm({
  onSubmit,
}: {
  onSubmit: (message: string) => void;
}) {
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const data = new FormData(e.currentTarget);
        onSubmit(data.get("message") as string);
        e.currentTarget.reset();
      }}
    >
      <input type="text" name="message" />
      <button type="submit">Send</button>
    </form>
  );
}
