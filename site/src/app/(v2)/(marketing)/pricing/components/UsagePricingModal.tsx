"use client";
import React from "react";

interface UsagePricingModalProps {
  open: boolean;
  onClose: () => void;
}

export default function UsagePricingModal({ open, onClose }: UsagePricingModalProps) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
      <div className="relative w-full max-w-3xl mx-4 bg-[#181818] rounded-2xl shadow-lg text-white p-8 overflow-y-auto max-h-[90vh]">
        <button
          onClick={onClose}
          className="absolute top-4 right-4 text-white/60 hover:text-white text-2xl font-bold focus:outline-none"
          aria-label="Close"
        >
          ×
        </button>
        <h2 className="text-2xl font-semibold mb-2">Rivet Cloud Usage Pricing</h2>
        <p className="text-white/70 mb-6">Choose your cloud provider.</p>
        <div className="mb-6">
          <label className="text-sm text-white/60 mr-2">Cloud Provider</label>
          <select className="bg-[#222] text-white rounded px-3 py-1 border border-white/10 focus:outline-none">
            <option>Linode</option>
            <option>AWS</option>
          </select>
        </div>
        <div className="overflow-x-auto mb-8">
          <table className="w-full text-left border-separate border-spacing-y-2">
            <thead>
              <tr className="text-white/80 text-sm">
                <th className="font-normal pb-2">Region</th>
                <th className="font-normal pb-2">Compute<br /><span className="text-xs font-normal">$ (per vCPU · hr)</span></th>
                <th className="font-normal pb-2">Storage<br /><span className="text-xs font-normal">$ (per GiB · hr)</span></th>
              </tr>
            </thead>
            <tbody className="text-white/90 text-sm">
              <tr><td>N. Virginia (us-east-1)</td><td>$0.113</td><td>$0.00103</td></tr>
              <tr><td>Oregon (us-west-2)</td><td>$0.113</td><td>$0.00103</td></tr>
              <tr><td>Mumbai (ap-south-1)</td><td>$0.113</td><td>$0.00114</td></tr>
              <tr><td>Singapore (ap-southeast-1)</td><td>$0.119</td><td>$0.00123</td></tr>
              <tr><td>Frankfurt (eu-central-1)</td><td>$0.117</td><td>$0.00114</td></tr>
              <tr><td>Ireland (eu-west-1)</td><td>$0.114</td><td>$0.00114</td></tr>
            </tbody>
          </table>
        </div>
        <div className="grid md:grid-cols-3 gap-4 mb-8">
          <div className="bg-[#222] rounded-lg p-4">
            <h3 className="font-medium mb-1 text-white">Stateless Functions</h3>
            <p className="text-white/70 text-sm">Deploy serverless functions that scale automatically</p>
          </div>
          <div className="bg-[#222] rounded-lg p-4">
            <h3 className="font-medium mb-1 text-white">Stateful Actors</h3>
            <p className="text-white/70 text-sm">Long running tasks with state persistence, hibernation, and realtime</p>
          </div>
          <div className="bg-[#222] rounded-lg p-4">
            <h3 className="font-medium mb-1 text-white">Sandboxed Containers</h3>
            <p className="text-white/70 text-sm">Run CPU- & memory-intensive workloads in secure containers with blazing fast coldstarts</p>
          </div>
        </div>
        <div className="mb-8">
          <h3 className="font-medium mb-2 text-white">Storage pricing</h3>
          <table className="w-full text-left border-separate border-spacing-y-2 mb-2">
            <thead>
              <tr className="text-white/80 text-xs">
                <th className="font-normal pb-1">&nbsp;</th>
                <th className="font-normal pb-1">Community</th>
                <th className="font-normal pb-1">Pro</th>
                <th className="font-normal pb-1">Team</th>
                <th className="font-normal pb-1">Enterprise</th>
              </tr>
            </thead>
            <tbody className="text-white/90 text-sm">
              <tr>
                <th className="font-normal text-white/70">Storage Reads</th>
                <td>1M Max</td>
                <td>1M + $0.20/million</td>
                <td>1M + $0.20/million</td>
                <td>Custom</td>
              </tr>
              <tr>
                <th className="font-normal text-white/70">Storage Writes</th>
                <td>1M Max</td>
                <td>1M + $1.00/million</td>
                <td>1M + $1.00/million</td>
                <td>Custom</td>
              </tr>
              <tr>
                <th className="font-normal text-white/70">Storage Stored Data</th>
                <td>1GB Max</td>
                <td>1GB + $0.20/GB month</td>
                <td>1GB + $0.20/GB month</td>
                <td>Custom</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
} 