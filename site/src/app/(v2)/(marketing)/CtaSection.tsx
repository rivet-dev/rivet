"use client";

import Link from "next/link";
import { Button } from "@rivet-gg/components";

// CTA section
export const CtaSection = () => {
  return (
    <div className="mx-auto max-w-7xl px-6 py-36 lg:py-52 lg:px-8 border-t border-white/5 mt-24">
      <div className="text-center">
        <h2 className="text-4xl font-medium tracking-tight text-white">Get building today</h2>
        <p className="mt-6 text-xl text-white/70 max-w-2xl mx-auto">
          Start for free, no credit card required. Deploy your first serverless project in minutes.
        </p>
        
        <div className="mt-12 flex items-center justify-center gap-x-6">
          <Button 
            size="lg" 
            asChild 
            className="px-4 py-3 text-base bg-gradient-to-b from-[#FF5C00] to-[#FF5C00]/90 border border-[#FF5C00]/30 hover:border-[#FF5C00]/60 hover:from-[#E65400] hover:to-[#E65400]/90 transition-all duration-200"
          >
            <Link href="#deploy">
              <span>Deploy in 1 Minute</span>
            </Link>
          </Button>
          <Button 
            variant="outline" 
            size="lg" 
            asChild 
            className="px-4 py-3 text-base border-white/10 hover:border-white/30 transition-all duration-200"
          >
            <Link href="#demo">
              <span>Book Demo</span>
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
};