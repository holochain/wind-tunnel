import { DurableObject } from "cloudflare:workers";

interface Env {
    RUN_STORE: DurableObjectNamespace<RunStore>;

    // Use `wrangler secret put SECRET_KEY` to set this in production.
    SECRET_KEY: string;
}

interface RunRecord {
    value: unknown;
    createdAt: number;
}

/** Creates a Response with Content-Type set to `application/json`. */
function createJSONResponse(body: BodyInit, status?: number): Response {
    return new Response(body, {
        headers: { "Content-Type": "application/json" },
        status: status,
    });
}

/**
 * Durable Object that stores a single JSON blob per run ID.
 * Handles /set (POST) and /get (GET) internally via `stub.fetch` calls from
 * the worker's `handlePost` and `handleGet` functions which are not exposed.
 */
export class RunStore extends DurableObject<Env> {
    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);
        const method = request.method;

        if (method === "POST" && url.pathname === "/set") {
            const { value } = await request.json<{ value: unknown }>();
            const createdAt = Date.now();
            await this.ctx.storage.put("data", { value, createdAt });
            return createJSONResponse(JSON.stringify({ success: true }));
        }

        if (method === "GET" && url.pathname === "/get") {
            const record = await this.ctx.storage.get<RunRecord>("data");
            if (!record) {
                return createJSONResponse(JSON.stringify({ error: "Not found" }), 404);
            }
            const twelveHours = 12 * 60 * 60 * 1000;
            if (Date.now() - record.createdAt > twelveHours) {
                await this.ctx.storage.delete("data");
                return createJSONResponse(JSON.stringify({ error: "Expired" }), 404);
            }
            return createJSONResponse(JSON.stringify({ value: record.value }));
        }

        return new Response("Not found", { status: 404 });
    }
}

/**
 * Handles POST requests to store a JSON blob for a given run ID.
 * Requires a valid `SECRET_KEY` in the request body to prevent unauthorised writes.
 */
async function handlePost(request: Request, env: Env): Promise<Response> {
    try {
        const { run_id, value, secret } = await request.json<{ run_id: string; value: unknown; secret: string }>();
        if (run_id == null || value == null || secret == null) {
            return createJSONResponse(JSON.stringify({ error: "Missing required fields" }), 400);
        }
        if (secret !== env.SECRET_KEY) {
            return createJSONResponse(JSON.stringify({ error: "Unauthorized" }), 403);
        }
        const id = env.RUN_STORE.idFromName(run_id);
        const stub = env.RUN_STORE.get(id);
        const resp = await stub.fetch("https://internal/set", {
            method: "POST",
            body: JSON.stringify({ value }),
        });
        if (!resp.ok) {
            return resp;
        }
        return createJSONResponse(JSON.stringify({ success: true }), 200);
    } catch (err) {
        if (err instanceof SyntaxError) {
            return createJSONResponse(JSON.stringify({ error: "Invalid JSON" }), 400);
        }
        const message = err instanceof Error ? err.message : "Unknown error";
        return createJSONResponse(JSON.stringify({ error: message }), 500);
    }
}

/**
 * Handles GET requests to retrieve the stored JSON blob for a given run ID.
 * No authentication is required — any caller with the run ID can read the value.
 * Values expire after 12 hours and are deleted on first access after expiry.
 */
async function handleGet(url: URL, env: Env): Promise<Response> {
    const run_id = url.searchParams.get("run_id");
    if (!run_id) {
        return createJSONResponse(JSON.stringify({ error: "Missing run_id" }), 400);
    }
    const id = env.RUN_STORE.idFromName(run_id);
    const stub = env.RUN_STORE.get(id);
    const resp = await stub.fetch("https://internal/get");
    const data = await resp.json<{ error?: string; value?: unknown }>();
    if (data.error) {
        return createJSONResponse(JSON.stringify(data), 404);
    }
    return createJSONResponse(JSON.stringify(data), 200);
}

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);
        const method = request.method;
        if (method === "POST") {
            return await handlePost(request, env);
        } else if (method === "GET") {
            return await handleGet(url, env);
        } else {
            return new Response("Method Not Allowed", { status: 405 });
        }
    },
} satisfies ExportedHandler<Env>;
