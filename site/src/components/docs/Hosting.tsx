import { CardGroup, Card } from '@/components/Card'

export function Hosting() {
	return (
		<>
			<p>By default, Rivet stores actor state on the local file system. To scale Rivet in production, follow a guide to deploy to a hosting provider or integrate a driver:</p>

			<p><b>Hosting Providers</b></p>
			<CardGroup>
				<Card title="Railway" href="/docs/hosting-providers/railway">
					Deploy Rivet applications with Railway's platform-as-a-service
				</Card>
				<Card title="Cloudflare Workers" href="/docs/hosting-providers/cloudflare-workers">
					Run Rivet actors on Cloudflare's edge computing platform
				</Card>
				<Card title="Rivet Cloud (Enterprise)" href="/docs/hosting-providers/rivet-cloud">
					Managed Rivet hosting with enterprise features and support
				</Card>
			</CardGroup>

			<p><b>Drivers</b></p>
			<CardGroup>
				<Card title="Redis" href="/docs/drivers/redis">
					High-performance in-memory data store for production workloads
				</Card>
				<Card title="File System" href="/docs/drivers/file-system">
					Simple file-based storage for development and small deployments
				</Card>
				<Card title="Memory" href="/docs/drivers/memory">
					In-memory storage for testing and ephemeral use cases
				</Card>
				<Card title="Build Your Own" href="/docs/drivers/build-your-own">
					Create custom storage drivers for your specific requirements
				</Card>
			</CardGroup>
		</>
	)
}

