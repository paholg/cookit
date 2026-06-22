export const BASE_DOMAIN = process.env.BASE_DOMAIN!;

// The book's host (no port). Requests reach the app through Caddy (see
// `playwright.config.ts`), which sets `X-Forwarded-Host` from this host so the
// server resolves the book from the subdomain — standing in for the prod proxy.
export function bookHost(slug: string): string {
  return `${slug}.${BASE_DOMAIN}`;
}

export function bookBaseURL(slug: string): string {
  const port = process.env.E2E_PORT;
  return `http://${bookHost(slug)}:${port}`;
}
