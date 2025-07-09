import Link from "next/link";
import { Icon, faCode, faArrowRight, faRobot, faCodeMerge, faWaveSine, faGamepad, faFilePen } from "@rivet-gg/icons";

// TODO: update to this
//
// - Deploy stateful AI agent with Vercel AI
// - Local-first sync OR CRDT with yjs
// - Run untrusted user code on demand
// - Collaborative document
// - Stateful stream processing
// - Multiplayer game servers

// Tutorials section
export const TutorialsSection = () => {
  const tutorials = [
    {
      title: "Orchestrate stateful agent with AI SDK",
      description: "Build intelligent agents that maintain conversation context & tool calls",
      icons: [faRobot],
      href: "/docs/tutorials/stateful-ai-agent",
      useCases: ["AI assistants", "Chat applications", "Customer support"]
    },
    {
      title: "Local-first sync with yjs",
      description: "Create collaborative apps with real-time synchronization",
      icons: [faCodeMerge],
      href: "/docs/tutorials/local-first-sync",
      useCases: ["Collaborative editing", "Offline-first apps", "Real-time sync"]
    },
    {
      title: "Run code in a secure sandbox",
      description: "Execute user or LLM-generated code safely with isolation",
      icons: [faCode],
      href: "/docs/tutorials/sandbox-code-execution",
      useCases: ["LLM code execution", "User code execution", "Code playgrounds"]
    },
    {
      title: "Collaborative document",
      description: "Build multi-user document editing capabilities",
      icons: [faFilePen],
      href: "/docs/tutorials/collaborative-document",
      useCases: ["Team editing", "Shared workspaces", "Real-time collaboration"]
    },
    {
      title: "Stateful stream processing",
      description: "Process and transform data streams with persistent state",
      icons: [faWaveSine],
      href: "/docs/tutorials/stateful-stream",
      useCases: ["Event processing", "Data pipelines", "Real-time analytics"]
    },
    {
      title: "Multiplayer game servers",
      description: "Deploy game servers that scale with your player base",
      icons: [faGamepad],
      href: "/docs/tutorials/multiplayer-game",
      useCases: ["Real-time games", "Match hosting", "Game infrastructure"]
    }
  ];

  return (
    <div className="mx-auto max-w-7xl px-6 py-28 lg:py-44 lg:px-8 mt-16">
      <div className="text-center mb-12">
        <h2 className="text-3xl font-bold tracking-tight text-white">Start building in seconds</h2>
        <p className="mt-4 text-lg text-white/70">Follow our step-by-step tutorials to deploy your first project quickly</p>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
        {tutorials.map((tutorial, index) => (
          <Link key={index} href={tutorial.href} className="group">
            <div className="rounded-xl h-[280px] bg-[#121212] group-hover:bg-zinc-800/90 border border-white/5 group-hover:border-white/20 shadow-sm transition-all duration-200 flex flex-col overflow-hidden">
              {/* Top section with icons */}
              <div className="bg-black/40 p-4 flex items-center justify-center h-[120px] w-full">
                <div className="flex space-x-4">
                  {tutorial.icons.map((icon, iconIndex) => (
                    <div key={iconIndex} className="flex items-center justify-center w-14 h-14">
                      <Icon 
                        icon={icon} 
                        className="text-4xl text-white/50 group-hover:text-white transition-colors duration-200" 
                      />
                    </div>
                  ))}
                </div>
              </div>
              
              {/* Bottom section with content */}
              <div className="p-5 flex flex-col flex-1">
                <h3 className="text-xl font-semibold text-white mb-2">{tutorial.title}</h3>
                <p className="text-white/60 text-sm">{tutorial.description}</p>
                
                <div className="flex items-center mt-auto pt-3 text-white opacity-0 group-hover:opacity-100 transition-opacity">
                  <span className="text-sm font-medium">View tutorial</span>
                  <Icon icon={faArrowRight} className="ml-2 text-xs group-hover:translate-x-0.5 transition-all" />
                </div>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
};
