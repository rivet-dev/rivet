'use client';

import { useState } from "react";
import { Icon } from "@rivet-gg/icons";

type ExampleTab = "ai" | "crdt" | "chat" | "database" | "rate" | "stream" | "game" | "sync";
type StateTab = "memory" | "sqlite";

export function CodeSnippetsSection() {
  const [activeExample, setActiveExample] = useState<ExampleTab>("ai");
  const [activeState, setActiveState] = useState<StateTab>("memory");

  const examples = [
    { id: "ai" as ExampleTab, icon: "robot", title: "AI Agent" },
    { id: "crdt" as ExampleTab, icon: "file-pen", title: "Collaborative Document (CRDT)" },
    { id: "chat" as ExampleTab, icon: "message", title: "Chat Room" },
    { id: "database" as ExampleTab, icon: "database", title: "Per-User Databases" },
    { id: "rate" as ExampleTab, icon: "gauge-high", title: "Rate Limiter" },
    { id: "stream" as ExampleTab, icon: "wave-sine", title: "Stream Processing" },
    { id: "game" as ExampleTab, icon: "gamepad", title: "Multiplayer Game" },
    { id: "sync" as ExampleTab, icon: "rotate", title: "Local-First Sync" },
  ];

  const getActorCode = (example: ExampleTab, state: StateTab) => {
    if (example === "ai") {
      return state === "memory" ? `import { actor } from "rivetkit";
import { generateText, tool } from "ai";
import { openai } from "@ai-sdk/openai";

export type Message = { 
  role: "user" | "assistant"; 
  content: string; 
  timestamp: number; 
}

const aiAgent = actor({
  // State is automatically persisted
  state: { 
    messages: [] as Message[]
  },

  actions: {
    // Get conversation history
    getMessages: (c) => c.state.messages,

    // Send a message to the AI and get a response
    sendMessage: async (c, userMessage: string) => {
      // Add user message to conversation
      const userMsg: Message = { 
        role: "user", 
        content: userMessage,
        timestamp: Date.now()
      };
      c.state.messages.push(userMsg);
      
      // Generate AI response using Vercel AI SDK
      const { text } = await generateText({
        model: openai("o3-mini"),
        prompt: userMessage,
        messages: c.state.messages,
      });
      
      // Add AI response to conversation
      const assistantMsg: Message = { 
        role: "assistant", 
        content: text, 
        timestamp: Date.now() 
      };
      c.state.messages.push(assistantMsg);
      
      // Broadcast to all connected clients
      c.broadcast("messageReceived", assistantMsg);
      
      return assistantMsg;
    },
  }
});

export default aiAgent;` : `import { actor, drizzle } from "rivetkit";
import { sql } from "drizzle-orm";
import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";
import { messages } from "./schema";

const aiAgent = actor({
  sql: drizzle(),

  actions: {
    getMessages: async (c) => {
      return await c.db.select().from(messages)
        .orderBy(messages.timestamp);
    },

    sendMessage: async (c, userMessage: string) => {
      // Insert user message
      await c.db.insert(messages).values({
        role: "user",
        content: userMessage,
        timestamp: Date.now()
      });
      
      // Get conversation history
      const history = await c.db.select().from(messages)
        .orderBy(messages.timestamp);
      
      // Generate AI response
      const { text } = await generateText({
        model: openai("o3-mini"),
        prompt: userMessage,
        messages: history,
      });
      
      // Insert AI response
      const assistantMsg = await c.db.insert(messages)
        .values({
          role: "assistant",
          content: text,
          timestamp: Date.now()
        }).returning();
      
      c.broadcast("messageReceived", assistantMsg[0]);
      return assistantMsg[0];
    },
  }
});

export default aiAgent;`;
    }
    
    // Add other examples here - simplified for brevity
    return `// ${example} example with ${state} state coming soon...`;
  };

  const getReactCode = (example: ExampleTab) => {
    if (example === "ai") {
      return `import { createClient } from "rivetkit/client";
import { createReactRivetKit } from "@rivetkit/react";
import { useState, useEffect } from "react";

const client = createClient<App>("http://localhost:8080");
const { useActor, useActorEvent } = createReactRivetKit(client);

export function AIAssistant() {
  const [{ actor }] = useActor("aiAgent", { 
    tags: { conversationId: "default" } 
  });
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  // Load initial messages
  useEffect(() => {
    if (actor) {
      actor.getMessages().then(setMessages);
    }
  }, [actor]);

  // Listen for real-time messages
  useActorEvent({ actor, event: "messageReceived" }, (message) => {
    setMessages(prev => [...prev, message as Message]);
    setIsLoading(false);
  });

  const handleSendMessage = async () => {
    if (actor && input.trim()) {
      setIsLoading(true);
      
      // Add user message to UI immediately
      const userMessage = { role: "user", content: input };
      setMessages(prev => [...prev, userMessage]);
      
      // Send to actor
      await actor.sendMessage(input);
      setInput("");
    }
  };

  return (
    <div className="ai-chat">
      <div className="messages">
        {messages.map((msg, i) => (
          <div key={i} className={\`message \${msg.role}\`}>
            <div className="avatar">
              {msg.role === "user" ? "👤" : "🤖"}
            </div>
            <div className="content">{msg.content}</div>
          </div>
        ))}
      </div>
      
      <div className="input-area">
        <input
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyPress={e => e.key === "Enter" && handleSendMessage()}
          placeholder="Ask the AI assistant..."
          disabled={isLoading}
        />
        <button 
          onClick={handleSendMessage}
          disabled={isLoading || !input.trim()}
        >
          Send
        </button>
      </div>
    </div>
  );
}`;
    }
    
    return `// ${example} React component coming soon...`;
  };

  return (
    <div className="mx-auto max-w-7xl">
      <div className="text-center mb-16">
        <h2 className="text-4xl sm:text-5xl font-700 text-white mb-6">
          Reconsider What Your Backend Can Do
        </h2>
        <p className="text-lg sm:text-xl font-500 text-white/60 max-w-3xl mx-auto">
          Build powerful applications with Rivet Actors
        </p>
      </div>

      <div className="bg-white/5 backdrop-blur border border-white/10 rounded-2xl overflow-hidden">
        {/* Tabs */}
        <div className="border-b border-white/10">
          {/* Example Tabs */}
          <div className="px-6 py-4 border-b border-white/5">
            <div className="flex items-center gap-1 text-sm text-white/40 mb-3">
              <span className="font-medium">Example</span>
            </div>
            <div className="flex gap-2 overflow-x-auto scrollbar-hide">
              {examples.map((example) => (
                <button
                  key={example.id}
                  onClick={() => setActiveExample(example.id)}
                  className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all duration-200 ${
                    activeExample === example.id
                      ? "bg-white/10 text-white border border-white/20"
                      : "text-white/60 hover:text-white/80 hover:bg-white/5"
                  }`}
                >
                  <Icon icon={example.icon as any} className="w-3.5 h-3.5" />
                  {example.title}
                </button>
              ))}
            </div>
          </div>

          {/* State Tabs */}
          <div className="px-6 py-3">
            <div className="flex items-center gap-4">
              <span className="text-sm text-white/40 font-medium">State</span>
              <div className="flex gap-2">
                <button
                  onClick={() => setActiveState("memory")}
                  className={`px-3 py-1 rounded text-sm font-medium transition-all duration-200 ${
                    activeState === "memory"
                      ? "bg-white/10 text-white"
                      : "text-white/60 hover:text-white/80"
                  }`}
                >
                  JavaScript
                </button>
                <button
                  onClick={() => setActiveState("sqlite")}
                  className={`px-3 py-1 rounded text-sm font-medium transition-all duration-200 relative ${
                    activeState === "sqlite"
                      ? "bg-white/10 text-white"
                      : "text-white/60 hover:text-white/80"
                  }`}
                >
                  SQLite
                  <span className="ml-2 text-xs text-orange-400 font-normal">Available In July</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Code Panels */}
        <div className="grid grid-cols-1 lg:grid-cols-2">
          {/* Server Code */}
          <div className="border-r border-white/10">
            <div className="px-6 py-4 border-b border-white/5 bg-white/5">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-white">actor.ts</span>
                <span className="text-xs text-white/50">Runs on the server</span>
              </div>
            </div>
            <div className="p-6">
              <pre className="text-sm text-white/80 overflow-auto max-h-96 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-white/20">
                <code>{getActorCode(activeExample, activeState)}</code>
              </pre>
            </div>
          </div>

          {/* Client Code */}
          <div>
            <div className="px-6 py-4 border-b border-white/5 bg-white/5">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-white">App.tsx</span>
                <span className="text-xs text-white/50">Runs in the browser</span>
              </div>
            </div>
            <div className="p-6">
              <pre className="text-sm text-white/80 overflow-auto max-h-96 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-white/20">
                <code>{getReactCode(activeExample)}</code>
              </pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
