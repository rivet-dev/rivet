import type { Handler } from "@netlify/functions";
import { registry } from "../src/actors.ts";

export const handler: Handler = async (event, context) => {
  const { httpMethod, path, queryStringParameters, headers, body } = event;
  
  // Convert Netlify event to standard Request
  const url = `https://${headers.host}${path}${
    queryStringParameters 
      ? '?' + new URLSearchParams(queryStringParameters).toString() 
      : ''
  }`;
  
  const request = new Request(url, {
    method: httpMethod,
    headers: headers as HeadersInit,
    body: body ? body : undefined,
  });

  const response = await registry.handler(request);
  
  // Convert Response to Netlify format
  const responseHeaders: Record<string, string> = {};
  response.headers.forEach((value, key) => {
    responseHeaders[key] = value;
  });

  return {
    statusCode: response.status,
    headers: responseHeaders,
    body: await response.text(),
  };
};