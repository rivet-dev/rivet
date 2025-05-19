'use client'

import { useState } from 'react'
import { Switch } from '@headlessui/react'
import { MarketingButton } from '../MarketingButton'

export default function SalesPageClient() {
  const [agreed, setAgreed] = useState(false)

  return (
    <main className="min-h-screen w-full bg-black">
      <div className="relative isolate overflow-hidden pb-8 sm:pb-10 pt-40">
        <div className="mx-auto max-w-4xl px-6 lg:px-8 text-center">
          <h1 className="text-6xl font-700 text-white leading-[1.1] tracking-normal">
            Contact Sales
          </h1>
          <p className="mt-6 max-w-3xl mx-auto text-xl leading-[1.2] tracking-tight font-500 text-white/60">
            Get in touch with our sales team to discuss your enterprise needs and how Rivet can help scale your infrastructure.
          </p>
        </div>
      </div>

      <div className="mx-auto max-w-2xl px-6 lg:px-8 pt-16 sm:pt-24">
        <form action="#" method="POST" className="mx-auto mt-16 max-w-xl sm:mt-20">
          <div className="grid grid-cols-1 gap-x-8 gap-y-6 sm:grid-cols-2">
            <div>
              <label htmlFor="first-name" className="block text-sm/6 font-semibold text-white">
                First name
              </label>
              <div className="mt-2.5">
                <input
                  id="first-name"
                  name="first-name"
                  type="text"
                  autoComplete="given-name"
                  className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
                />
              </div>
            </div>
            <div>
              <label htmlFor="last-name" className="block text-sm/6 font-semibold text-white">
                Last name
              </label>
              <div className="mt-2.5">
                <input
                  id="last-name"
                  name="last-name"
                  type="text"
                  autoComplete="family-name"
                  className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
                />
              </div>
            </div>
            <div className="sm:col-span-2">
              <label htmlFor="company" className="block text-sm/6 font-semibold text-white">
                Company
              </label>
              <div className="mt-2.5">
                <input
                  id="company"
                  name="company"
                  type="text"
                  autoComplete="organization"
                  className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
                />
              </div>
            </div>
            <div className="sm:col-span-2">
              <label htmlFor="email" className="block text-sm/6 font-semibold text-white">
                Email
              </label>
              <div className="mt-2.5">
                <input
                  id="email"
                  name="email"
                  type="email"
                  autoComplete="email"
                  className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
                />
              </div>
            </div>
            <div className="sm:col-span-2">
              <label htmlFor="message" className="block text-sm/6 font-semibold text-white">
                Message
              </label>
              <div className="mt-2.5">
                <textarea
                  id="message"
                  name="message"
                  rows={4}
                  className="block w-full rounded-md bg-[#121212] px-3.5 py-2 text-base text-white outline outline-1 -outline-offset-1 outline-white/10 placeholder:text-white/40 focus:outline focus:outline-2 focus:-outline-offset-2 focus:outline-[#FF5C00]"
                  placeholder="I would like Rivet to help solve for my company..."
                />
              </div>
            </div>
          </div>
          <div className="mt-10">
            <button
              type="submit"
              className="w-full inline-flex items-center justify-center px-6 py-3 text-base font-medium rounded-xl transition-all duration-200 active:scale-[0.97] bg-[#FF5C00]/90 hover:bg-[#FF5C00] hover:brightness-110 text-white"
            >
              Let's talk
            </button>
          </div>
        </form>
      </div>
    </main>
  )
} 