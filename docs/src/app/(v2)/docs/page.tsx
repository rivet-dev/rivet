import { redirect } from 'next/navigation';

export const metadata = {
	title: 'Redirecting...',
};

export default async function DocsPage() {
	redirect('/docs/introduction');
}
