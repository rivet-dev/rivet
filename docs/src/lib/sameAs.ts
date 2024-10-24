export const CORE_DIRECTORIES = ['general', 'modules'];

export const ENGINES = ['godot', 'unity', 'unreal', 'html5', 'custom'];

export const ENGINE_LABEL_MAP = {
  godot: 'Godot',
  unity: 'Unity',
  unreal: 'Unreal',
  html5: 'HTML5',
  custom: 'Custom',
  default: 'General'
};

export function getAliasedSlug(slug: string[]) {
  if (
    (ENGINES.includes(slug[0]) && CORE_DIRECTORIES.includes(slug[1])) ||
    (CORE_DIRECTORIES.includes(slug[0]) && CORE_DIRECTORIES.includes(slug[1]))
  ) {
    slug = slug.slice(1);
  }
  return slug;
}

export function getAliasedHref(href: string) {
  const [_, __, ...slug] = href.split('/');
  const aliasedSlug = getAliasedSlug(slug);
  return '/docs/' + aliasedSlug.join('/');
}
