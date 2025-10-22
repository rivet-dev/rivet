import { Info } from '@rivet-gg/components/mdx';
import Link from 'next/link';

export function EnterpriseCloud() {
	return (
		<Info>
			<p>Rivet Cloud is currently intended for enterprise clients that need a scalable, high-performance way to deploy Rivet Functions, Actors, and Containers.</p>

			<p>Please <Link href="/sales">reach out for access</Link> for access to Rivet Cloud. To deploy RivetKit without Rivet Cloud, see our <Link href="/docs/general/self-hosting">self-hosting documentation</Link>.</p>
		</Info>
	)
}

