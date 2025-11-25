// Import redesigned sections
import { RedesignedHero } from './sections/RedesignedHero';
import { StatsSection } from './sections/StatsSection';
import { ConceptSection } from './sections/ConceptSection';
import { CodeWalkthrough } from './sections/CodeWalkthrough';
import { ObservabilitySection } from './sections/ObservabilitySection';
import { FeaturesSection } from './sections/FeaturesSection';
import { SolutionsSection } from './sections/SolutionsSection';
import { HostingSection } from './sections/HostingSection';
import { IntegrationsSection } from './sections/IntegrationsSection';
import { RedesignedCTA } from './sections/RedesignedCTA';
import { ScrollObserver } from '@/components/ScrollObserver';
import { loadArticles } from '@/lib/article';

async function getLatestChangelogTitle(): Promise<string> {
  const articles = await loadArticles();
  const changelogEntries = articles.filter(article => article.category.id === 'changelog');
  const latest = changelogEntries.sort((a, b) => b.published.getTime() - a.published.getTime())[0];

  if (!latest) {
    throw new Error('no changelog entries found');
  }

  // Read the MDX file to extract the h1 title
  const fs = await import('node:fs/promises');
  const path = await import('node:path');
  const mdxPath = path.join(process.cwd(), 'src/posts', latest.slug, 'page.mdx');
  const content = await fs.readFile(mdxPath, 'utf-8');

  // Extract the first h1 (markdown heading)
  const h1Match = content.match(/^#\s+(.+)$/m);
  if (!h1Match) {
    throw new Error(`no h1 title found in changelog ${latest.slug}`);
  }

  return h1Match[1];
}

export default async function IndexPage() {
  const latestChangelogTitle = await getLatestChangelogTitle();

  return (
    <ScrollObserver>
      <div className='min-h-screen bg-black font-sans text-zinc-300 selection:bg-[#FF4500]/30 selection:text-orange-200'>
        <main>
          <RedesignedHero latestChangelogTitle={latestChangelogTitle} />
          <StatsSection />
          <ConceptSection />
          <CodeWalkthrough />
          <FeaturesSection />
          <IntegrationsSection />
          <ObservabilitySection />
          <SolutionsSection />
          <HostingSection />
          <RedesignedCTA />
        </main>
      </div>
    </ScrollObserver>
  );
}
