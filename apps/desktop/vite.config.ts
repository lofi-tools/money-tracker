import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
export default defineConfig(({ mode }) => ({
  plugins: [
    solid(),
    ...(mode === "review"
      ? [
          {
            name: "local-portfolio-review",
            configureServer(server: import("vite").ViteDevServer) {
              server.middlewares.use(
                "/__portfolio_review",
                async (req, res) => {
                  // Same-origin loopback dev preview only; no write endpoints or file paths from clients.
                  if (
                    req.method !== "GET" ||
                    !["127.0.0.1:1420", "localhost:1420"].includes(
                      req.headers.host ?? "",
                    ) ||
                    (req.headers.origin &&
                      ![
                        "http://127.0.0.1:1420",
                        "http://localhost:1420",
                      ].includes(req.headers.origin))
                  ) {
                    res.statusCode = 403;
                    res.end();
                    return;
                  }
                  try {
                    const file =
                      process.env.PORTFOLIO_REVIEW_FILE ??
                      resolve("../../.cache/portfolio-review.json");
                    res.setHeader("Content-Type", "application/json");
                    res.setHeader("Cache-Control", "no-store");
                    res.end(await readFile(file));
                  } catch {
                    res.statusCode = 503;
                    res.end("Review snapshot unavailable");
                  }
                },
              );
            },
          },
        ]
      : []),
  ],
  resolve: { dedupe: ["solid-js"] },
  clearScreen: false,
  server: { host: "127.0.0.1", port: 1420, strictPort: true, cors: false },
}));
