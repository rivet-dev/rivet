import routes from '@/generated/routes.json';
import { SidebarItem } from '@/lib/sitemap';
import { getAliasedHref } from '@/lib/sameAs';
import { Icon, iconPack } from '@rivet-gg/icons';
import { PropsWithChildren, ReactNode } from 'react';
import { cn } from '@rivet-gg/components';
import { CollapsibleSidebarItem } from '@/components/CollapsibleSidebarItem';
import { ActiveLink } from '@/components/ActiveLink';
import { library } from '@fortawesome/fontawesome-svg-core';

library.add(iconPack);

interface TreeItemProps {
  item: SidebarItem;
}

function TreeItem({ item }: TreeItemProps) {
  if ('collapsible' in item && 'title' in item && 'pages' in item && item.collapsible) {
    return (
      <CollapsibleSidebarItem item={item}>
        <Tree pages={item.pages} />
      </CollapsibleSidebarItem>
    );
  }

  if ('title' in item && 'pages' in item) {
    return (
      <div>
        <p className='mt-2 px-2 py-1 text-sm font-semibold'>
          {item.icon ? <Icon icon={item.icon} className='mr-2 size-3.5' /> : null}
          <span className='truncate'> {item.title}</span>
        </p>
        <Tree pages={item.pages} />
      </div>
    );
  }

  return (
    <NavLink href={item.href}>
      {item.icon ? <Icon icon={item.icon} className='mr-2 size-3.5' /> : null}
      <span className='truncate'>{item.title ?? routes.pages[getAliasedHref(item.href)]?.title}</span>
    </NavLink>
  );
}

interface TreeProps {
  pages: SidebarItem[];
  className?: string;
}

export function Tree({ pages, className }: TreeProps) {
  return (
    <ul role='list' className={cn(className)}>
      {pages.map((item, index) => (
        <li key={index} className='relative'>
          <TreeItem item={item} />
        </li>
      ))}
    </ul>
  );
}

export function NavLink({
  href,
  children,
  className
}: PropsWithChildren<{ href: string; children: ReactNode; className?: string }>) {
  return (
    <ActiveLink
      strict
      href={href}
      className={cn(
        'text-muted-foreground aria-current-page:text-foreground group flex w-full items-center rounded-md border border-transparent px-2 py-1 text-sm hover:underline',
        className
      )}>
      {children}
    </ActiveLink>
  );
}

export function DocsNavigation({ sidebar }: { sidebar: SidebarItem[] }) {
  return (
    <div className='top-header sticky pr-4 text-white md:max-h-content md:overflow-y-auto md:pb-4 md:pt-8'>
      <Tree pages={sidebar} />
    </div>
  );
}
