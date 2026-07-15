(() => {
  const TOKEN_KEY = "axleops_admin_token";

  const els = {
    viewLogin: document.getElementById("view-login"),
    viewApp: document.getElementById("view-app"),
    formLogin: document.getElementById("form-login"),
    loginToken: document.getElementById("login-token"),
    loginError: document.getElementById("login-error"),
    btnRefresh: document.getElementById("btn-refresh"),
    btnLogout: document.getElementById("btn-logout"),
    agentList: document.getElementById("agent-list"),
    agentEmpty: document.getElementById("agent-empty"),
    btnShowRegister: document.getElementById("btn-show-register"),
    panelRegister: document.getElementById("panel-register"),
    panelDetail: document.getElementById("panel-detail"),
    panelWelcome: document.getElementById("panel-welcome"),
    formRegister: document.getElementById("form-register"),
    registerError: document.getElementById("register-error"),
    btnCancelRegister: document.getElementById("btn-cancel-register"),
    detailName: document.getElementById("detail-name"),
    detailUrl: document.getElementById("detail-url"),
    btnPing: document.getElementById("btn-ping"),
    btnDeleteAgent: document.getElementById("btn-delete-agent"),
    pingResult: document.getElementById("ping-result"),
    serviceList: document.getElementById("service-list"),
    serviceEmpty: document.getElementById("service-empty"),
    serviceCount: document.getElementById("service-count"),
    agentCount: document.getElementById("agent-count"),
    btnWelcomeRegister: document.getElementById("btn-welcome-register"),
    btnReloadServices: document.getElementById("btn-reload-services"),
    formStart: document.getElementById("form-start"),
    formTitle: document.getElementById("form-title"),
    fieldName: document.getElementById("field-name"),
    startKind: document.getElementById("start-kind"),
    startError: document.getElementById("start-error"),
    btnSaveService: document.getElementById("btn-save-service"),
    btnStartService: document.getElementById("btn-start-service"),
    btnResetForm: document.getElementById("btn-reset-form"),
    logBytes: document.getElementById("log-bytes"),
    btnFetchLogs: document.getElementById("btn-fetch-logs"),
    logTarget: document.getElementById("log-target"),
    logContent: document.getElementById("log-content"),
    toast: document.getElementById("toast"),
  };

  const state = {
    agents: [],
    selectedId: null,
    logService: null,
    editingName: null,
  };

  function getToken() {
    return sessionStorage.getItem(TOKEN_KEY) || "";
  }

  function setToken(token) {
    sessionStorage.setItem(TOKEN_KEY, token);
  }

  function clearToken() {
    sessionStorage.removeItem(TOKEN_KEY);
  }

  function showToast(message, isError = false) {
    els.toast.hidden = false;
    els.toast.textContent = message;
    els.toast.classList.toggle("error", isError);
    clearTimeout(showToast._t);
    showToast._t = setTimeout(() => {
      els.toast.hidden = true;
    }, 2600);
  }

  function setBusy(btn, busy, label) {
    if (!btn) return;
    if (busy) {
      btn.dataset.label = btn.textContent;
      btn.classList.add("busy");
      btn.disabled = true;
      if (label) btn.textContent = label;
    } else {
      btn.classList.remove("busy");
      btn.disabled = false;
      if (btn.dataset.label) btn.textContent = btn.dataset.label;
    }
  }

  function initialOf(name) {
    const s = String(name || "?").trim();
    return (s[0] || "?").toUpperCase();
  }

  function splitArgs(text) {
    if (!text || !text.trim()) return [];
    return text.trim().split(/\s+/);
  }

  async function api(path, options = {}) {
    const headers = Object.assign(
      { "Content-Type": "application/json" },
      options.headers || {},
      { "X-AxleOps-Token": getToken() }
    );
    const res = await fetch(path, { ...options, headers });
    let body = null;
    const text = await res.text();
    if (text) {
      try {
        body = JSON.parse(text);
      } catch {
        body = { ok: false, message: text };
      }
    }
    if (res.status === 401) {
      throw new Error("Token 无效或未授权");
    }
    if (!res.ok) {
      const msg = (body && body.message) || `HTTP ${res.status}`;
      throw new Error(msg);
    }
    return body;
  }

  function showLogin() {
    els.viewLogin.classList.remove("hidden");
    els.viewApp.classList.add("hidden");
  }

  function showApp() {
    els.viewLogin.classList.add("hidden");
    els.viewApp.classList.remove("hidden");
  }

  function showWelcome() {
    state.selectedId = null;
    state.logService = null;
    els.panelWelcome.classList.remove("hidden");
    els.panelDetail.classList.add("hidden");
    els.panelRegister.classList.add("hidden");
    els.pingResult.hidden = true;
    els.logContent.textContent = "";
    els.logTarget.textContent = "选择服务后查看日志";
    els.btnFetchLogs.disabled = true;
    highlightAgent(null);
  }

  function showRegister() {
    els.panelWelcome.classList.add("hidden");
    els.panelDetail.classList.add("hidden");
    els.panelRegister.classList.remove("hidden");
    els.registerError.hidden = true;
  }

  function showDetail() {
    els.panelWelcome.classList.add("hidden");
    els.panelRegister.classList.add("hidden");
    els.panelDetail.classList.remove("hidden");
  }

  function highlightAgent(id) {
    els.agentList.querySelectorAll(".agent-item").forEach((el) => {
      el.classList.toggle("active", el.dataset.id === id);
    });
  }

  function updateKindFields() {
    const kind = els.startKind.value;
    document.querySelectorAll(".kind-jar").forEach((el) => {
      el.classList.toggle("hidden", kind !== "jar");
    });
    document.querySelectorAll(".kind-script").forEach((el) => {
      el.classList.toggle("hidden", kind !== "script");
    });
    document.querySelectorAll(".kind-command").forEach((el) => {
      el.classList.toggle("hidden", kind !== "command");
    });
  }

  function renderAgents() {
    els.agentList.innerHTML = "";
    if (els.agentCount) els.agentCount.textContent = String(state.agents.length);
    els.agentEmpty.hidden = state.agents.length > 0;
    state.agents.forEach((agent, i) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "agent-item";
      btn.dataset.id = agent.id;
      btn.style.animationDelay = `${i * 45}ms`;
      const tags = (agent.tags || [])
        .slice(0, 3)
        .map((t) => `<span class="tag">${escapeHtml(t)}</span>`)
        .join("");
      btn.innerHTML = `
        <span class="agent-avatar">${escapeHtml(initialOf(agent.name))}</span>
        <span>
          <span class="name">${escapeHtml(agent.name)}</span>
          <span class="meta">${escapeHtml(agent.base_url)}</span>
          ${tags ? `<span class="tag-row">${tags}</span>` : ""}
        </span>
      `;
      btn.addEventListener("click", () => selectAgent(agent.id));
      els.agentList.appendChild(btn);
    });
    highlightAgent(state.selectedId);
  }

  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  async function loadAgents() {
    const res = await api("/api/v1/agents");
    state.agents = (res && res.data) || [];
    renderAgents();
    if (state.selectedId) {
      const still = state.agents.find((a) => a.id === state.selectedId);
      if (!still) showWelcome();
      else await openAgentDetail(still);
    }
  }

  async function selectAgent(id) {
    state.selectedId = id;
    highlightAgent(id);
    const agent = state.agents.find((a) => a.id === id);
    if (!agent) return;
    await openAgentDetail(agent);
  }

  async function openAgentDetail(agent) {
    showDetail();
    els.detailName.textContent = agent.name;
    els.detailUrl.textContent = agent.base_url;
    els.pingResult.hidden = true;
    state.logService = null;
    els.logContent.textContent = "";
    els.logTarget.textContent = "选择服务后查看日志";
    els.btnFetchLogs.disabled = true;
    resetServiceForm();
    await loadServices();
  }

  function unwrapAgentData(payload) {
    // Admin wraps agent response; agent itself also wraps with ok/data
    if (!payload) return null;
    let data = payload.data !== undefined ? payload.data : payload;
    if (data && typeof data === "object" && data.data !== undefined && data.ok !== undefined) {
      data = data.data;
    }
    return data;
  }

  function joinArgs(arr) {
    return Array.isArray(arr) ? arr.join(" ") : "";
  }

  function parseEnvText(text) {
    const env = {};
    String(text || "")
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith("#"))
      .forEach((line) => {
        const i = line.indexOf("=");
        if (i <= 0) return;
        const k = line.slice(0, i).trim();
        const v = line.slice(i + 1).trim();
        if (k) env[k] = v;
      });
    return env;
  }

  function formatEnv(env) {
    if (!env || typeof env !== "object") return "";
    return Object.keys(env)
      .sort()
      .map((k) => `${k}=${env[k]}`)
      .join("\n");
  }

  function buildServiceBody() {
    const fd = new FormData(els.formStart);
    const kind = String(fd.get("kind") || "jar");
    const body = {
      name: String(fd.get("name") || "").trim(),
      kind,
      work_dir: String(fd.get("work_dir") || "").trim() || null,
      health_url: String(fd.get("health_url") || "").trim() || null,
      jvm_args: [],
      app_args: [],
      args: [],
      env: parseEnvText(fd.get("env")),
    };
    if (kind === "jar") {
      body.jar_path = String(fd.get("jar_path") || "").trim() || null;
      body.jvm_args = splitArgs(fd.get("jvm_args"));
      body.app_args = splitArgs(fd.get("app_args"));
    } else if (kind === "script") {
      body.script_path = String(fd.get("script_path") || "").trim() || null;
      body.interpreter = String(fd.get("interpreter") || "").trim() || null;
      body.args = splitArgs(fd.get("args"));
    } else {
      body.command = String(fd.get("command") || "").trim() || null;
      body.args = splitArgs(fd.get("args"));
    }
    return body;
  }

  function fillServiceForm(spec) {
    const form = els.formStart;
    const kind = (spec.kind || "jar").toLowerCase();
    els.fieldName.value = spec.name || "";
    els.startKind.value = ["jar", "script", "command"].includes(kind) ? kind : "jar";
    form.elements.namedItem("jar_path").value = spec.jar_path || "";
    form.elements.namedItem("jvm_args").value = joinArgs(spec.jvm_args);
    form.elements.namedItem("app_args").value = joinArgs(spec.app_args);
    form.elements.namedItem("script_path").value = spec.script_path || "";
    form.elements.namedItem("interpreter").value = spec.interpreter || "";
    form.elements.namedItem("command").value = spec.command || "";
    form.elements.namedItem("args").value = joinArgs(spec.args);
    form.elements.namedItem("work_dir").value = spec.work_dir || "";
    form.elements.namedItem("health_url").value = spec.health_url || "";
    const envField = form.elements.namedItem("env");
    if (envField) envField.value = formatEnv(spec.env);
    updateKindFields();
  }

  function setEditingMode(name) {
    state.editingName = name;
    els.fieldName.readOnly = true;
    els.formTitle.textContent = `编辑 · ${name}`;
  }

  function resetServiceForm() {
    state.editingName = null;
    els.formStart.reset();
    els.startKind.value = "jar";
    els.fieldName.readOnly = false;
    els.formTitle.textContent = "配置服务";
    els.startError.hidden = true;
    updateKindFields();
  }

  async function loadServices() {
    if (!state.selectedId) return;
    els.serviceList.innerHTML = "";
    try {
      const res = await api(`/api/v1/agents/${state.selectedId}/services`);
      const data = unwrapAgentData(res);
      const list = Array.isArray(data) ? data : [];
      if (els.serviceCount) els.serviceCount.textContent = String(list.length);
      els.serviceEmpty.hidden = list.length > 0;
      list.forEach((svc, i) => {
        const row = document.createElement("div");
        row.className = "service-item";
        row.dataset.name = svc.name || "";
        row.style.animationDelay = `${i * 45}ms`;
        if (state.logService && state.logService === svc.name) {
          row.classList.add("log-active");
        }
        const rawState = svc && svc.state;
        const stateName = String(
          rawState && typeof rawState === "object"
            ? rawState.name || rawState.state || "unknown"
            : rawState || "unknown"
        )
          .trim()
          .toLowerCase();
        const running = stateName === "running" || stateName === "unhealthy";
        const target = svc.target || svc.jar_path || "—";
        const kind = svc.kind ? String(svc.kind) : "";
        const msg = svc.message ? `<span class="meta">${escapeHtml(svc.message)}</span>` : "";
        row.innerHTML = `
          <div class="info">
            <span class="state-pill ${escapeHtml(stateName)}">${escapeHtml(stateName)}</span>
            <strong>${escapeHtml(svc.name || "")}</strong>
            <span class="meta">${escapeHtml(target)}</span>
            <span class="meta">${escapeHtml(kind)}${svc.pid ? " · pid " + svc.pid : ""}</span>
            ${msg}
          </div>
          <div class="row-actions">
            <button type="button" class="btn small" data-act="logs">日志</button>
            <button type="button" class="btn small" data-act="edit">编辑</button>
            <button type="button" class="btn small" data-act="restart">重启</button>
            ${
              running
                ? `<button type="button" class="btn small danger" data-act="stop">停止</button>`
                : `<button type="button" class="btn small cta-start" data-act="start">启动</button>`
            }
            <button type="button" class="btn small danger" data-act="remove">移除</button>
          </div>
        `;
        row.querySelector('[data-act="logs"]').addEventListener("click", () => {
          state.logService = svc.name;
          els.logTarget.textContent = `tail · ${svc.name}`;
          els.btnFetchLogs.disabled = false;
          els.serviceList.querySelectorAll(".service-item").forEach((el) => {
            el.classList.toggle("log-active", el.dataset.name === svc.name);
          });
          fetchLogs();
        });
        row.querySelector('[data-act="edit"]').addEventListener("click", () => editService(svc.name));
        row.querySelector('[data-act="restart"]').addEventListener("click", async (ev) => {
          setBusy(ev.currentTarget, true, "…");
          await restartService(svc.name);
        });
        row.querySelector('[data-act="remove"]').addEventListener("click", () => removeService(svc.name));
        const stopBtn = row.querySelector('[data-act="stop"]');
        if (stopBtn) {
          stopBtn.addEventListener("click", async () => {
            setBusy(stopBtn, true, "…");
            await stopService(svc.name);
          });
        }
        const startBtn = row.querySelector('[data-act="start"]');
        if (startBtn) {
          startBtn.addEventListener("click", async () => {
            setBusy(startBtn, true, "…");
            await startSavedService(svc.name);
          });
        }
        els.serviceList.appendChild(row);
      });
    } catch (e) {
      els.serviceEmpty.hidden = true;
      if (els.serviceCount) els.serviceCount.textContent = "0";
      showToast(e.message, true);
    }
  }

  async function editService(name) {
    if (!state.selectedId) return;
    try {
      const res = await api(
        `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/spec`
      );
      const spec = unwrapAgentData(res) || {};
      fillServiceForm(spec);
      setEditingMode(name);
      els.formStart.scrollIntoView({ behavior: "smooth", block: "nearest" });
      showToast(`已载入 ${name}`);
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function startSavedService(name) {
    if (!state.selectedId) return;
    try {
      await api(`/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/start`, {
        method: "POST",
      });
      showToast(`已启动 ${name}`);
      await loadServices();
    } catch (e) {
      showToast(e.message, true);
      await loadServices();
    }
  }

  async function restartService(name) {
    if (!state.selectedId) return;
    try {
      await api(
        `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/restart`,
        { method: "POST" }
      );
      showToast(`已重启 ${name}`);
      await loadServices();
    } catch (e) {
      showToast(e.message, true);
      await loadServices();
    }
  }

  async function removeService(name) {
    if (!state.selectedId) return;
    if (!confirm(`确认移除服务「${name}」？运行中会被先停止，配置定义将删除。`)) return;
    try {
      await api(`/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}`, {
        method: "DELETE",
      });
      showToast(`已移除 ${name}`);
      if (state.editingName === name) resetServiceForm();
      if (state.logService === name) {
        state.logService = null;
        els.logTarget.textContent = "从服务列表点「日志」载入输出";
        els.btnFetchLogs.disabled = true;
        els.logContent.textContent = "";
      }
      await loadServices();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function saveServiceOnly() {
    if (!state.selectedId) return;
    els.startError.hidden = true;
    const body = buildServiceBody();
    if (!body.name) {
      els.startError.textContent = "名称必填";
      els.startError.hidden = false;
      return;
    }
    try {
      if (state.editingName) {
        await api(
          `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(state.editingName)}`,
          { method: "PUT", body: JSON.stringify(body) }
        );
      } else {
        await api(`/api/v1/agents/${state.selectedId}/services`, {
          method: "POST",
          body: JSON.stringify(body),
        });
      }
      showToast("已保存（未启动）");
      setEditingMode(body.name);
      await loadServices();
    } catch (e) {
      els.startError.textContent = e.message;
      els.startError.hidden = false;
    }
  }

  async function startFromForm() {
    if (!state.selectedId) return;
    els.startError.hidden = true;
    const body = buildServiceBody();
    if (!body.name) {
      els.startError.textContent = "名称必填";
      els.startError.hidden = false;
      return;
    }
    try {
      await api(`/api/v1/agents/${state.selectedId}/services/start`, {
        method: "POST",
        body: JSON.stringify(body),
      });
      showToast(`已启动 ${body.name}`);
      setEditingMode(body.name);
      await loadServices();
    } catch (e) {
      els.startError.textContent = e.message;
      els.startError.hidden = false;
    }
  }

  async function pingAgent() {
    if (!state.selectedId) return;
    try {
      const res = await api(`/api/v1/agents/${state.selectedId}/ping`);
      const data = res.data || {};
      els.pingResult.hidden = false;
      els.pingResult.classList.toggle("ok", !!data.reachable);
      els.pingResult.classList.toggle("bad", !data.reachable);
      els.pingResult.textContent = JSON.stringify(data, null, 2);
      showToast(data.reachable ? "Agent 可达" : "Agent 不可达", !data.reachable);
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function deleteAgent() {
    if (!state.selectedId) return;
    if (!confirm("确认删除该 Agent 注册信息？")) return;
    try {
      await api(`/api/v1/agents/${state.selectedId}`, { method: "DELETE" });
      showToast("已删除");
      showWelcome();
      await loadAgents();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function stopService(name) {
    if (!state.selectedId) return;
    try {
      await api(`/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/stop`, {
        method: "POST",
      });
      showToast(`已停止 ${name}`);
      await loadServices();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function fetchLogs() {
    if (!state.selectedId || !state.logService) return;
    const bytes = Number(els.logBytes.value) || 32768;
    try {
      const path = `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(
        state.logService
      )}/logs?bytes=${bytes}`;
      const res = await api(path);
      const data = unwrapAgentData(res);
      els.logContent.textContent = typeof data === "string" ? data : JSON.stringify(data, null, 2);
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function verifyToken(token) {
    const res = await fetch("/api/v1/agents", {
      headers: { "X-AxleOps-Token": token },
    });
    if (res.status === 401) throw new Error("Token 无效");
    if (!res.ok) {
      const text = await res.text();
      throw new Error(text || `HTTP ${res.status}`);
    }
  }

  // Events
  els.formLogin.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    els.loginError.hidden = true;
    const token = els.loginToken.value.trim();
    try {
      await verifyToken(token);
      setToken(token);
      showApp();
      await loadAgents();
      showWelcome();
    } catch (e) {
      els.loginError.textContent = e.message;
      els.loginError.hidden = false;
    }
  });

  els.btnLogout.addEventListener("click", () => {
    clearToken();
    showLogin();
  });

  els.btnRefresh.addEventListener("click", async () => {
    try {
      await loadAgents();
      showToast("已刷新");
    } catch (e) {
      showToast(e.message, true);
    }
  });

  els.btnShowRegister.addEventListener("click", showRegister);
  els.btnCancelRegister.addEventListener("click", () => {
    if (state.selectedId) {
      const agent = state.agents.find((a) => a.id === state.selectedId);
      if (agent) openAgentDetail(agent);
      else showWelcome();
    } else {
      showWelcome();
    }
  });

  els.formRegister.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    els.registerError.hidden = true;
    const fd = new FormData(els.formRegister);
    const tags = String(fd.get("tags") || "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const body = {
      name: String(fd.get("name") || "").trim(),
      base_url: String(fd.get("base_url") || "").trim(),
      token: String(fd.get("token") || "").trim(),
      tags,
    };
    try {
      const res = await api("/api/v1/agents", {
        method: "POST",
        body: JSON.stringify(body),
      });
      showToast("注册成功");
      els.formRegister.reset();
      await loadAgents();
      if (res.data && res.data.id) {
        await selectAgent(res.data.id);
      }
    } catch (e) {
      els.registerError.textContent = e.message;
      els.registerError.hidden = false;
    }
  });

  els.btnPing.addEventListener("click", pingAgent);
  els.btnDeleteAgent.addEventListener("click", deleteAgent);
  els.btnReloadServices.addEventListener("click", loadServices);
  els.btnFetchLogs.addEventListener("click", fetchLogs);
  els.startKind.addEventListener("change", updateKindFields);
  els.btnSaveService.addEventListener("click", async () => {
    setBusy(els.btnSaveService, true, "保存中…");
    try {
      await saveServiceOnly();
    } finally {
      setBusy(els.btnSaveService, false);
    }
  });
  els.btnStartService.addEventListener("click", async () => {
    setBusy(els.btnStartService, true, "启动中…");
    try {
      await startFromForm();
    } finally {
      setBusy(els.btnStartService, false);
    }
  });
  els.btnResetForm.addEventListener("click", resetServiceForm);
  if (els.btnWelcomeRegister) {
    els.btnWelcomeRegister.addEventListener("click", showRegister);
  }
  els.formStart.addEventListener("submit", (ev) => {
    ev.preventDefault();
    els.btnStartService.click();
  });

  // Boot
  updateKindFields();
  (async () => {
    const token = getToken();
    if (!token) {
      showLogin();
      return;
    }
    try {
      await verifyToken(token);
      showApp();
      await loadAgents();
      showWelcome();
    } catch {
      clearToken();
      showLogin();
    }
  })();
})();
