import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
	serverExternalPackages: [
		'@earendil-works/pi-ai',
		'@flue/runtime',
		'@mongodb-js/zstd',
		'@rivet-dev/flue',
		'@rivetkit/next-js',
		'@rivetkit/rivetkit-napi',
		'node-liblzma',
		'rivetkit',
		'rivetkit/client',
		'rivetkit/db',
	],
	webpack(config, { isServer }) {
		if (isServer) {
			config.externals.push({
				'@rivetkit/rivetkit-napi': 'module @rivetkit/rivetkit-napi',
				rivetkit: 'module rivetkit',
				'rivetkit/client': 'module rivetkit/client',
				'rivetkit/db': 'module rivetkit/db',
			});
		}
		return config;
	},
};

export default nextConfig;
