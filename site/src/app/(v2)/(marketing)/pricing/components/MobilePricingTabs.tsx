'use client'

import { Fragment } from 'react'
import { CheckIcon, MinusIcon } from '@heroicons/react/16/solid'
import { Tab } from '@headlessui/react'

const tiers = [
  {
    name: 'Starter',
    href: '#',
  },
  {
    name: 'Growth',
    href: '#',
  },
  {
    name: 'Scale',
    href: '#',
  },
]

const sections = [
  {
    name: 'Features',
    features: [
      { name: 'Edge content delivery', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Custom domains', tiers: { Starter: '1', Growth: '3', Scale: 'Unlimited' } },
      { name: 'Team members', tiers: { Starter: '3', Growth: '20', Scale: 'Unlimited' } },
      { name: 'Single sign-on (SSO)', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
  {
    name: 'Reporting',
    features: [
      { name: 'Advanced analytics', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Basic reports', tiers: { Starter: false, Growth: true, Scale: true } },
      { name: 'Professional reports', tiers: { Starter: false, Growth: false, Scale: true } },
      { name: 'Custom report builder', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
  {
    name: 'Support',
    features: [
      { name: '24/7 online support', tiers: { Starter: true, Growth: true, Scale: true } },
      { name: 'Quarterly workshops', tiers: { Starter: false, Growth: true, Scale: true } },
      { name: 'Priority phone support', tiers: { Starter: false, Growth: false, Scale: true } },
      { name: '1:1 onboarding tour', tiers: { Starter: false, Growth: false, Scale: true } },
    ],
  },
]

export function MobilePricingTabs() {
  return (
    <div className="sm:hidden">
      <Tab.Group>
        <Tab.List className="flex">
          {tiers.map((tier) => (
            <Tab
              key={tier.name}
              className="w-1/3 border-b border-gray-800 py-4 text-base/8 font-medium text-indigo-400 data-[selected]:border-indigo-400 [&:not([data-focus])]:focus:outline-none"
            >
              {tier.name}
            </Tab>
          ))}
        </Tab.List>
        <Tab.Panels>
          {tiers.map((tier) => (
            <Tab.Panel key={tier.name}>
              <a
                href={tier.href}
                className="mt-8 block rounded-md bg-gray-900 px-3.5 py-2.5 text-center text-sm font-semibold text-white shadow-sm ring-1 ring-inset ring-white/10 hover:bg-gray-800"
              >
                Get started
              </a>
              {sections.map((section) => (
                <Fragment key={section.name}>
                  <div className="-mx-6 mt-10 rounded-lg bg-gray-900 px-6 py-3 text-sm/6 font-semibold text-white group-first-of-type:mt-5">
                    {section.name}
                  </div>
                  <dl>
                    {section.features.map((feature) => (
                      <div
                        key={feature.name}
                        className="grid grid-cols-2 border-b border-gray-800 py-4 last:border-none"
                      >
                        <dt className="text-sm/6 font-normal text-gray-300">{feature.name}</dt>
                        <dd className="text-center">
                          {typeof feature.tiers[tier.name] === 'string' ? (
                            <span className="text-sm/6 text-white">{feature.tiers[tier.name]}</span>
                          ) : (
                            <>
                              {feature.tiers[tier.name] === true ? (
                                <CheckIcon aria-hidden="true" className="inline-block size-4 fill-green-400" />
                              ) : (
                                <MinusIcon aria-hidden="true" className="inline-block size-4 fill-gray-600" />
                              )}

                              <span className="sr-only">{feature.tiers[tier.name] === true ? 'Yes' : 'No'}</span>
                            </>
                          )}
                        </dd>
                      </div>
                    ))}
                  </dl>
                </Fragment>
              ))}
            </Tab.Panel>
          ))}
        </Tab.Panels>
      </Tab.Group>
    </div>
  )
}