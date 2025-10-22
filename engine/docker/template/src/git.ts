import { TemplateContext } from "./context";

export function generateGitAttributes(context: TemplateContext) {
	const gitAttributesContent = `. linguist-generated=true
`;

	context.writeFile(".gitattributes", gitAttributesContent);
}

