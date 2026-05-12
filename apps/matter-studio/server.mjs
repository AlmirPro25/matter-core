import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { existsSync, readFileSync } from "node:fs";
import { extname, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const appDir = resolve(fileURLToPath(new URL(".", import.meta.url)));
const repoRoot = resolve(appDir, "..", "..");
const publicDir = join(appDir, "public");

loadEnv(join(appDir, ".env"));
loadEnv(join(repoRoot, ".env"));

const port = Number(process.env.MATTER_STUDIO_PORT || 4177);

const mimeTypes = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url || "/", `http://${req.headers.host}`);

    if (req.method === "GET" && url.pathname === "/api/status") {
      return sendJson(res, {
        ok: true,
        providers: {
          openai: Boolean(process.env.OPENAI_API_KEY),
          gemini: Boolean(process.env.GEMINI_API_KEY),
        },
        models: {
          openai: process.env.OPENAI_MODEL || "gpt-4o-mini",
          gemini: process.env.GEMINI_MODEL || "gemini-1.5-flash",
        },
      });
    }

    if (req.method === "POST" && url.pathname === "/api/chat") {
      const body = await readJson(req);
      const result = await chat(body);
      return sendJson(res, result);
    }

    if (req.method === "POST" && url.pathname === "/api/matter") {
      const body = await readJson(req);
      const result = await runMatter(body);
      return sendJson(res, result);
    }

    if (req.method === "GET" && url.pathname === "/api/examples") {
      return sendJson(res, {
        ok: true,
        examples: [
          await readRepoFile("examples/first_run.matter"),
          await readRepoFile("examples/language_tour.matter"),
          await readRepoFile("examples/reflexive_self.matter"),
        ],
      });
    }

    return serveStatic(req, res, url.pathname);
  } catch (error) {
    return sendJson(res, { ok: false, error: String(error?.message || error) }, 500);
  }
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Matter Studio running at http://127.0.0.1:${port}`);
});

function loadEnv(path) {
  if (!existsSync(path)) return;
  const source = requireSafeRead(path);
  for (const line of source.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq < 0) continue;
    const key = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    if (key && process.env[key] === undefined) process.env[key] = value;
  }
}

function requireSafeRead(path) {
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

async function serveStatic(req, res, pathname) {
  const safePath = pathname === "/" ? "/index.html" : pathname;
  const filePath = resolve(publicDir, "." + decodeURIComponent(safePath));
  if (!filePath.startsWith(publicDir)) {
    res.writeHead(403);
    return res.end("Forbidden");
  }
  try {
    const content = await readFile(filePath);
    res.writeHead(200, {
      "Content-Type": mimeTypes[extname(filePath)] || "application/octet-stream",
      "Cache-Control": "no-store",
    });
    return res.end(content);
  } catch {
    const content = await readFile(join(publicDir, "index.html"));
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    return res.end(content);
  }
}

async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString("utf8");
  return raw ? JSON.parse(raw) : {};
}

function sendJson(res, payload, status = 200) {
  res.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(payload));
}

async function chat(body) {
  const provider = body.provider || "local";
  const messages = Array.isArray(body.messages) ? body.messages : [];
  const source = normalizeSource(body.source);
  const system = matterSystemPrompt(source);

  if (provider === "openai") return chatOpenAI(system, messages);
  if (provider === "gemini") return chatGemini(system, messages);

  const last = messages[messages.length - 1]?.content || "";
  return {
    ok: true,
    provider: "local",
    message:
      "Modo local ativo. Configure OPENAI_API_KEY ou GEMINI_API_KEY para respostas de IA. " +
      `Recebi: ${last.slice(0, 240)}`,
  };
}

async function chatOpenAI(system, messages) {
  const apiKey = process.env.OPENAI_API_KEY;
  if (!apiKey) return { ok: false, error: "OPENAI_API_KEY is not configured." };

  const baseUrl = process.env.OPENAI_BASE_URL || "https://api.openai.com/v1";
  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model: process.env.OPENAI_MODEL || "gpt-4o-mini",
      messages: [{ role: "system", content: system }, ...messages],
      temperature: 0.2,
    }),
  });
  const json = await response.json();
  if (!response.ok) return { ok: false, provider: "openai", error: json.error?.message || response.statusText };
  return {
    ok: true,
    provider: "openai",
    model: json.model,
    message: json.choices?.[0]?.message?.content || "",
  };
}

async function chatGemini(system, messages) {
  const apiKey = process.env.GEMINI_API_KEY;
  if (!apiKey) return { ok: false, error: "GEMINI_API_KEY is not configured." };

  const model = process.env.GEMINI_MODEL || "gemini-1.5-flash";
  const last = messages[messages.length - 1]?.content || "";
  const response = await fetch(
    `https://generativelanguage.googleapis.com/v1beta/models/${encodeURIComponent(model)}:generateContent?key=${apiKey}`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        systemInstruction: { parts: [{ text: system }] },
        contents: [{ role: "user", parts: [{ text: last }] }],
        generationConfig: { temperature: 0.2 },
      }),
    },
  );
  const json = await response.json();
  if (!response.ok) return { ok: false, provider: "gemini", error: json.error?.message || response.statusText };
  return {
    ok: true,
    provider: "gemini",
    model,
    message: json.candidates?.[0]?.content?.parts?.map((part) => part.text).join("") || "",
  };
}

function matterSystemPrompt(source) {
  return [
    "You are Matter Studio, an assistant specialized in the Matter Core language.",
    "Matter Core has a parser, bytecode, VM/runtime, CLI, JSON tooling, reflect-json, and reflexive-guard-json.",
    "Be practical. Prefer valid .matter code and explain what can be run locally.",
    "When code changes are proposed, recommend check-json and reflexive-guard-json before run.",
    source ? `Current Matter source:\n${source.slice(0, 12000)}` : "",
  ].filter(Boolean).join("\n\n");
}

async function runMatter(body) {
  const action = String(body.action || "check-json");
  const source = normalizeSource(body.source);
  const allowed = new Set(["run", "check-json", "reflect-json", "reflexive-guard-json"]);
  if (!allowed.has(action)) return { ok: false, error: `Unsupported Matter action: ${action}` };
  if (!source.trim()) return { ok: false, error: "Matter source is empty." };

  const cli = resolveMatterCli();
  const args = cli.kind === "exe" ? [action, "-"] : ["run", "-q", "-p", "matter-cli", "--", action, "-"];
  const command = cli.kind === "exe" ? cli.command : "cargo";
  const result = await runProcess(command, args, source);
  return {
    ok: result.code === 0,
    action,
    command: cli.kind === "exe" ? cli.command : "cargo run -q -p matter-cli --",
    code: result.code,
    stdout: result.stdout,
    stderr: result.stderr,
  };
}

function normalizeSource(value) {
  if (typeof value === "string") return value;
  if (value && typeof value.value === "string") return value.value;
  return String(value || "");
}

function resolveMatterCli() {
  if (process.env.MATTER_CLI) return { kind: "exe", command: process.env.MATTER_CLI };
  const cargoTargetDir = readCargoTargetDir();
  const candidates = [
    cargoTargetDir && join(cargoTargetDir, "debug", "matter-cli.exe"),
    cargoTargetDir && join(cargoTargetDir, "debug", "matter-cli"),
    join(repoRoot, "target", "debug", "matter-cli.exe"),
    join(repoRoot, "target", "debug", "matter-cli"),
    cargoTargetDir && join(cargoTargetDir, "release", "matter-cli.exe"),
    cargoTargetDir && join(cargoTargetDir, "release", "matter-cli"),
    join(repoRoot, "target", "release", "matter-cli.exe"),
    join(repoRoot, "target", "release", "matter-cli"),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (existsSync(candidate)) return { kind: "exe", command: candidate };
  }
  return { kind: "cargo" };
}

function readCargoTargetDir() {
  const configPath = join(repoRoot, ".cargo", "config.toml");
  if (!existsSync(configPath)) return "";
  const config = readFileSync(configPath, "utf8");
  const match = config.match(/target-dir\s*=\s*"([^"]+)"/);
  return match ? resolve(repoRoot, match[1]) : "";
}

function runProcess(command, args, stdin) {
  return new Promise((resolveProcess) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      windowsHide: true,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk.toString()));
    child.stderr.on("data", (chunk) => (stderr += chunk.toString()));
    child.on("error", (error) => resolveProcess({ code: 1, stdout, stderr: String(error.message || error) }));
    child.on("close", (code) => resolveProcess({ code, stdout, stderr }));
    child.stdin.end(stdin);
  });
}

async function readRepoFile(path) {
  const content = await readFile(join(repoRoot, path), "utf8");
  return { path, content };
}
