import { Sitemap, SiteTab } from '@/lib/sitemap';
// import { advanced, common, developingModules, platforms, usingModules } from '@/sitemap/common';

// Goals:
// - Siebar links should advertise the product, collapse any advanced pages away
// - The sidebar should be 1 screen height when collapsed

// Profiles:
// - What does Rivet do?
//	- Does it work for my use cases -> Use Cases
//	- Curious about the technology -> Build with Rivet
// - Just want to jump in
// - People who want to run Open Source

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
					// { title: 'Local Development', href: '/docs', icon: 'computer' },
					{
						title: 'Languages & Game Engines',
						collapsible: true,
						pages: [
							{ title: 'Godot', href: '/docs' },
							{ title: 'Unity', href: '/docs' },
							{ title: 'Unreal', href: '/docs' },
							{ title: 'JavaScript & TypeScript', href: '/docs' },
						]
					},
				]
			},
			{
				title: 'Use Cases',
				pages: [
					{ title: 'Multiplayer Tools', href: '/docs', icon: 'rotate' },
					{ title: 'Dedicated Game Servers', href: '/docs', icon: 'gamepad' },
					{ title: 'AI Agents', href: '/docs', icon: 'robot' },
					{ title: 'Batch Jobs', href: '/docs', icon: 'forklift' },
					// { title: 'Live Events', href: '/docs', icon: 'calendar' },
					{ title: 'Run User Code', href: '/docs', icon: 'alien-8bit' },
					{ title: 'More', href: '/docs', icon: 'ellipsis' },
				]
			},
			// {
			// 	title: 'Learn About Rivet',
			// 	pages: [
			// 		{ title: 'What Are Actors?', href: '/docs', icon: 'user-group' },
			// 		{ title: 'Containers vs Isolates', href: '/docs', icon: 'box' },
			// 		{
			// 			title: 'Use Cases',
			// 			collapsible: true,
			// 			pages: [
			// 				{ title: 'Dedicated Game Servers', href: '/docs', icon: 'gamepad' },
			// 				{ title: 'AI Voice Agents', href: '/docs', icon: 'robot' },
			// 				{ title: 'Run User Code', href: '/docs', icon: 'robot' },
			// 			]
			// 		}
			// 	]
			// },
			{
				title: 'Build with Rivet',
				pages: [
					{ title: 'What Are Actors?', href: '/docs', icon: 'block-question' },
					// { title: 'Low-Latency Networking', href: '/docs', icon: 'network-wired' },
					{ title: 'Low-Latency Networking', href: '/docs', icon: 'network-wired' },
					{ title: 'Realtime Messaging', href: '/docs', icon: 'envelope' },
					{ title: 'Persistence', href: '/docs', icon: 'floppy-disk' },
					{ title: 'Scaling & Concurrency', href: '/docs', icon: 'maximize' },
					{ title: 'Edge Regions', href: '/docs', icon: 'globe' },
					{
						title: 'Concepts',
						collapsible: true,
						pages: [
							{ title: 'Authentication', href: '/docs', icon: 'fingerprint' },
							{ title: 'DDoS & Botting Mitigation', href: '/docs', icon: 'shield-halved' },
							{ title: 'Fault Tolerance', href: '/docs', icon: 'fingerprint' },
							{ title: 'Containers vs Isolates', href: '/docs', icon: 'box' },
						]
					},
				]
			},
			{
				title: 'Resources',
				pages: [
					{ title: 'Self-Hosting', href: '/docs', icon: 'server' },
					{ title: 'Billing', href: '/docs', icon: 'credit-card' },
					{ title: 'DDoS Mitigation', href: '/docs', icon: 'shield' },
					{ title: 'Security', href: '/docs', icon: 'lock' },
					{
						title: 'Comparison',
						collapsible: true,
						pages: [
							{ title: 'Kubernetes', href: '/docs' },
							{ title: 'Cloudflare', href: '/docs' },
							{ title: 'Socket.io', href: '/docs' },
							{ title: 'Redis', href: '/docs' },
						]
					}
				]
			},
		]
	},
	{
		title: 'Examples',
		href: '/docs/examples',
		sidebar: [
			// TODO: Gropu by type in sidebar
		]
	},
	// {
	// 	title: 'Integrations',
	// 	href: '/docs/examples',
	// 	sidebar: [
	// 		// TODO: Gropu by type in sidebar
	// 	]
	// },
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
