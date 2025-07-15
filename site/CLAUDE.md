## Next.js Best Practices

- Avoid use of "use client" for any components with text content. If you need to have client-side functionality, add a useEffect or move the client-side code in to a separate client-only component.
- Do not run `next dev` yourself. If you need a specific change to be tested, tell me to manually check it for you.