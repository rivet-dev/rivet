"use client";

import { cn } from "@/lib/utils";
import { CodeBlock, CodeBlockCode } from "./ui/code-block";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ToolInvocationType = any;

export function ToolMessage({
  toolInvocation,
}: {
  toolInvocation: ToolInvocationType;
  className?: string;
}) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const input = (toolInvocation as any).input as Record<string, any> | undefined;

  if (toolInvocation.type === "tool-list_directory") {
    return (
      <ToolBlock
        name="listing directory"
        argsText={input?.path?.split("/").slice(2).join("/")}
        toolInvocation={toolInvocation as ToolInvocationType}
      />
    );
  }

  if (toolInvocation.type === "tool-read_file") {
    return (
      <ToolBlock
        name="read file"
        argsText={input?.path?.split("/").slice(2).join("/")}
        toolInvocation={toolInvocation as ToolInvocationType}
      />
    );
  }

  if (toolInvocation.type === "tool-edit_file") {
    return <EditFileTool toolInvocation={toolInvocation} />;
  }

  if (toolInvocation.type === "tool-write_file") {
    return <WriteFileTool toolInvocation={toolInvocation} />;
  }

  if (toolInvocation.type === "tool-exec") {
    return (
      <ToolBlock
        name="exec"
        toolInvocation={toolInvocation}
        argsText={toolInvocation.input?.command}
      />
    );
  }

  if (toolInvocation.type === "tool-create_directory") {
    return (
      <ToolBlock
        name="create directory"
        toolInvocation={toolInvocation}
        argsText={toolInvocation.input?.path?.split("/").slice(2).join("/")}
      />
    );
  }

  if (toolInvocation.type === "tool-update_todo_list") {
    return (
      <ToolBlock name="update todo list" toolInvocation={toolInvocation}>
        <div className="grid gap-2">
          {toolInvocation.input?.items?.map?.(
            (
              item: { description: string; completed: boolean },
              index: number
            ) => (
              <div key={index} className="flex items-center gap-3 px-4 py-1">
                {/* Minimal sleek checkbox */}
                <div className="relative flex-shrink-0 pointer-events-none">
                  <div
                    className={cn(
                      "w-4 h-4 rounded border transition-all duration-200",
                      item.completed
                        ? "bg-black border-black"
                        : "border-gray-300 hover:border-gray-400"
                    )}
                  >
                    {item.completed && (
                      <svg
                        className="w-3 h-3 text-white absolute top-0.5 left-0.5"
                        fill="currentColor"
                        viewBox="0 0 20 20"
                      >
                        <path
                          fillRule="evenodd"
                          d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                          clipRule="evenodd"
                        />
                      </svg>
                    )}
                  </div>
                </div>
                <span
                  className={cn(
                    "flex-1",
                    item.completed && "line-through text-gray-500"
                  )}
                >
                  {item.description}
                </span>
              </div>
            )
          )}
        </div>
      </ToolBlock>
    );
  }

  // Fallback for other tools
  return (
    <ToolBlock
      toolInvocation={toolInvocation}
      name={toolInvocation.type.replaceAll("_", " ").replace("tool-", "")}
    >
      {/* <div className="text-sm text-gray-500">
        {JSON.stringify(toolInvocation.args, null, 2)}
      </div>
      {toolInvocation.state === "result" && (
        <div>
          {toolInvocation.result.content?.map((content, index) => (
            <DefaultContentRenderer content={content} key={index} />
          ))}
        </div>
      )} */}
    </ToolBlock>
  );
}

function DefaultContentRenderer(props: {
  content: {
    type: "text";
    text: string;
  };
}) {
  if (props.content?.type === "text") {
    return <div className="text-sm text-gray-500">{props.content?.text}</div>;
  }

  return (
    <div className="text-sm text-gray-500">{JSON.stringify(props.content)}</div>
  );
}

function EditFileTool({
  toolInvocation,
}: {
  toolInvocation: ToolInvocationType;
}) {
  return (
    <ToolBlock
      name="edit file"
      argsText={toolInvocation.input?.path?.split("/").slice(2).join("/")}
      toolInvocation={toolInvocation}
    >
      <div className="grid gap-2">
        {toolInvocation.input?.edits?.map?.(
          (edit: { newText: string; oldText: string }, index: number) =>
            (edit.oldText || edit.newText) && (
              <CodeBlock key={index} className="overflow-scroll py-2">
                <CodeBlockCode
                  code={edit.oldText?.split("\n").slice(0, 5).join("\n")}
                  language={"tsx"}
                  className="col-start-1 col-end-1 row-start-1 row-end-1 overflow-visible [&>pre]:py-0! [&_code]:bg-red-200! bg-red-200"
                />
                {edit.oldText?.split("\n").length > 5 && (
                  <div className="text-red-700 px-4 text-xs font-mono">
                    +{edit.oldText?.split("\n").length - 5} more
                  </div>
                )}
                <CodeBlockCode
                  code={edit.newText
                    ?.trimEnd()
                    ?.split("\n")
                    .slice(0, 5)
                    .join("\n")}
                  language={"tsx"}
                  className="col-start-1 col-end-1 row-start-1 row-end-1 overflow-visible [&>pre]:py-0! [&_code]:bg-green-200! bg-green-200"
                />
                {edit.newText?.split("\n").length > 5 && (
                  <div className="text-green-700 px-4 text-xs font-mono">
                    +{edit.newText?.split("\n").length - 5} more
                  </div>
                )}
              </CodeBlock>
            )
        )}
      </div>
    </ToolBlock>
  );
}

// function StreamLines({ text }: { text: string }) {
//   const [lines, setLines] = useState<string[]>(text.split("\n"));

//   // useEffect(() => {
//   //   const newLines = text.split("\n");
//   //   setLines();
//   // }, [text]);

//   return (
//     <div className="bg-green-200 font-mono text-xs whitespace-pre-wrap pl-2">
//       {lines.join("\n")}
//     </div>
//   );
// }

function WriteFileTool({
  toolInvocation,
}: {
  toolInvocation: ToolInvocationType;
}) {
  return (
    <ToolBlock
      name="write file"
      argsText={toolInvocation.input?.path?.split("/").slice(2).join("/")}
      toolInvocation={toolInvocation}
    >
      {toolInvocation.input?.content && (
        <CodeBlock className="overflow-scroll sticky bottom-0">
          <CodeBlockCode
            code={
              toolInvocation.input?.content
                ?.split("\n")
                .slice(0, 5)
                .join("\n") ?? ""
            }
            language={"tsx"}
            className="col-start-1 col-end-1 row-start-1 row-end-1 overflow-visible [&_code]:bg-green-200! bg-green-200"
          />
          {toolInvocation.input?.content?.split("\n").length > 5 && (
            <div className="text-green-700 px-4 text-xs pb-2 font-mono">
              +{toolInvocation.input?.content?.split("\n").length - 5} more
            </div>
          )}
        </CodeBlock>
      )}
    </ToolBlock>
  );
}

function ToolBlock(props: {
  toolInvocation?: ToolInvocationType;
  name: string;
  argsText?: string;
  children?: React.ReactNode;
}) {
  return (
    <div>
      <div className="flex py-1">
        <div
          className="flex items-center gap-2"
          // className={cn(
          //   "text-sm  px-2 mt-2 py-1 rounded max-h-24 overflow-scroll max-w-sm transition-colors duration-500",
          //   props.toolInvocation?.state !== "result"
          //     ? "border border-gray-800 animate-pulse bg-gray-800 text-white"
          //     : "border border-neutral-500 text-neutral-500 bg-transparent"
          // )}
        >
          <div className="grid translate-y-[1px]">
            {props.toolInvocation?.state !== "output-available" && (
              <div
                className={cn(
                  "border border-black w-2 h-2 rounded-full inline-block col-start-1 col-end-1 row-start-1 row-end-1",

                  "bg-black animate-ping"
                )}
              ></div>
            )}
            <div
              className={cn(
                "border w-2 h-2 rounded-full inline-block col-start-1 col-end-1 row-start-1 row-end-1",
                props.toolInvocation?.state === "output-available" &&
                  props.toolInvocation.result?.isError
                  ? "bg-red-500 border-red-500"
                  : props.toolInvocation?.state === "output-available"
                    ? "border-gray-400 bg-gray-400"
                    : "border-black bg-black"
              )}
            ></div>
          </div>
          <span className="font-medium">{props.name}</span>
          <span>{props.argsText}</span>
        </div>
      </div>
      {(props.children && <div className="mb-2">{props.children}</div>) ||
        (props.toolInvocation?.state === "output-available" &&
          props.toolInvocation.output?.isError &&
          props.toolInvocation.output?.content?.map(
            (content: { type: "text"; text: string }, i: number) => (
              <CodeBlock key={i} className="overflow-scroll py-2">
                <CodeBlockCode
                  className="[&>pre]:py-0! py-2 text-red-500"
                  code={content.text}
                  language="text"
                />
              </CodeBlock>
            )
          ))}
    </div>
  );
}
