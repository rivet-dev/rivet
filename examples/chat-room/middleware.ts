// Middleware for Vercel to serve index.html as a SPA
export default function middleware(request: Request) {
    const path = new URL(request.url).pathname;
    if (path.startsWith("/api") || path.startsWith("/assets")) return;
    return new Response(null, {
        headers: { "x-middleware-rewrite": new URL("/index.html", request.url).toString() },
    });
}
