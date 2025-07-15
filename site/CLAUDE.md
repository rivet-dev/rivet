## Next.js Best Practices

- Avoid use of "use client" for any components with text content. If you need to have client-side functionality, add a useEffect or move the client-side code in to a separate client-only component.