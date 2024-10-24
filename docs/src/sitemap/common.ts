import { SidebarSection, Sitemap } from '@/lib/sitemap';

import apiPages from '@/generated/apiPages.json' assert { type: 'json' };

export const common = (prefix: string = '/docs'): SidebarSection[] => [
  usingModules(prefix),
  developingModules(prefix),
  platforms(prefix.replace('/general', '')),
  advanced(prefix.replace('/general', ''))
];

export const usingModules = (prefix: string = '/docs'): SidebarSection => ({
  title: 'Rivet Modules',
  pages: [
    { href: `${prefix}/modules`, icon: 'puzzle' },
    { href: `${prefix}/modules/categories/matchmaking`, icon: 'chess' },
    // { href: `${prefix}/modules/categories/parties`, icon: 'party-horn' },
    { href: `${prefix}/modules/categories/authentication`, icon: 'key' },
    { href: `${prefix}/modules/categories/social`, icon: 'share-nodes' },
    // { href: `${prefix}/modules/categories/competitive`, icon: 'ranking-star' },
    // { href: `${prefix}/modules/categories/economy`, icon: 'coin-front' },
    { href: `${prefix}/modules/categories/storage`, icon: 'floppy-disk' },
    {
      title: 'Advanced',
      collapsible: true,
      pages: [
        { href: `${prefix}/modules/project-config`, icon: 'square-sliders' },
        { href: `${prefix}/modules/cli`, icon: 'terminal' },
        { href: `${prefix}/modules/sdk`, icon: 'code' },
        { href: `${prefix}/modules/registries`, icon: 'share-nodes' },
        { href: `${prefix}/modules/multiple-games`, icon: 'object-intersect' },
        { href: `${prefix}/modules/environment-variables`, icon: 'leaf' }
      ]
    }
  ]
});

export const developingModules = (prefix: string = '/docs'): SidebarSection => ({
  title: 'Developing Rivet Modules',
  pages: [
    {
      href: `${prefix}/modules/build`,
      icon: 'screwdriver-wrench'
    },
    { href: `${prefix}/modules/build/scripts`, icon: 'file-code' },
    { href: `${prefix}/modules/build/database`, icon: 'database' },
    { href: `${prefix}/modules/build/actors`, icon: 'bolt' },
    // TODO:
    // {
    //   href: `${prefix}/modules/build/user-config`,
    //   icon: 'paint-roller'
    // },
    { href: `${prefix}/modules/build/utility-modules`, icon: 'toolbox' },
    {
      title: 'Advanced',
      collapsible: true,
      pages: [
        { href: `${prefix}/modules/build/conventions` },
        { href: `${prefix}/modules/build/module-config` },
        { href: `${prefix}/modules/build/publish` },
        { href: `${prefix}/modules/build/errors` },
        { href: `${prefix}/modules/build/ide` },
        { href: `${prefix}/modules/build/public` },
        { href: `${prefix}/modules/build/logging` }
      ]
    }
  ]
});

export const platforms = (prefix: string = '/docs'): SidebarSection => ({
  title: 'Platforms',
  pages: [{ href: `${prefix}/general/platforms/discord`, icon: 'discord' }]
});

export const advanced = (prefix: string = '/docs'): SidebarSection => ({
  title: 'Advanced',
  pages: [
    {
      title: 'Concepts',
      collapsible: true,
      pages: [{ href: `${prefix}/general/concepts/authoritative-vs-p2p` }]
    },
    {
      title: 'Reference',
      collapsible: true,
      pages: [{ href: `${prefix}/general/errors` }]
    },
    {
      title: 'Dynamic Servers',
      collapsible: true,
      pages: [
        { href: `${prefix}/general/dynamic-servers`, title: 'Overview' },
        {
          title: 'Concepts',
          collapsible: true,
          pages: [
            // { href: `${prefix}/general/dynamic-servers/architecture` },
            // { href: `${prefix}/general/dynamic-servers/billing` },
            { href: `${prefix}/general/dynamic-servers/crash-reporting` },
            // { href: `${prefix}/general/dynamic-servers/ddos` },
            // { href: `${prefix}/general/dynamic-servers/debugging-lobbies` },
            { href: `${prefix}/general/dynamic-servers/docker-root-user` },
            { href: `${prefix}/general/dynamic-servers/game-guard` },
            { href: `${prefix}/general/dynamic-servers/graceful-exit` },
            { href: `${prefix}/general/dynamic-servers/host-bridge-networking` },
            // { href: `${prefix}/general/dynamic-servers/instant-deploys` },
            // { href: `${prefix}/general/dynamic-servers/lifecycle` },
            // { href: `${prefix}/general/dynamic-servers/logging-metrics` },
            { href: `${prefix}/general/dynamic-servers/logging` },
            // { href: `${prefix}/general/dynamic-servers/managing-lobbies` },
            { href: `${prefix}/general/dynamic-servers/monitoring` },
            // { href: `${prefix}/general/dynamic-servers/one-lobby-one-container` },
            { href: `${prefix}/general/dynamic-servers/ports` }
            // { href: `${prefix}/general/dynamic-servers/resource-limits` },
            // { href: `${prefix}/general/dynamic-servers/ssl` }
          ]
        },
        {
          title: 'Reference',
          collapsible: true,
          pages: [
            { href: `${prefix}/general/dynamic-servers/protocols` },
            { href: `${prefix}/general/dynamic-servers/available-regions` },
            { href: `${prefix}/general/dynamic-servers/available-tiers` }
          ]
        },
        {
          title: 'API',
          collapsible: true,
          pages: apiPages['dynamic-servers'].pages.map(({ href }) => ({
            href: href.replace('/docs', prefix)
          }))
        }
      ]
    },
    {
      title: 'Cloud',
      collapsible: true,
      pages: [
        {
          title: 'API',
          collapsible: true,
          pages: apiPages.cloud.pages.map(({ href }) => ({
            href: href.replace('/docs', prefix)
          }))
        }
      ]
    }
  ]
});
