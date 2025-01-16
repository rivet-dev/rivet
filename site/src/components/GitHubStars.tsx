import { useState, useEffect } from 'react';
import { Icon, faGithub } from '@rivet-gg/icons';
import { cn } from '@rivet-gg/components';

interface GitHubStarsProps extends React.AnchorHTMLAttributes<HTMLAnchorElement> {
  repo?: string;
}

function formatNumber(num: number): string {
  if (num >= 1000) {
    return `${(num / 1000).toFixed(1)}k`;
  }
  return num.toString();
}

export async function GitHubStars({ repo = 'rivet-gg/rivet', className, ...props }: GitHubStarsProps) {
  try {
    const response = await fetch(`https://api.github.com/repos/${repo}`);
    const data = await response.json();
    const { stargazers_count: stars } = data;
    
    return (
      <a
        href={`https://github.com/${repo}`}
        target='_blank'
        rel='noreferrer'
        className={cn(
          "md:bg-white/10 rounded-md px-4 h-10 flex items-center gap-2 md:hover:bg-white/20 transition-colors",
          className
        )}
        {...props}>
        <Icon icon={faGithub} /> <span className="hidden md:inline">{stars ? `${formatNumber(stars)} Stars` : 'GitHub'}</span>
      </a>
    );
  } catch (err) {
    console.error('Failed to fetch stars', err);
    return null;
  }
} 