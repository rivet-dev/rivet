"use client";

import React, { useEffect, useState } from "react";
import Chat from "./chat";
import { TopBar } from "./topbar";
import { MessageCircle, Monitor } from "lucide-react";
import WebView from "./webview";
import { UIMessage } from "ai";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const queryClient = new QueryClient();

export default function AppWrapper({
  appName,
  repo,
  initialMessages,
  appId,
  repoId,
  baseId,
  domain,
  running,
  codeServerUrl,
  consoleUrl,
}: {
  appName: string;
  repo: string;
  appId: string;
  respond?: boolean;
  initialMessages: UIMessage[];
  repoId: string;
  baseId: string;
  codeServerUrl: string;
  consoleUrl: string;
  domain?: string;
  running: boolean;
}) {
  const [mobileActiveTab, setMobileActiveTab] = useState<"chat" | "preview">(
    "chat"
  );
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768); // md breakpoint
    };

    checkMobile();
    window.addEventListener("resize", checkMobile);
    return () => window.removeEventListener("resize", checkMobile);
  }, []);

  useEffect(() => {
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "auto"; // or 'visible'
    };
  }, []);

  return (
    <div className="h-screen flex flex-col" style={{ height: "100dvh" }}>
      {/* Desktop and Mobile container */}
      <div className="flex-1 overflow-hidden flex flex-col md:grid md:grid-cols-[1fr_2fr]">
        {/* Chat component - positioned for both mobile and desktop */}
        <div
          className={
            isMobile
              ? `absolute inset-0 z-10 flex flex-col transition-transform duration-200 ${
                  mobileActiveTab === "chat"
                    ? "translate-x-0"
                    : "-translate-x-full"
                }`
              : "h-full overflow-hidden flex flex-col"
          }
          style={
            isMobile
              ? {
                  top: "env(safe-area-inset-top)",
                  bottom: "calc(60px + env(safe-area-inset-bottom))",
                }
              : undefined
          }
        >
          <QueryClientProvider client={queryClient}>
            <Chat
              topBar={
                <TopBar
                  appName={appName}
                  repoId={repoId}
                  consoleUrl={consoleUrl}
                  codeServerUrl={codeServerUrl}
                />
              }
              appId={appId}
              initialMessages={initialMessages}
              key={appId}
              running={running}
            />
          </QueryClientProvider>
        </div>

        {/* Preview component - positioned for both mobile and desktop */}
        <div
          className={
            isMobile
              ? `absolute inset-0 z-10 transition-transform duration-200 ${
                  mobileActiveTab === "preview"
                    ? "translate-x-0"
                    : "translate-x-full"
                }`
              : "overflow-auto"
          }
          style={
            isMobile
              ? {
                  top: "env(safe-area-inset-top)",
                  bottom: "calc(60px + env(safe-area-inset-bottom))",
                }
              : undefined
          }
        >
          <div className="h-full overflow-hidden relative">
            <WebView
              repo={repo}
              baseId={baseId}
              appId={appId}
              domain={domain}
            />
          </div>
        </div>
      </div>

      {/* Mobile tab navigation */}
      {isMobile && (
        <div
          className="fixed bottom-0 left-0 right-0 flex border-t bg-background/95 backdrop-blur-sm pb-safe"
          style={{ paddingBottom: "env(safe-area-inset-bottom)" }}
        >
          <button
            onClick={() => setMobileActiveTab("chat")}
            className={`flex-1 flex flex-col items-center justify-center py-2 px-1 transition-colors ${
              mobileActiveTab === "chat"
                ? "text-primary"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <MessageCircle
              className={`h-6 w-6 mb-1 ${
                mobileActiveTab === "chat" ? "fill-current" : ""
              }`}
            />
            <span className="text-xs font-medium">Chat</span>
          </button>
          <button
            onClick={() => setMobileActiveTab("preview")}
            className={`flex-1 flex flex-col items-center justify-center py-2 px-1 transition-colors ${
              mobileActiveTab === "preview"
                ? "text-primary"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Monitor
              className={`h-6 w-6 mb-1 ${
                mobileActiveTab === "preview" ? "fill-current" : ""
              }`}
            />
            <span className="text-xs font-medium">Preview</span>
          </button>
        </div>
      )}
    </div>
  );
}
