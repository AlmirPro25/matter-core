const state = {
  provider: "local",
  messages: [
    {
      role: "assistant",
      content:
        "Matter Studio pronto. Carregue um exemplo, rode a VM, reflita o codigo, ou configure OPENAI_API_KEY/GEMINI_API_KEY para conectar uma IA real ao Matter Core.",
    },
  ],
  source: "",
  output: "Output do Matter aparece aqui.",
  visual: null,
  activeExample: "",
  status: null,
};

const app = document.querySelector("#app");

init();

async function init() {
  await loadStatus();
  await loadExamples();
  render();
}

async function loadStatus() {
  state.status = await api("/api/status");
}

async function loadExamples() {
  const result = await api("/api/examples");
  state.examples = result.examples || [];
  const first = state.examples[0];
  if (first) {
    state.source = first.content;
    state.activeExample = first.path;
  }
}

function render() {
  app.innerHTML = `
    <div class="shell">
      ${sidebar()}
      ${mainPanel()}
      ${workbench()}
    </div>
  `;
  wireEvents();
}

function sidebar() {
  const providers = state.status?.providers || {};
  return `
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><span class="spark"></span><span>Matter Studio</span></div>
        <button class="icon-button" title="Novo chat" data-action="new-chat">+</button>
      </div>

      <div class="segmented">
        <button class="active">Chat</button>
        <button>Code</button>
      </div>

      <div class="nav">
        <button data-action="ask-architecture"># Arquitetura</button>
        <button data-action="ask-syntax">&lt;&gt; Sintaxe</button>
        <button data-action="ask-reflection">o Reflexao</button>
        <button data-action="ask-product">-&gt; Produto</button>
      </div>

      <div class="section-title">Exemplos</div>
      <div class="examples">
        ${(state.examples || [])
          .map(
            (example) => `
              <button class="example-item ${example.path === state.activeExample ? "active" : ""}" data-example="${escapeHtml(example.path)}">
                <span>${escapeHtml(example.path.replace("examples/", ""))}</span>
                <span>&gt;</span>
              </button>
            `,
          )
          .join("")}
      </div>

      <div class="sidebar-footer">
        <div><span class="provider-dot ${providers.openai ? "on" : ""}"></span>OpenAI API</div>
        <div><span class="provider-dot ${providers.gemini ? "on" : ""}"></span>Gemini API</div>
        <div class="muted">Chaves ficam so no servidor local.</div>
      </div>
    </aside>
  `;
}

function mainPanel() {
  return `
    <main class="main">
      <header class="topbar">
        <div class="title">Analisar e construir Matter <span class="muted">v</span></div>
        <div class="toolbar">
          <select id="provider">
            <option value="local" ${state.provider === "local" ? "selected" : ""}>Local</option>
            <option value="openai" ${state.provider === "openai" ? "selected" : ""}>OpenAI</option>
            <option value="gemini" ${state.provider === "gemini" ? "selected" : ""}>Gemini</option>
          </select>
          <button class="icon-button" title="Enviar" data-action="send">&gt;</button>
        </div>
      </header>

      <section class="messages" id="messages">
        ${state.messages.map(messageTemplate).join("")}
      </section>

      <section class="composer">
        <div class="composer-box">
          <textarea id="prompt" placeholder="Converse com a linguagem, peca analise, gere codigo Matter..."></textarea>
          <div class="composer-actions">
            <button class="icon-button" title="Anexar contexto">+</button>
            <div class="toolbar">
              <span class="muted">${modelLabel()}</span>
              <button class="send" data-action="send" title="Enviar">^</button>
            </div>
          </div>
        </div>
      </section>
    </main>
  `;
}

function workbench() {
  return `
    <aside class="workbench">
      <header class="panel-top">
        <div class="title">Workbench Matter</div>
        <span class="pill">MBC1 VM</span>
      </header>

      <div class="code-area">
        <textarea class="code" id="source">${escapeHtml(state.source)}</textarea>
        <div class="runbar">
          <button class="primary" data-matter="run">Run</button>
          <button data-matter="check-json">Check</button>
          <button data-matter="reflect-json">Reflect</button>
          <button data-matter="reflexive-guard-json">Guard</button>
          <button data-action="render-ui">Visual</button>
        </div>
      </div>

      <section class="output">
        <div class="cards">
          <div class="card"><b>VM</b><span class="muted">source -> bytecode</span></div>
          <div class="card"><b>Reflect</b><span class="muted">code as data</span></div>
          <div class="card"><b>Guard</b><span class="muted">policy gate</span></div>
        </div>
        ${visualPreview()}
        <pre>${escapeHtml(state.output)}</pre>
      </section>
    </aside>
  `;
}

function visualPreview() {
  if (!state.visual) return "";
  const surface = state.visual.surfaces?.[0] || { width: 1280, height: 720, name: "surface" };
  const width = Number(surface.width) || 1280;
  const height = Number(surface.height) || 720;
  const regions = state.visual.regions || [];
  return `
    <div class="visual-preview">
      <div class="visual-meta">${escapeHtml(surface.name)} ${width}x${height}</div>
      <div class="visual-canvas" style="aspect-ratio:${width}/${height}">
        ${regions.map((region) => visualRegion(region, width, height)).join("")}
      </div>
    </div>
  `;
}

function visualRegion(region, width, height) {
  const props = region.properties || {};
  const x = percent(region.x, width);
  const y = percent(region.y, height);
  const w = percent(region.w, width);
  const h = percent(region.h, height);
  const label = props.text || region.name;
  const stateClass = props.state === "active" ? " active" : "";
  return `
    <div class="visual-region${stateClass}" style="left:${x}%;top:${y}%;width:${w}%;height:${h}%">
      <b>${escapeHtml(label)}</b>
      <span>${escapeHtml(props.semantic || props.event || region.name)}</span>
    </div>
  `;
}

function messageTemplate(message) {
  const cls = message.role === "assistant" ? "assistant" : "user";
  return `
    <article class="message ${cls}">
      <div class="role">${message.role}</div>
      <div class="bubble">${escapeHtml(message.content)}</div>
    </article>
  `;
}

function wireEvents() {
  document.querySelector("#provider")?.addEventListener("change", (event) => {
    state.provider = event.target.value;
    render();
  });

  document.querySelectorAll("[data-action='send']").forEach((button) => {
    button.addEventListener("click", sendPrompt);
  });

  document.querySelector("#prompt")?.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) sendPrompt();
  });

  document.querySelector("#source")?.addEventListener("input", (event) => {
    state.source = event.target.value;
  });

  document.querySelectorAll("[data-matter]").forEach((button) => {
    button.addEventListener("click", () => runMatter(button.dataset.matter));
  });

  document.querySelector("[data-action='render-ui']")?.addEventListener("click", renderMatterUi);

  document.querySelectorAll("[data-example]").forEach((button) => {
    button.addEventListener("click", () => {
      const example = state.examples.find((item) => item.path === button.dataset.example);
      if (!example) return;
      state.source = example.content;
      state.activeExample = example.path;
      state.output = `Carregado: ${example.path}`;
      render();
    });
  });

  document.querySelector("[data-action='new-chat']")?.addEventListener("click", () => {
    state.messages = [];
    render();
  });

  const quickPrompts = {
    "ask-architecture": "Explique a arquitetura deste programa Matter e como ele vira bytecode.",
    "ask-syntax": "Explique a sintaxe usada neste codigo e sugira uma melhoria simples.",
    "ask-reflection": "Use a ideia de reflect-json e explique o que este codigo revela sobre a VM.",
    "ask-product": "Transforme este exemplo em uma demo forte para mostrar Matter Core a outro dev.",
  };
  Object.entries(quickPrompts).forEach(([action, prompt]) => {
    document.querySelector(`[data-action='${action}']`)?.addEventListener("click", () => {
      const input = document.querySelector("#prompt");
      input.value = prompt;
      input.focus();
    });
  });
}

async function sendPrompt() {
  const input = document.querySelector("#prompt");
  const content = input.value.trim();
  if (!content) return;
  input.value = "";
  state.messages.push({ role: "user", content });
  state.messages.push({ role: "assistant", content: "Pensando..." });
  render();

  const result = await api("/api/chat", {
    provider: state.provider,
    source: state.source,
    messages: state.messages.filter((msg) => msg.content !== "Pensando..."),
  });
  state.messages.pop();
  state.messages.push({
    role: "assistant",
    content: result.ok ? result.message : `Erro: ${result.error}`,
  });
  render();
}

async function runMatter(action) {
  state.source = document.querySelector("#source")?.value || state.source;
  state.output = `Executando ${action}...`;
  state.visual = null;
  render();
  const result = await api("/api/matter", { action, source: state.source });
  const payload = result.stdout || result.stderr || JSON.stringify(result, null, 2);
  state.output = result.ok ? payload : `Erro (${result.code}):\n${payload}`;
  render();
}

async function renderMatterUi() {
  state.source = document.querySelector("#source")?.value || state.source;
  state.output = "Renderizando UI Matter...";
  state.visual = null;
  render();
  const result = await api("/api/ui-render", { source: state.source });
  if (result.ok) {
    state.visual = result.snapshot;
    state.output = [
      "UI renderizada a partir de codigo Matter.",
      ...(result.output || []),
      `regions=${result.snapshot?.regions?.length || 0}`,
    ].join("\n");
  } else {
    state.output = `Erro visual:\n${result.stderr || result.error || JSON.stringify(result, null, 2)}`;
  }
  render();
}

async function api(path, body) {
  const options = body
    ? { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
    : {};
  const response = await fetch(path, options);
  return response.json();
}

function modelLabel() {
  if (state.provider === "openai") return state.status?.models?.openai || "OpenAI";
  if (state.provider === "gemini") return state.status?.models?.gemini || "Gemini";
  return "Local";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function percent(value, total) {
  const number = Number(value) || 0;
  const base = Number(total) || 1;
  return Math.max(0, Math.min(100, (number / base) * 100)).toFixed(3);
}
