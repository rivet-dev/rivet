import * as shiki from 'shiki';

const LANGS: shiki.BundledLanguage[] = [
  'bash',
  'batch',
  'cpp',
  'csharp',
  'docker',
  'gdscript',
  'html',
  'ini',
  'js',
  'json',
  'json',
  'powershell',
  'ts',
  'typescript',
  'yaml',
  'http',
  'prisma'
];

const theme = shiki.createCssVariablesTheme({
  name: 'css-variables',
  variablePrefix: '--shiki-',
  variableDefaults: {},
  fontStyle: true
});

let highlighter: shiki.Highlighter;

export async function CodeBlock({ lang, code }: { lang: shiki.BundledLanguage; code: string }) {
  highlighter ??= await shiki.getSingletonHighlighter({
    langs: LANGS,
    themes: [theme]
  });

  const out = highlighter.codeToHtml(code, {
    lang,
    theme
  });

  return <div className='code' dangerouslySetInnerHTML={{ __html: out }} />;
}
