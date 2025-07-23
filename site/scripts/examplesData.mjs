// Shared example metadata used by both generateExamples.mjs and generateReadme.mjs

export const EXAMPLE_METADATA = {
  "ai-agent": { 
    icon: "faRobot", 
    title: "AI Agent",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "chat-room": { 
    icon: "faMessage", 
    title: "Chat Room",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "crdt": { 
    icon: "faFilePen", 
    title: "Collab (Yjs)",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "game": { 
    icon: "faGamepad", 
    title: "Multiplayer Game",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "sync": { 
    icon: "faRotate", 
    title: "Local-First Sync",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "rate": { 
    icon: "faGaugeHigh", 
    title: "Rate Limiter",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "database": { 
    icon: "faDatabase", 
    title: "Per-User DB",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "sqlite"
  },
  "tenant": { 
    icon: "faBuilding", 
    title: "Multi-Tenant SaaS",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
  "stream": { 
    icon: "faWaveSine", 
    title: "Stream Processing",
    filesToOpen: ['src/backend/registry.ts', 'src/frontend/App.tsx'],
    tab: "memory"
  },
};