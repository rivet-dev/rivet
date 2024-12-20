import { IconProp } from "@fortawesome/fontawesome-svg-core";
import { Route } from "next";

type Href = string | Route | URL;
type Page = { title?: string; href: Href; icon?: IconProp };
type PageWithTitle = { title: string; href: Href; icon?: IconProp };
type PageWithPages = {
  title: string;
  pages: AnyPage[];
  collapsible?: true;
  initiallyOpen?: boolean;
  icon?: IconProp;
};
export type AnyPage = Page | PageWithTitle | PageWithPages;

type SidebarTopLevelPage = Page;
export type SidebarSection = {
  title: string;
  collapsible?: true;
  pages: AnyPage[];
};

export type SidebarItem = SidebarTopLevelPage | SidebarSection;

type SiteTab = {
  title: string;
  href: Href;
  sidebar: SidebarItem[];
};

export type Sitemap = SiteTab[];

interface FoundTab {
  tab: SiteTab;
  page: FoundPage;
}

/** Recursively check if a tab contains a given href. */
export function findActiveTab(
  href: string,
  sitemap: Sitemap,
): FoundTab | undefined {
  for (const tab of sitemap) {
    const page = findPageForHref(href, tab);
    if (page) return { tab, page };
  }
}

interface FoundPage {
  page: AnyPage | SiteTab | SidebarItem;
  parent?: AnyPage | SiteTab | SidebarItem;
}

export function findPageForHref(
  href: string,
  page: AnyPage | SiteTab | SidebarItem,
  parent?: AnyPage | SiteTab | SidebarItem,
): FoundPage | undefined {
  // Remove trailing slash for consistency
  href = normalizeHref(href);

  // Check if page matches
  if ("href" in page && typeof page.href == "string") {
    // Remove trailing slash for consistency
    const pageHref = normalizeHref(page.href);
    if (pageHref === href) {
      return { page, parent };
    }
  }

  // Check child pages
  if ("pages" in page) {
    for (const childPage of page.pages as any) {
      const found = findPageForHref(href, childPage, page);
      if (found) return found;
    }
  } else if ("sidebar" in page) {
    for (const childPage of page.sidebar) {
      const found = findPageForHref(href, childPage, page);
      if (found) return found;
    }
  }

  return undefined;
}

function normalizeHref(href: string): string {
  return href.replace(/\/$/, "");
}
