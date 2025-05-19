"use client";

import Link from "next/link";
import { Icon, faCode, faLayerGroup, faTerminal, faDesktop, faListCheck, faArrowsToCircle, faArrowRight, faDatabase } from "@rivet-gg/icons";

// Tutorials section
export const TutorialsSection = () => {
  const tutorials = [
    {
      title: "Deploy your first function",
      description: "Get started with serverless functions in minutes",
      icons: [faCode],
      href: "/docs/tutorials/first-function",
      useCases: ["APIs", "Webhooks", "Edge computing"]
    },
    {
      title: "Create a stateful actor",
      description: "Build services that maintain state between requests",
      icons: [faLayerGroup],
      href: "/docs/tutorials/stateful-actor",
      useCases: ["AI agents", "Stateful workers", "Long-running processes"]
    },
    {
      title: "Build a workflow",
      description: "Orchestrate complex multi-step processes",
      icons: [faArrowsToCircle],
      href: "/docs/tutorials/workflows",
      useCases: ["Multi-agent systems", "Business logic", "ETL pipelines"]
    },
    {
      title: "Run AI generated code in a sandbox",
      description: "Execute untrusted code safely with isolation",
      icons: [faTerminal],
      href: "/docs/tutorials/ai-sandbox",
      useCases: ["LLM code execution", "User code execution", "AI agents"]
    },
    {
      title: "Access desktop sandbox",
      description: "Run GUI applications in an isolated environment",
      icons: [faDesktop],
      href: "/docs/tutorials/desktop-sandbox",
      useCases: ["Remote desktops", "Browser automation", "Visual apps"]
    },
    {
      title: "Store agent memory",
      description: "Persist and retrieve AI agent context and knowledge",
      icons: [faDatabase],
      href: "/docs/tutorials/agent-memory",
      useCases: ["RAG", "Vector embeddings", "AI agent state"]
    }
  ];

  return (
    <div className="mx-auto max-w-7xl px-6 py-28 lg:py-44 lg:px-8 mt-16">
      <div className="text-center mb-12">
        <h2 className="text-4xl font-medium tracking-tight text-white">Start building in seconds</h2>
        <p className="mt-4 text-lg text-white/70">Follow our step-by-step tutorials to deploy your first project quickly</p>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
        {tutorials.map((tutorial, index) => (
          <Link key={index} href={tutorial.href} className="group">
            <div className="rounded-xl h-[280px] bg-[#121212] border border-white/5 group-hover:border-white/20 shadow-sm transition-all duration-200 flex flex-col overflow-hidden relative">
              {/* Icon section */}
              <div className="flex-1 flex items-center justify-center">
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
              
              {/* Content section */}
              <div className="p-5">
                <h3 className="text-xl font-normal text-white mb-2">{tutorial.title}</h3>
                <p className="text-white/60 text-sm">{tutorial.description}</p>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
};
