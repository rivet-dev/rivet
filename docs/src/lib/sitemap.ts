import { IconProp } from "@fortawesome/fontawesome-svg-core";
import { Route } from "next";

type Href = string | Route | URL;
type Page = { title?: string; href: Href; icon?: IconProp };
type PageWithTitle = { title: string; href: Href; icon?: IconProp };
type PageWithPages = {
  title: string;
  pages: Page[];
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
  sidebar?: SidebarItem[];
};

export type Sitemap = SiteTab[];

/** Recursively check if a tab contains a given href. */
export function findActiveTab(href: string, sitemap: Sitemap): SiteTab {
  return sitemap.find(({ sidebar }) => pagesContainsHref(href, sidebar));
}

export function pagesContainsHref(href: string, pages: AnyPage): boolean {
	  for (const page of pages) {
		  if (page.href === href) {
			  return true;
		  } else if (page.pages != null && pagesContainsHref(href, page.pages)) {
			  return true;
		  }
	  }

	  return false;
}
