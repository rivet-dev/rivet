import { usePathname } from 'next/navigation';

import routes from '@/generated/routes.json';

export const useNavigation = () => {
  let pathname = usePathname();
  let page = routes.pages[pathname];
  let tableOfContents = page?.headings ?? null;
  return {
    navigation: {},
    page,
    tableOfContents
  };
};
