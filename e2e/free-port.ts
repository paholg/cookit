import { execFileSync } from "node:child_process";

// Ask the OS for an unused TCP port instead of hardcoding one.
//
// Playwright reads the port out of the config synchronously (it starts the web
// server before any async setup runs) and doesn't await an async default
// export, so we can't use Node's async `net` API directly here. Instead we shell
// out to a throwaway node process that binds to port 0 — letting the kernel pick
// a free port — reads the assigned port back, and prints it.
export function freePort(): number {
  const probe =
    'const s = require("net").createServer();' +
    's.listen(0, "127.0.0.1", () => {' +
    "  const { port } = s.address();" +
    "  s.close(() => process.stdout.write(String(port)));" +
    "});";

  const out = execFileSync(process.execPath, ["-e", probe], {
    encoding: "utf8",
  });

  const port = Number.parseInt(out.trim(), 10);

  if (!Number.isInteger(port) || port <= 0) {
    throw new Error(`could not determine a free port (got: ${JSON.stringify(out)})`);
  }

  return port;
}
