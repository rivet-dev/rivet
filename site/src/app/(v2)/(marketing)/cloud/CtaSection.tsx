"use client";

import { CtaButtons } from "./CtaButtons";

// CTA section
export const CtaSection = () => {
  return (
    <div className="mx-auto max-w-7xl px-6 py-36 lg:py-52 lg:px-8 border-t border-white/5 mt-24">
      <div className="text-center">
        <h2 className="text-4xl font-medium tracking-tight text-white">Get building today</h2>
        <p className="mt-6 text-xl text-white/70 max-w-lg mx-auto">
          Start for free, no credit card required. Deploy your first serverless project in minutes.
        </p>
        
        <div className="mt-12 flex items-center justify-center gap-x-6">
          <CtaButtons />
        </div>
      </div>
    </div>
  );
};
