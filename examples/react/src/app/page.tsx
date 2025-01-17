"use client";

import styles from "./page.module.css";
import { SimpleChat } from "./simple-chat";
import { ActorClientProvider } from "./use-actor";
import { Client } from "@rivet-gg/actor-client";

const actorClient = new Client(process.env.NEXT_PUBLIC_ACTOR_MANAGER_URL!);

export default function Home() {
  return (
    <div className={styles.page}>
      <main className={styles.main}>
        <div>
          <img src="/logo.svg" alt="Rivet Logo" className={styles.logo} />
          <h1>Rivet Actors &times; React</h1>
        </div>

        <section>
          <p>
            This chat is powered by Rivet Actors and React. Open a new tab to
            see changes in real-time. Send messages and receive them.
          </p>
          <ActorClientProvider client={actorClient}>
            <SimpleChat />
          </ActorClientProvider>
        </section>
      </main>
    </div>
  );
}
