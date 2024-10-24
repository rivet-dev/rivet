import { Sitemap, SiteTab } from '@/lib/sitemap';
// import { advanced, common, developingModules, platforms, usingModules } from '@/sitemap/common';

// Goals:
// - Siebar links should advertise the product, collapse any advanced pages away
// - The sidebar should be 1 screen height when collapsed

export const sitemap = [
	{
		title: 'Documentation',
		href: '/docs',
		sidebar: [
			{ title: 'Overview', href: '/docs/introduction', icon: 'square-info' },
			{
				title: 'Getting Started',
				pages: [
					{ title: 'Initial Setup', href: '/docs', icon: 'play' },
					{ title: 'Intro to Rivet', href: '/docs', icon: 'book' },
				]
			},
			{
				title: 'Learn About Rivet',
				pages: [
					{ title: 'What Are Actors?', href: '/docs', icon: 'user-group' },
					{ title: 'Containers vs Isolates', href: '/docs', icon: 'box' },
					{
						title: 'Use Cases',
						collapsible: true,
						pages: [
							{ title: 'Gaming', href: '/docs', icon: 'gamepad' },
							{ title: 'AI', href: '/docs', icon: 'robot' },
						]
					}
				]
			},
			{
				title: 'Build with Rivet',
				pages: [
					{ title: 'Creating an Actor', href: '/docs', icon: 'plus' },
					{ title: 'Actor Messaging', href: '/docs', icon: 'envelope' },
					{ title: 'Actor Networking', href: '/docs', icon: 'network-wired' },
					{ title: 'Authentication', href: '/docs', icon: 'lock' },
				]
			},
			{
				title: 'Resources',
				pages: [
					{ title: 'Self-Hosting', href: '/docs', icon: 'server' },
					{ title: 'Billing', href: '/docs', icon: 'credit-card' },
					{ title: 'DDoS Mitigation', href: '/docs', icon: 'shield' },
					{ title: 'Security', href: '/docs', icon: 'lock-open' },
					{
						title: 'Comparison',
						collapsible: true,
						pages: [
							{ title: 'Kubernetes', href: '/docs', icon: 'ship' },
							{ title: 'Cloudflare', href: '/docs', icon: 'cloud' },
							{ title: 'Socket.io', href: '/docs', icon: 'plug' },
							{ title: 'Redis', href: '/docs', icon: 'database' },
						]
					}
				]
			},
		]
	},
	{
		title: 'Guides',
		href: '/docs/examples',
		sidebar: [
		]
	},
	{
		title: 'Actors SDK',
		href: '/docs/examples',
		sidebar: [
		]
	},
	{
		title: 'Platform API',
		href: '/docs/examples',
		sidebar: [
		]
	},
	//  {
	//    title: 'Godot',
	//    href: '/docs/godot',
	//    sidebar: [
	//      { href: '/docs/godot', icon: 'square-info' },
	//      {
	//        title: 'Multiplayer',
	//        pages: [
	//          { href: '/docs/godot/tutorials/quickstart', icon: 'rocket' },
	//          {
	//            title: 'Concepts',
	//            collapsible: true,
	//            pages: [{ href: '/docs/godot/concepts/resources' }]
	//          }
	//        ]
	//      },
	//      // ...common('/docs/godot')
	//    ]
	//  },
	//  {
	//    title: 'Unity',
	//    href: '/docs/unity',
	//    sidebar: [
	//      { href: '/docs/unity', icon: 'square-info' },
	//      {
	//        title: 'Multiplayer',
	//        pages: [{ href: '/docs/unity/tutorials/quickstart-fishnet', icon: 'rocket' }]
	//      },
	//      // ...common('/docs/unity')
	//    ]
	//  },
	//  {
	//    title: 'Unreal',
	//    href: '/docs/unreal',
	//    sidebar: [
	//      { href: '/docs/unreal', icon: 'square-info' },
	//      {
	//        title: 'Multiplayer',
	//        pages: [
	//          { href: '/docs/unreal/tutorials/quickstart', icon: 'rocket' },
	//          {
	//            title: 'Concepts',
	//            collapsible: true,
	//            pages: [
	//              // { href: '/docs/unreal/concepts/run-methods' },
	//              { href: '/docs/unreal/concepts/resources' },
	//              { href: '/docs/unreal/concepts/build-engine-from-source' },
	//              { href: '/docs/unreal/concepts/useful-commands' }
	//            ]
	//          },
	//          {
	//            title: 'Troubleshooting',
	//            collapsible: true,
	//            pages: [
	//              { href: '/docs/unreal/troubleshooting/chmod-error' },
	//              { href: '/docs/unreal/troubleshooting/empty-level' },
	//              { href: '/docs/unreal/troubleshooting/port-7777-already-taken' },
	//              { href: '/docs/unreal/troubleshooting/standalone-wrong-map' }
	//            ]
	//          }
	//        ]
	//      },
	//      // ...common('/docs/unreal')
	//    ]
	//  },
	//  {
	//    title: 'HTML5',
	//    href: '/docs/html5',
	//    sidebar: [
	//      { href: '/docs/html5', icon: 'square-info' },
	//      {
	//        title: 'Multiplayer',
	//        pages: [{ href: '/docs/html5/tutorials/quickstart', icon: 'rocket' }]
	//      },
	//      // ...common('/docs/html5')
	//    ]
	//  },
	//  {
	//    title: 'Custom',
	//    href: '/docs/custom',
	//    sidebar: [{ href: '/docs/custom', icon: 'square-info' }, 
	// 	// ...common('/docs/custom')
	// ]
	//  },
	//  {
	//    title: 'Core',
	//    href: '/docs/general',
	//    sidebar: [{ href: '/docs/general', icon: 'square-info' }, 
	// 	// ...common('/docs/general')
	// ]
	//  },
	//  {
	//    title: 'Rivet Modules',
	//    href: '/docs/modules',
	//    sidebar: [
	//      // ...usingModules('/docs').pages,
	//      // developingModules('/docs'),
	//      // platforms('/docs/modules'),
	//      // advanced('/docs/modules')
	//    ]
	//  }
] satisfies Sitemap;
