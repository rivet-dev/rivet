/**
 * Echo HTTP Server with Deno
 * 
 * This server executes user-provided code in a subprocess and returns the output
 * as a response to HTTP requests.
 */

console.log(Deno.env.toObject());

// Check for required USER_CODE_FILE_NAME environment variable
const userCodeFileName = Deno.env.get("USER_CODE_FILE_NAME");
if (!userCodeFileName) {
  console.error("ERROR: USER_CODE_FILE_NAME environment variable is required");
  Deno.exit(1);
}

console.log(`User code file set to: fixtures/${userCodeFileName}`);

/**
 * Load and run the user code from the specified file path
 * 
 * @returns Object containing success status and output text
 */
async function loadAndRunUserCode(): Promise<{success: boolean, output: string}> {
  try {
    // Construct the path to the user code file
    const filePath = `fixtures/${userCodeFileName}`;
    
    // Run user code as a subprocess with a clean environment
    const command = new Deno.Command(Deno.execPath(), {
      args: ["run", filePath],
      stdout: "piped",
      stderr: "piped",
      clearEnv: true, // Run with a clean environment (no parent env vars)
    });
    
    const { code, stdout, stderr } = await command.output();
    
    if (code !== 0) {
      const errorMessage = new TextDecoder().decode(stderr);
      console.error(`Error executing user code: ${errorMessage}`);
      return {
        success: false,
        output: `Error executing user code: ${errorMessage}`
      };
    }
    
    const output = new TextDecoder().decode(stdout);
    console.log(`User code output: ${output}`);
    return {
      success: true,
      output
    };
  } catch (error) {
    console.error(`Exception: ${error.message}`);
    return {
      success: false,
      output: `Server error: ${error.message}`
    };
  }
}

/**
 * HTTP request handler
 * 
 * @param req - The incoming HTTP request
 * @returns HTTP response with the output from user code execution
 */
async function handler(req: Request): Promise<Response> {
  console.log("req");

  const result = await loadAndRunUserCode();
  
  if (!result.success) {
    return new Response(result.output, {
      status: 500,
      headers: { "Content-Type": "text/plain" },
    });
  }
  
  return new Response(result.output, {
    status: 200,
    headers: { "Content-Type": "text/plain" },
  });
}

const port = parseInt(Deno.env.get("PORT_HTTP") || "8000");
console.log(`Server starting on port ${port}`);

Deno.serve({
  handler,
  port,
  hostname: "0.0.0.0",
});
