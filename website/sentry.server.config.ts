import * as Sentry from "@sentry/astro";

Sentry.init({
    dsn: "https://f65fbba4e667acd89187b9806737ac32@o4504307129188352.ingest.us.sentry.io/4510985550823424",
    sendDefaultPii: true,
});