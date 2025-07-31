import { CodeBlock } from "../CodeBlock";
import { Code, CodeGroup } from "../v2/Code";

interface InstallPackageProps {
    name: string;
}

export function InstallPackage({ name }: InstallPackageProps) {
    return <CodeGroup>
        <Code title="npm" language="bash">
            <CodeBlock lang="bash" code={`npm install ${name}`} />
        </Code>
        <Code title="pnpm" language="bash">
            <CodeBlock lang="bash" code={`pnpm add ${name}`} />
        </Code>
        <Code title="yarn" language="bash">
            <CodeBlock lang="bash" code={`yarn add ${name}`} />
        </Code>
        <Code title="bun" language="bash">
            <CodeBlock lang="bash" code={`bun add ${name}`} />
        </Code>
    </CodeGroup>
}