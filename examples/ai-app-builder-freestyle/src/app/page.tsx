"use client";

import { useRouter } from "next/navigation";
import { PromptInput, PromptInputActions } from "@/components/ui/prompt-input";
import { FrameworkSelector } from "@/components/framework-selector";
import Image from "next/image";
import LogoSvg from "@/logo.svg";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { ExampleButton } from "@/components/ExampleButton";
import { UserApps } from "@/components/user-apps";
import { PromptInputTextareaWithTypingAnimation } from "@/components/prompt-input";

export default function Home() {
  const [prompt, setPrompt] = useState("");
  const [framework, setFramework] = useState("nextjs");
  const [isLoading, setIsLoading] = useState(false);
  const router = useRouter();

  const handleSubmit = async () => {
    setIsLoading(true);

    router.push(
      `/app/new?message=${encodeURIComponent(prompt)}&template=${framework}`
    );
  };

  return (
    <main className="min-h-screen p-4 relative">
      <div className="flex w-full justify-between items-center">
	  {/*<h1 className="text-lg font-bold flex-1 sm:w-80">
          <a href="https://www.freestyle.sh">freestyle.sh</a>
        </h1>
        <Image
          className="dark:invert mx-2"
          src={LogoSvg}
          alt="Adorable Logo"
          width={36}
          height={36}
        />*/}
        <div className="flex items-center gap-2 flex-1 sm:w-80 justify-end">
          {/* User actions could go here */}
        </div>
      </div>

        <div>
          <div className="w-full max-w-lg px-4 sm:px-0 mx-auto flex flex-col items-center mt-16 sm:mt-24 md:mt-32 col-start-1 col-end-1 row-start-1 row-end-1 z-10">
            <p className="text-neutral-600 text-center mb-6 text-3xl sm:text-4xl md:text-5xl font-bold">
              Rivet AI App Builder
            </p>

            <div className="w-full relative my-5">
              <div className="relative w-full max-w-full overflow-hidden">
                <div className="w-full bg-accent rounded-md relative z-10 border transition-colors">
                  <PromptInput
                    leftSlot={
                      <FrameworkSelector
                        value={framework}
                        onChange={setFramework}
                      />
                    }
                    isLoading={isLoading}
                    value={prompt}
                    onValueChange={setPrompt}
                    onSubmit={handleSubmit}
                    className="relative z-10 border-none bg-transparent shadow-none focus-within:border-gray-400 focus-within:ring-1 focus-within:ring-gray-200 transition-all duration-200 ease-in-out "
                  >
                    <PromptInputTextareaWithTypingAnimation />
                    <PromptInputActions>
                      <Button
                        variant={"ghost"}
                        size="sm"
                        onClick={handleSubmit}
                        disabled={isLoading || !prompt.trim()}
                        className="h-7 text-xs"
                      >
                        <span className="hidden sm:inline">
                          Start Creating ⏎
                        </span>
                        <span className="sm:hidden">Create ⏎</span>
                      </Button>
                    </PromptInputActions>
                  </PromptInput>
                </div>
              </div>
            </div>
            <Examples setPrompt={setPrompt} />
            <div className="mt-8 mb-16">
              <a
                href="https://freestyle.sh"
                className="border rounded-md px-4 py-2 mt-4 text-sm font-semibold transition-colors duration-200 ease-in-out cursor-pointer w-full max-w-72 text-center block"
              >
                <span className="block font-bold">
                  Works with <span className="underline">freestyle.sh</span>
                </span>
                <span className="text-xs">
                  JavaScript infrastructure for AI.
                </span>
              </a>
            </div>
          </div>
        </div>
      <div className="border-t py-8 mx-0 sm:-mx-4">
        <UserApps />
      </div>
    </main>
  );
}

function Examples({ setPrompt }: { setPrompt: (text: string) => void }) {
  return (
    <div className="mt-2">
      <div className="flex flex-wrap justify-center gap-2 px-2">
        <ExampleButton
          text="Dog Food Marketplace"
          promptText="Build a dog food marketplace where users can browse and purchase premium dog food."
          onClick={(text) => {
            console.log("Example clicked:", text);
            setPrompt(text);
          }}
        />
        <ExampleButton
          text="Personal Website"
          promptText="Create a personal website with portfolio, blog, and contact sections."
          onClick={(text) => {
            console.log("Example clicked:", text);
            setPrompt(text);
          }}
        />
        <ExampleButton
          text="Burrito B2B SaaS"
          promptText="Build a B2B SaaS for burrito shops to manage inventory, orders, and delivery logistics."
          onClick={(text) => {
            console.log("Example clicked:", text);
            setPrompt(text);
          }}
        />
      </div>
    </div>
  );
}
