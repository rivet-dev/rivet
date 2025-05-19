"use client";

import Link from "next/link";
import { Button } from "@rivet-gg/components";
import { Icon, faArrowRight } from "@rivet-gg/icons";

// CTA section
export const CtaSection = () => {
  return (
    <div className="mx-auto max-w-7xl px-6 py-36 lg:py-52 lg:px-8 border-t border-white/5 mt-24">
      <div className="text-center">
        <h2 className="text-4xl font-bold tracking-tight text-white">Get building today</h2>
        <p className="mt-6 text-xl text-white/70 max-w-2xl mx-auto">
          Start for free, no credit card required. Deploy your first serverless project in minutes.
        </p>
        
        <div className="mt-12 flex items-center justify-center gap-x-6">
          <Button 
            size="lg" 
            asChild 
            className="px-4 pr-6 py-3 text-base bg-gradient-to-b from-[#FF5C00] to-[#FF5C00]/90 border border-[#FF5C00]/30 hover:border-[#FF5C00]/60 transition-all duration-200 group"
          >
            <Link href="#deploy" className="flex items-center justify-center relative">
              <span>Deploy in 1 Minute</span>
              <Icon icon={faArrowRight} className="absolute right-2 text-sm opacity-0 group-hover:opacity-100 group-hover:translate-x-1 transition-all duration-200" />
            </Link>
          </Button>
          <Button 
            variant="outline" 
            size="lg" 
            asChild 
            className="px-4 pr-6 py-3 text-base border-white/10 hover:border-white/30 transition-all duration-200 group"
          >
            <Link href="#demo" className="flex items-center justify-center relative">
              <span>Book Demo</span>
              <Icon icon={faArrowRight} className="absolute right-2 text-sm opacity-0 group-hover:opacity-100 group-hover:translate-x-1 transition-all duration-200" />
            </Link>
          </Button>
        </div>
      </div>
    </div>
  );
};