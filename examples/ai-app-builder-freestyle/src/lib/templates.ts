export const templates: Record<
  string,
  { name: string; repo: string; logo: string }
> = {
  nextjs: {
    name: "Next.js",
    repo: "https://github.com/freestyle-sh/freestyle-base-nextjs-shadcn",
    logo: "/logos/next.svg",
  },
  vite: {
    name: "React Vite",
    repo: "https://github.com/freestyle-sh/freestyle-base-vite-react-typescript-swc",
    logo: "/logos/vite.svg",
  },
  expo: {
    name: "Expo",
    repo: "https://github.com/freestyle-sh/freestyle-expo",
    logo: "/logos/expo.svg",
  },
};
