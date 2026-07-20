(() => {
  const SESSION_KEY = "axleops_admin_session";
  const USER_KEY = "axleops_admin_user";
  const THEME_KEY = "axleops_theme";
  const THEMES = [
    { id: "dark", label: "深色" },
    { id: "light", label: "浅色" },
    { id: "contrast", label: "高对比" },
  ];

  function currentThemeId() {
    const id = document.documentElement.getAttribute("data-theme") || "dark";
    return THEMES.some((t) => t.id === id) ? id : "dark";
  }

  function applyTheme(id) {
    const theme = THEMES.find((t) => t.id === id) || THEMES[0];
    document.documentElement.setAttribute("data-theme", theme.id);
    try {
      localStorage.setItem(THEME_KEY, theme.id);
    } catch (_) {}
    document.querySelectorAll("[data-theme-toggle]").forEach((btn) => {
      btn.textContent = theme.label;
      btn.title = `主题：${theme.label}（点击切换）`;
      btn.setAttribute("aria-label", `当前主题 ${theme.label}，点击切换`);
    });
  }

  function cycleTheme() {
    const i = THEMES.findIndex((t) => t.id === currentThemeId());
    applyTheme(THEMES[(i + 1) % THEMES.length].id);
  }

  applyTheme(currentThemeId());
  document.querySelectorAll("[data-theme-toggle]").forEach((btn) => {
    btn.addEventListener("click", cycleTheme);
  });

  const els = {
    viewLogin: document.getElementById("view-login"),
    viewApp: document.getElementById("view-app"),
    formLogin: document.getElementById("form-login"),
    loginUsername: document.getElementById("login-username"),
    loginPassword: document.getElementById("login-password"),
    loginError: document.getElementById("login-error"),
    btnRefresh: document.getElementById("btn-refresh"),
    btnLogout: document.getElementById("btn-logout"),
    btnPassword: document.getElementById("btn-password"),
    currentUser: document.getElementById("current-user"),
    btnToggleToken: document.getElementById("btn-toggle-token"),
    tabOverview: document.getElementById("tab-overview"),
    tabAgents: document.getElementById("tab-agents"),
    tabProxies: document.getElementById("tab-proxies"),
    tabSystem: document.getElementById("tab-system"),
    railOverview: document.getElementById("rail-overview"),
    railAgents: document.getElementById("rail-agents"),
    railProxies: document.getElementById("rail-proxies"),
    railSystem: document.getElementById("rail-system"),
    panelOverview: document.getElementById("panel-overview"),
    overviewList: document.getElementById("overview-list"),
    overviewEmpty: document.getElementById("overview-empty"),
    overviewSummary: document.getElementById("overview-summary"),
    btnReloadOverview: document.getElementById("btn-reload-overview"),
    btnRefreshOverview: document.getElementById("btn-refresh-overview"),
    btnRotateAgentToken: document.getElementById("btn-rotate-agent-token"),
    btnRotateProxyToken: document.getElementById("btn-rotate-proxy-token"),
    btnShowUsers: document.getElementById("btn-show-users"),
    btnShowAudit: document.getElementById("btn-show-audit"),
    panelPassword: document.getElementById("panel-password"),
    panelUsers: document.getElementById("panel-users"),
    panelAudit: document.getElementById("panel-audit"),
    formPassword: document.getElementById("form-password"),
    passwordError: document.getElementById("password-error"),
    btnCancelPassword: document.getElementById("btn-cancel-password"),
    formCreateUser: document.getElementById("form-create-user"),
    usersError: document.getElementById("users-error"),
    userList: document.getElementById("user-list"),
    btnCancelUsers: document.getElementById("btn-cancel-users"),
    auditList: document.getElementById("audit-list"),
    auditEmpty: document.getElementById("audit-empty"),
    btnReloadAudit: document.getElementById("btn-reload-audit"),
    btnCancelAudit: document.getElementById("btn-cancel-audit"),
    agentList: document.getElementById("agent-list"),
    agentEmpty: document.getElementById("agent-empty"),
    proxyList: document.getElementById("proxy-list"),
    proxyEmpty: document.getElementById("proxy-empty"),
    btnShowRegister: document.getElementById("btn-show-register"),
    btnShowProxyRegister: document.getElementById("btn-show-proxy-register"),
    panelRegister: document.getElementById("panel-register"),
    panelProxyRegister: document.getElementById("panel-proxy-register"),
    panelProxyDetail: document.getElementById("panel-proxy-detail"),
    panelDetail: document.getElementById("panel-detail"),
    panelWelcome: document.getElementById("panel-welcome"),
    formRegister: document.getElementById("form-register"),
    formProxyRegister: document.getElementById("form-proxy-register"),
    registerError: document.getElementById("register-error"),
    proxyRegisterError: document.getElementById("proxy-register-error"),
    btnCancelRegister: document.getElementById("btn-cancel-register"),
    btnCancelProxyRegister: document.getElementById("btn-cancel-proxy-register"),
    detailName: document.getElementById("detail-name"),
    detailUrl: document.getElementById("detail-url"),
    detailOnline: document.getElementById("detail-online"),
    detailVia: document.getElementById("detail-via"),
    btnPing: document.getElementById("btn-ping"),
    btnDeleteAgent: document.getElementById("btn-delete-agent"),
    pingResult: document.getElementById("ping-result"),
    proxyDetailName: document.getElementById("proxy-detail-name"),
    proxyDetailUrl: document.getElementById("proxy-detail-url"),
    proxyDetailOnline: document.getElementById("proxy-detail-online"),
    proxyDetailNotes: document.getElementById("proxy-detail-notes"),
    btnPingProxy: document.getElementById("btn-ping-proxy"),
    btnReloadUpstreams: document.getElementById("btn-reload-upstreams"),
    btnDeleteProxy: document.getElementById("btn-delete-proxy"),
    btnImportAll: document.getElementById("btn-import-all"),
    proxyPingResult: document.getElementById("proxy-ping-result"),
    upstreamList: document.getElementById("upstream-list"),
    upstreamEmpty: document.getElementById("upstream-empty"),
    upstreamCount: document.getElementById("upstream-count"),
    formUpstream: document.getElementById("form-upstream"),
    upstreamFormTitle: document.getElementById("upstream-form-title"),
    upstreamId: document.getElementById("upstream-id"),
    upstreamName: document.getElementById("upstream-name"),
    upstreamBaseUrl: document.getElementById("upstream-base-url"),
    upstreamToken: document.getElementById("upstream-token"),
    upstreamFormError: document.getElementById("upstream-form-error"),
    btnSaveUpstream: document.getElementById("btn-save-upstream"),
    btnResetUpstreamForm: document.getElementById("btn-reset-upstream-form"),
    serviceList: document.getElementById("service-list"),
    serviceEmpty: document.getElementById("service-empty"),
    serviceBatchBar: document.getElementById("service-batch-bar"),
    serviceCount: document.getElementById("service-count"),
    agentCount: document.getElementById("agent-count"),
    proxyCount: document.getElementById("proxy-count"),
    btnWelcomeRegister: document.getElementById("btn-welcome-register"),
    btnWelcomeProxy: document.getElementById("btn-welcome-proxy"),
    btnReloadServices: document.getElementById("btn-reload-services"),
    chkSelectAllServices: document.getElementById("chk-select-all-services"),
    batchSelectedCount: document.getElementById("batch-selected-count"),
    btnBatchStart: document.getElementById("btn-batch-start"),
    btnBatchStop: document.getElementById("btn-batch-stop"),
    btnBatchRestart: document.getElementById("btn-batch-restart"),
    formStart: document.getElementById("form-start"),
    formTitle: document.getElementById("form-title"),
    fieldName: document.getElementById("field-name"),
    startKind: document.getElementById("start-kind"),
    startError: document.getElementById("start-error"),
    btnSaveService: document.getElementById("btn-save-service"),
    btnStartService: document.getElementById("btn-start-service"),
    btnResetForm: document.getElementById("btn-reset-form"),
    logBytes: document.getElementById("log-bytes"),
    logAuto: document.getElementById("log-auto"),
    logInterval: document.getElementById("log-interval"),
    logFollow: document.getElementById("log-follow"),
    btnFetchLogs: document.getElementById("btn-fetch-logs"),
    logTarget: document.getElementById("log-target"),
    logContent: document.getElementById("log-content"),
    toast: document.getElementById("toast"),
  };

  const ONLINE_INTERVAL_MS = 15000;

  const state = {
    rail: "overview",
    me: null,
    agents: [],
    proxies: [],
    upstreams: [],
    selectedId: null,
    selectedProxyId: null,
    editingUpstreamId: null,
    logService: null,
    editingName: null,
    online: { agents: {}, proxies: {} },
    onlineTimer: null,
    onlineProbing: false,
    logTimer: null,
    logFetching: false,
  };

  function getToken() {
    return sessionStorage.getItem(SESSION_KEY) || "";
  }

  function setToken(token) {
    sessionStorage.setItem(SESSION_KEY, token);
  }

  function clearToken() {
    sessionStorage.removeItem(SESSION_KEY);
    sessionStorage.removeItem(USER_KEY);
  }

  function showToast(message, isError = false) {
    els.toast.hidden = false;
    els.toast.textContent = message;
    els.toast.classList.toggle("error", isError);
    clearTimeout(showToast._t);
    const hold = isError && String(message).includes("\n") ? 7000 : 2600;
    showToast._t = setTimeout(() => {
      els.toast.hidden = true;
    }, hold);
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
      { "X-AxleOps-Session": getToken() }
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
      throw new Error(friendlyError("会话无效或未授权"));
    }
    if (!res.ok) {
      const msg = (body && body.message) || `HTTP ${res.status}`;
      throw new Error(friendlyError(msg));
    }
    return body;
  }

  function friendlyError(msg) {
    const s = String(msg || "").trim();
    if (!s) return "操作失败";
    if (/无法连接|请求超时|认证失败|目标不存在|下游不可用|转发请求失败/.test(s)) {
      return s;
    }
    if (/error sending request|connection refused|tcp connect error|dns error|ConnectError/i.test(s)) {
      return `无法连接 Agent/Proxy：检查地址、防火墙与进程是否启动。\n${s}`;
    }
    if (/timeout|timed out/i.test(s)) {
      return `请求超时：目标无响应或网络过慢。\n${s}`;
    }
    if (/401|unauthorized|invalid or missing.*token/i.test(s)) {
      return `认证失败：Token 不正确或权限不足。\n${s}`;
    }
    if (/404|not found/i.test(s)) {
      return `目标不存在：检查名称、路径或上游 id。\n${s}`;
    }
    if (/502|503|504|bad gateway/i.test(s)) {
      return `下游不可用：经 Proxy 时确认上游 Agent 可达。\n${s}`;
    }
    return s;
  }

  function isHttpUrl(value) {
    try {
      const u = new URL(String(value || "").trim());
      return u.protocol === "http:" || u.protocol === "https:";
    } catch {
      return false;
    }
  }

  function markInvalid(el, on) {
    if (!el) return;
    el.classList.toggle("field-invalid", !!on);
  }

  function clearFormInvalid(root) {
    if (!root) return;
    root.querySelectorAll(".field-invalid").forEach((el) => el.classList.remove("field-invalid"));
  }

  function validateServiceBody(body) {
    clearFormInvalid(els.formStart);
    if (!body.name) {
      markInvalid(els.fieldName, true);
      return "请填写服务名称";
    }
    if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(body.name)) {
      markInvalid(els.fieldName, true);
      return "名称需以字母/数字开头，仅含字母数字 . _ -，最长 64";
    }
    if (body.kind === "jar") {
      const jar = els.formStart.elements.namedItem("jar_path");
      if (!body.jar_path) {
        markInvalid(jar, true);
        return "JAR 类型需填写 JAR 路径";
      }
    } else if (body.kind === "script") {
      const sp = els.formStart.elements.namedItem("script_path");
      if (!body.script_path) {
        markInvalid(sp, true);
        return "脚本类型需填写脚本路径";
      }
    } else if (body.kind === "command") {
      const cmd = els.formStart.elements.namedItem("command");
      if (!body.command) {
        markInvalid(cmd, true);
        return "命令类型需填写可执行命令";
      }
    }
    if (body.health_url && !isHttpUrl(body.health_url)) {
      markInvalid(els.formStart.elements.namedItem("health_url"), true);
      return "Health URL 需为 http:// 或 https:// 地址";
    }
    const envText = els.formStart.elements.namedItem("env");
    const badEnv = String((envText && envText.value) || "")
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith("#"))
      .find((line) => !/^[^=]+=[\s\S]*$/.test(line) || line.indexOf("=") === 0);
    if (badEnv) {
      markInvalid(envText, true);
      return `环境变量格式错误（应为 KEY=value）：${badEnv}`;
    }
    return null;
  }

  function validateBaseUrl(value, fieldEl) {
    const v = String(value || "").trim();
    if (!v) {
      markInvalid(fieldEl, true);
      return "请填写 Base URL";
    }
    if (!isHttpUrl(v)) {
      markInvalid(fieldEl, true);
      return "Base URL 需为 http:// 或 https:// 地址";
    }
    return null;
  }

  function showLogin() {
    els.viewLogin.classList.remove("hidden");
    els.viewApp.classList.add("hidden");
  }

  function showApp() {
    els.viewLogin.classList.add("hidden");
    els.viewApp.classList.remove("hidden");
  }

  function hideMainPanels() {
    els.panelWelcome.classList.add("hidden");
    els.panelDetail.classList.add("hidden");
    els.panelRegister.classList.add("hidden");
    els.panelProxyRegister.classList.add("hidden");
    els.panelProxyDetail.classList.add("hidden");
    if (els.panelOverview) els.panelOverview.classList.add("hidden");
    if (els.panelPassword) els.panelPassword.classList.add("hidden");
    if (els.panelUsers) els.panelUsers.classList.add("hidden");
    if (els.panelAudit) els.panelAudit.classList.add("hidden");
  }

  function setRail(rail) {
    state.rail = rail;
    if (els.tabOverview) els.tabOverview.classList.toggle("active", rail === "overview");
    els.tabAgents.classList.toggle("active", rail === "agents");
    els.tabProxies.classList.toggle("active", rail === "proxies");
    if (els.tabSystem) els.tabSystem.classList.toggle("active", rail === "system");
    if (els.railOverview) els.railOverview.classList.toggle("hidden", rail !== "overview");
    els.railAgents.classList.toggle("hidden", rail !== "agents");
    els.railProxies.classList.toggle("hidden", rail !== "proxies");
    if (els.railSystem) els.railSystem.classList.toggle("hidden", rail !== "system");
  }

  function showOverview() {
    state.selectedId = null;
    state.selectedProxyId = null;
    highlightAgent(null);
    highlightProxy(null);
    setRail("overview");
    hideMainPanels();
    if (els.panelOverview) els.panelOverview.classList.remove("hidden");
    loadOverview();
  }

  async function loadOverview() {
    if (!els.overviewList) return;
    els.overviewList.innerHTML = "";
    try {
      const res = await api("/api/v1/overview/services");
      const data = res.data || {};
      const list = Array.isArray(data.services) ? data.services : [];
      const total = data.agents_total || 0;
      const reachable = data.agents_reachable || 0;
      if (els.overviewSummary) {
        els.overviewSummary.textContent = `Agent ${reachable}/${total} 可达 · 服务行 ${list.length}`;
      }
      const rows = list.filter((r) => r.name || r.error);
      if (els.overviewEmpty) els.overviewEmpty.hidden = rows.length > 0;
      rows.forEach((row, i) => {
        const el = document.createElement("div");
        el.className = "service-item";
        el.style.animationDelay = `${i * 30}ms`;
        const stateName = String(row.state || "unknown").toLowerCase();
        const title = row.name
          ? `${row.agent_name} / ${row.name}`
          : `${row.agent_name}（不可达）`;
        const meta = row.error
          ? row.error
          : [row.target || "", row.kind || "", row.pid ? `pid ${row.pid}` : "", row.message || ""]
              .filter(Boolean)
              .join(" · ");
        el.innerHTML = `
          <div class="info">
            <span class="state-pill ${escapeHtml(stateName)}">${escapeHtml(stateName)}</span>
            <strong title="${escapeHtml(title)}">${escapeHtml(title)}</strong>
            <span class="meta" title="${escapeHtml(meta)}">${escapeHtml(meta)}</span>
          </div>
          <div class="row-actions">
            <button type="button" class="btn small" data-act="open">打开</button>
          </div>`;
        el.querySelector("[data-act=open]").addEventListener("click", () => {
          if (row.agent_id) selectAgent(row.agent_id);
        });
        els.overviewList.appendChild(el);
      });
    } catch (e) {
      if (els.overviewSummary) els.overviewSummary.textContent = "加载失败";
      if (els.overviewEmpty) els.overviewEmpty.hidden = false;
      showToast(e.message, true);
    }
  }

  async function rotateAgentToken() {
    if (!state.selectedId) return;
    if (!confirm("轮换该 Agent 在 Admin 中的 Token？\n将尝试同步到远端 Agent（sync）。请保存返回的新 Token。")) {
      return;
    }
    try {
      const res = await api(`/api/v1/agents/${state.selectedId}/rotate-token`, {
        method: "POST",
        body: JSON.stringify({ sync: true }),
      });
      const data = res.data || {};
      const token = data.token || "";
      const msg = [
        data.synced ? "已同步远端 Agent" : "仅更新了 Admin 登记",
        data.message || "",
        token ? `新 Token：\n${token}` : "",
      ]
        .filter(Boolean)
        .join("\n");
      showToast(msg, !data.synced);
      if (token && navigator.clipboard) {
        try {
          await navigator.clipboard.writeText(token);
          showToast("新 Token 已复制到剪贴板");
        } catch (_) {}
      }
      await loadAgents();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function rotateProxyToken() {
    if (!state.selectedProxyId) return;
    if (!confirm("轮换该 Proxy 在 Admin 中的 Token？\n需同步修改 Proxy 的 config.toml 并重启/重载。")) {
      return;
    }
    try {
      const res = await api(`/api/v1/proxies/${state.selectedProxyId}/rotate-token`, {
        method: "POST",
        body: JSON.stringify({ sync: false }),
      });
      const data = res.data || {};
      const token = data.token || "";
      showToast(token ? `新 Token：\n${token}` : "已轮换", true);
      if (token && navigator.clipboard) {
        try {
          await navigator.clipboard.writeText(token);
          showToast("新 Token 已复制到剪贴板");
        } catch (_) {}
      }
      await loadProxies();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  function showWelcome() {
    state.selectedId = null;
    state.selectedProxyId = null;
    state.logService = null;
    stopLogPoll();
    if (els.logAuto) els.logAuto.checked = false;
    hideMainPanels();
    els.panelWelcome.classList.remove("hidden");
    els.pingResult.hidden = true;
    if (els.proxyPingResult) els.proxyPingResult.hidden = true;
    els.logContent.textContent = "";
    els.logTarget.textContent = "选择服务后查看日志";
    els.btnFetchLogs.disabled = true;
    highlightAgent(null);
    highlightProxy(null);
  }

  function showRegister() {
    setRail("agents");
    hideMainPanels();
    els.panelRegister.classList.remove("hidden");
    els.registerError.hidden = true;
  }

  function showProxyRegister() {
    setRail("proxies");
    hideMainPanels();
    els.panelProxyRegister.classList.remove("hidden");
    els.proxyRegisterError.hidden = true;
  }

  function showDetail() {
    hideMainPanels();
    els.panelDetail.classList.remove("hidden");
  }

  function showProxyDetail() {
    hideMainPanels();
    els.panelProxyDetail.classList.remove("hidden");
  }

  function highlightAgent(id) {
    els.agentList.querySelectorAll(".agent-item").forEach((el) => {
      el.classList.toggle("active", el.dataset.id === id);
    });
  }

  function highlightProxy(id) {
    els.proxyList.querySelectorAll(".agent-item").forEach((el) => {
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

  function proxyNameOf(proxyId) {
    const p = state.proxies.find((x) => x.id === proxyId);
    return p ? p.name : null;
  }

  function onlineInfo(kind, id) {
    const map = kind === "proxy" ? state.online.proxies : state.online.agents;
    return map[id] || null;
  }

  function onlineDotHtml(kind, id) {
    const info = onlineInfo(kind, id);
    if (!info) {
      return `<span class="online-dot unknown" title="未探测"></span>`;
    }
    if (info.reachable) {
      return `<span class="online-dot online" title="在线 ${info.latencyMs}ms"></span>`;
    }
    return `<span class="online-dot offline" title="离线"></span>`;
  }

  function setHeroOnline(el, kind, id) {
    if (!el) return;
    const info = onlineInfo(kind, id);
    el.classList.remove("online", "offline", "unknown");
    if (!info) {
      el.classList.add("unknown");
      el.textContent = "未探测";
      return;
    }
    if (info.reachable) {
      el.classList.add("online");
      el.textContent = `在线 · ${info.latencyMs}ms`;
    } else {
      el.classList.add("offline");
      el.textContent = "离线";
    }
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
      const via = agent.proxy_id ? proxyNameOf(agent.proxy_id) : null;
      const tags = (agent.tags || [])
        .slice(0, 3)
        .map((t) => `<span class="tag">${escapeHtml(t)}</span>`)
        .join("");
      btn.innerHTML = `
        <span class="agent-avatar">${escapeHtml(initialOf(agent.name))}</span>
        <span class="agent-body">
          <span class="name-row">${onlineDotHtml("agent", agent.id)}<span class="name" title="${escapeHtml(
        agent.name
      )}">${escapeHtml(agent.name)}</span></span>
          <span class="meta" title="${escapeHtml(agent.base_url)}">${escapeHtml(agent.base_url)}</span>
          ${via ? `<span class="meta via" title="via ${escapeHtml(via)}">via ${escapeHtml(via)}</span>` : ""}
          ${tags ? `<span class="tag-row">${tags}</span>` : ""}
        </span>
      `;
      btn.addEventListener("click", () => selectAgent(agent.id));
      els.agentList.appendChild(btn);
    });
    highlightAgent(state.selectedId);
    if (state.selectedId) setHeroOnline(els.detailOnline, "agent", state.selectedId);
  }

  function renderProxies() {
    els.proxyList.innerHTML = "";
    if (els.proxyCount) els.proxyCount.textContent = String(state.proxies.length);
    els.proxyEmpty.hidden = state.proxies.length > 0;
    state.proxies.forEach((proxy, i) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "agent-item";
      btn.dataset.id = proxy.id;
      btn.style.animationDelay = `${i * 45}ms`;
      btn.innerHTML = `
        <span class="agent-avatar proxy">${escapeHtml(initialOf(proxy.name))}</span>
        <span class="agent-body">
          <span class="name-row">${onlineDotHtml("proxy", proxy.id)}<span class="name" title="${escapeHtml(
        proxy.name
      )}">${escapeHtml(proxy.name)}</span></span>
          <span class="meta" title="${escapeHtml(proxy.base_url)}">${escapeHtml(proxy.base_url)}</span>
          ${
            proxy.notes
              ? `<span class="meta" title="${escapeHtml(proxy.notes)}">${escapeHtml(proxy.notes)}</span>`
              : ""
          }
        </span>
      `;
      btn.addEventListener("click", () => selectProxy(proxy.id));
      els.proxyList.appendChild(btn);
    });
    highlightProxy(state.selectedProxyId);
    if (state.selectedProxyId) setHeroOnline(els.proxyDetailOnline, "proxy", state.selectedProxyId);
  }

  async function probeAgent(id) {
    const t0 = performance.now();
    try {
      const res = await api(`/api/v1/agents/${id}/ping`);
      const data = res.data || {};
      state.online.agents[id] = {
        reachable: !!data.reachable,
        latencyMs: Math.round(performance.now() - t0),
        checkedAt: Date.now(),
      };
      return data;
    } catch (e) {
      state.online.agents[id] = {
        reachable: false,
        latencyMs: Math.round(performance.now() - t0),
        checkedAt: Date.now(),
        error: e.message,
      };
      throw e;
    }
  }

  async function probeProxy(id) {
    const t0 = performance.now();
    try {
      const res = await api(`/api/v1/proxies/${id}/ping`);
      const data = res.data || {};
      state.online.proxies[id] = {
        reachable: !!data.reachable,
        latencyMs: Math.round(performance.now() - t0),
        checkedAt: Date.now(),
      };
      return data;
    } catch (e) {
      state.online.proxies[id] = {
        reachable: false,
        latencyMs: Math.round(performance.now() - t0),
        checkedAt: Date.now(),
        error: e.message,
      };
      throw e;
    }
  }

  async function probeAllOnline() {
    if (!getToken() || state.onlineProbing) return;
    state.onlineProbing = true;
    try {
      const agentIds = state.agents.map((a) => a.id);
      const proxyIds = state.proxies.map((p) => p.id);
      await Promise.all([
        ...agentIds.map((id) => probeAgent(id).catch(() => null)),
        ...proxyIds.map((id) => probeProxy(id).catch(() => null)),
      ]);
      renderAgents();
      renderProxies();
    } finally {
      state.onlineProbing = false;
    }
  }

  function startOnlinePoll() {
    stopOnlinePoll();
    probeAllOnline();
    state.onlineTimer = setInterval(probeAllOnline, ONLINE_INTERVAL_MS);
  }

  function stopOnlinePoll() {
    if (state.onlineTimer) {
      clearInterval(state.onlineTimer);
      state.onlineTimer = null;
    }
  }

  function stopLogPoll() {
    if (state.logTimer) {
      clearInterval(state.logTimer);
      state.logTimer = null;
    }
  }

  function syncLogPoll() {
    stopLogPoll();
    if (!els.logAuto || !els.logAuto.checked) return;
    if (!state.selectedId || !state.logService) return;
    const ms = Number(els.logInterval && els.logInterval.value) || 3000;
    state.logTimer = setInterval(() => {
      fetchLogs({ quiet: true });
    }, ms);
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
      if (!still) {
        if (state.rail === "agents" && !state.selectedProxyId) showWelcome();
      } else if (state.rail === "agents") {
        await openAgentDetail(still);
      }
    }
  }

  async function loadProxies() {
    const res = await api("/api/v1/proxies");
    state.proxies = (res && res.data) || [];
    renderProxies();
    renderAgents();
    if (state.selectedProxyId) {
      const still = state.proxies.find((p) => p.id === state.selectedProxyId);
      if (!still) {
        if (state.rail === "proxies") showWelcome();
      } else if (state.rail === "proxies") {
        await openProxyDetail(still);
      }
    }
  }

  async function selectAgent(id) {
    setRail("agents");
    state.selectedId = id;
    state.selectedProxyId = null;
    highlightAgent(id);
    highlightProxy(null);
    const agent = state.agents.find((a) => a.id === id);
    if (!agent) return;
    await openAgentDetail(agent);
  }

  async function selectProxy(id) {
    setRail("proxies");
    state.selectedProxyId = id;
    state.selectedId = null;
    highlightProxy(id);
    highlightAgent(null);
    const proxy = state.proxies.find((p) => p.id === id);
    if (!proxy) return;
    await openProxyDetail(proxy);
  }

  async function openAgentDetail(agent) {
    showDetail();
    els.detailName.textContent = agent.name;
    els.detailName.title = agent.name || "";
    els.detailUrl.textContent = agent.base_url;
    els.detailUrl.title = agent.base_url || "";
    setHeroOnline(els.detailOnline, "agent", agent.id);
    if (els.detailVia) {
      const via = agent.proxy_id ? proxyNameOf(agent.proxy_id) : null;
      if (via) {
        els.detailVia.hidden = false;
        els.detailVia.textContent = `经 Proxy：${via}`;
        els.detailVia.title = `经 Proxy：${via}`;
      } else {
        els.detailVia.hidden = true;
        els.detailVia.textContent = "";
        els.detailVia.title = "";
      }
    }
    els.pingResult.hidden = true;
    state.logService = null;
    stopLogPoll();
    if (els.logAuto) els.logAuto.checked = false;
    els.logContent.textContent = "";
    els.logTarget.textContent = "选择服务后查看日志";
    els.btnFetchLogs.disabled = true;
    resetServiceForm();
    await loadServices();
  }

  async function openProxyDetail(proxy) {
    showProxyDetail();
    els.proxyDetailName.textContent = proxy.name;
    els.proxyDetailName.title = proxy.name || "";
    els.proxyDetailUrl.textContent = proxy.base_url;
    els.proxyDetailUrl.title = proxy.base_url || "";
    setHeroOnline(els.proxyDetailOnline, "proxy", proxy.id);
    els.proxyDetailNotes.textContent = proxy.notes || "";
    els.proxyDetailNotes.title = proxy.notes || "";
    els.proxyPingResult.hidden = true;
    resetUpstreamForm();
    await loadUpstreams();
  }

  function resetUpstreamForm() {
    state.editingUpstreamId = null;
    if (els.formUpstream) els.formUpstream.reset();
    if (els.upstreamId) {
      els.upstreamId.readOnly = false;
      els.upstreamId.required = true;
    }
    if (els.upstreamToken) {
      els.upstreamToken.required = true;
      els.upstreamToken.placeholder = "Agent 的 X-AxleOps-Token";
    }
    if (els.upstreamFormTitle) els.upstreamFormTitle.textContent = "配置下游 Agent";
    if (els.upstreamFormError) els.upstreamFormError.hidden = true;
  }

  function fillUpstreamForm(up) {
    state.editingUpstreamId = up.id;
    els.upstreamId.value = up.id || "";
    els.upstreamId.readOnly = true;
    els.upstreamName.value = up.name || "";
    els.upstreamBaseUrl.value = up.base_url || "";
    els.upstreamToken.value = "";
    els.upstreamToken.required = false;
    els.upstreamToken.placeholder = "留空则不修改 Token";
    els.upstreamFormTitle.textContent = `编辑 · ${up.id}`;
    els.upstreamFormError.hidden = true;
  }

  function unwrapAgentData(payload) {
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

  async function loadUpstreams() {
    if (!state.selectedProxyId) return;
    els.upstreamList.innerHTML = "";
    state.upstreams = [];
    try {
      const res = await api(`/api/v1/proxies/${state.selectedProxyId}/upstreams`);
      const list = Array.isArray(res.data) ? res.data : [];
      state.upstreams = list;
      if (els.upstreamCount) els.upstreamCount.textContent = String(list.length);
      els.upstreamEmpty.hidden = list.length > 0;
      els.btnImportAll.disabled = list.length === 0;

      const importedUrls = new Set(state.agents.map((a) => a.base_url.replace(/\/$/, "")));

      list.forEach((up, i) => {
        const row = document.createElement("div");
        row.className = "service-item";
        row.style.animationDelay = `${i * 45}ms`;
        const path = up.path_prefix || `/a/${up.id}`;
        const proxy = state.proxies.find((p) => p.id === state.selectedProxyId);
        const fullUrl = proxy
          ? `${proxy.base_url.replace(/\/$/, "")}${path.startsWith("/") ? path : "/" + path}`
          : path;
        const already = importedUrls.has(fullUrl.replace(/\/$/, ""));
        row.innerHTML = `
          <div class="info">
            <strong title="${escapeHtml(up.name || up.id)}">${escapeHtml(up.name || up.id)}</strong>
            <span class="meta" title="${escapeHtml(up.id)}">${escapeHtml(up.id)}</span>
            <span class="meta" title="${escapeHtml(up.base_url || "")}">${escapeHtml(up.base_url || "")}</span>
            <span class="meta" title="${escapeHtml(fullUrl)}">${escapeHtml(fullUrl)}</span>
            ${already ? `<span class="state-pill running">已导入</span>` : ""}
          </div>
          <div class="row-actions">
            <button type="button" class="btn small" data-act="edit">编辑</button>
            <button type="button" class="btn small accent" data-act="import" ${
              already ? "disabled" : ""
            }>导入</button>
            <button type="button" class="btn small danger" data-act="remove">删除</button>
          </div>
        `;
        row.querySelector('[data-act="edit"]').addEventListener("click", () => {
          fillUpstreamForm(up);
          els.formUpstream.scrollIntoView({ behavior: "smooth", block: "nearest" });
        });
        const importBtn = row.querySelector('[data-act="import"]');
        if (importBtn && !already) {
          importBtn.addEventListener("click", async (ev) => {
            setBusy(ev.currentTarget, true, "…");
            try {
              await importUpstream(up.id, up.name);
            } finally {
              setBusy(ev.currentTarget, false);
            }
          });
        }
        row.querySelector('[data-act="remove"]').addEventListener("click", () =>
          deleteUpstream(up.id)
        );
        els.upstreamList.appendChild(row);
      });
    } catch (e) {
      els.upstreamEmpty.hidden = true;
      if (els.upstreamCount) els.upstreamCount.textContent = "0";
      els.btnImportAll.disabled = true;
      showToast(e.message, true);
    }
  }

  async function saveUpstream() {
    if (!state.selectedProxyId) return;
    els.upstreamFormError.hidden = true;
    clearFormInvalid(els.formUpstream);
    const id = String(els.upstreamId.value || "").trim();
    const name = String(els.upstreamName.value || "").trim();
    const base_url = String(els.upstreamBaseUrl.value || "").trim();
    const token = String(els.upstreamToken.value || "").trim();

    try {
      if (state.editingUpstreamId) {
        const urlErr = validateBaseUrl(base_url, els.upstreamBaseUrl);
        if (urlErr) {
          els.upstreamFormError.textContent = urlErr;
          els.upstreamFormError.hidden = false;
          return;
        }
        const body = { name: name || undefined, base_url: base_url || undefined };
        if (token) body.token = token;
        await api(
          `/api/v1/proxies/${state.selectedProxyId}/upstreams/${encodeURIComponent(
            state.editingUpstreamId
          )}`,
          { method: "PUT", body: JSON.stringify(body) }
        );
        showToast("已更新下游");
      } else {
        if (!id) {
          markInvalid(els.upstreamId, true);
          els.upstreamFormError.textContent = "请填写 ID（路径键）";
          els.upstreamFormError.hidden = false;
          return;
        }
        if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(id)) {
          markInvalid(els.upstreamId, true);
          els.upstreamFormError.textContent = "ID 需以字母/数字开头，不可含空格或路径分隔符";
          els.upstreamFormError.hidden = false;
          return;
        }
        const urlErr = validateBaseUrl(base_url, els.upstreamBaseUrl);
        if (urlErr) {
          els.upstreamFormError.textContent = urlErr;
          els.upstreamFormError.hidden = false;
          return;
        }
        if (!token) {
          markInvalid(els.upstreamToken, true);
          els.upstreamFormError.textContent = "新建时必须填写 Agent Token";
          els.upstreamFormError.hidden = false;
          return;
        }
        await api(`/api/v1/proxies/${state.selectedProxyId}/upstreams`, {
          method: "POST",
          body: JSON.stringify({
            id,
            name: name || id,
            base_url,
            token,
          }),
        });
        showToast("已添加下游");
      }
      resetUpstreamForm();
      await loadUpstreams();
    } catch (e) {
      els.upstreamFormError.textContent = e.message;
      els.upstreamFormError.hidden = false;
    }
  }

  async function deleteUpstream(upstreamId) {
    if (!state.selectedProxyId) return;
    if (!confirm(`确认从 Proxy 删除下游「${upstreamId}」？已导入的 Admin Agent 不会自动删除。`)) {
      return;
    }
    try {
      await api(
        `/api/v1/proxies/${state.selectedProxyId}/upstreams/${encodeURIComponent(upstreamId)}`,
        { method: "DELETE" }
      );
      showToast("已删除下游");
      if (state.editingUpstreamId === upstreamId) resetUpstreamForm();
      await loadUpstreams();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function importUpstream(upstreamId, preferredName) {
    if (!state.selectedProxyId) return;
    try {
      const res = await api(`/api/v1/proxies/${state.selectedProxyId}/import`, {
        method: "POST",
        body: JSON.stringify({
          upstream_id: upstreamId,
          agent_name: preferredName || null,
        }),
      });
      showToast(`已导入 ${res.data && res.data.name ? res.data.name : upstreamId}`);
      await loadAgents();
      await loadUpstreams();
    } catch (e) {
      showToast(e.message, true);
    }
  }

  async function importAllUpstreams() {
    if (!state.selectedProxyId || !state.upstreams.length) return;
    let ok = 0;
    let fail = 0;
    for (const up of state.upstreams) {
      try {
        await api(`/api/v1/proxies/${state.selectedProxyId}/import`, {
          method: "POST",
          body: JSON.stringify({ upstream_id: up.id, agent_name: up.name || null }),
        });
        ok += 1;
      } catch {
        fail += 1;
      }
    }
    showToast(`导入完成：成功 ${ok}` + (fail ? `，跳过/失败 ${fail}` : ""));
    await loadAgents();
    await loadUpstreams();
  }

  async function loadServices() {
    if (!state.selectedId) return;
    els.serviceList.innerHTML = "";
    if (els.chkSelectAllServices) els.chkSelectAllServices.checked = false;
    try {
      const res = await api(`/api/v1/agents/${state.selectedId}/services`);
      const data = unwrapAgentData(res);
      const list = Array.isArray(data) ? data : [];
      if (els.serviceCount) els.serviceCount.textContent = String(list.length);
      els.serviceEmpty.hidden = list.length > 0;
      if (els.serviceBatchBar) els.serviceBatchBar.hidden = list.length === 0;
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
        row.innerHTML = `
          <label class="service-check">
            <input type="checkbox" class="svc-check" data-name="${escapeHtml(svc.name || "")}" />
          </label>
          <div class="info">
            <span class="state-pill ${escapeHtml(stateName)}">${escapeHtml(stateName)}</span>
            <strong title="${escapeHtml(svc.name || "")}">${escapeHtml(svc.name || "")}</strong>
            <span class="meta" title="${escapeHtml(target)}">${escapeHtml(target)}</span>
            <span class="meta">${escapeHtml(kind)}${svc.pid ? " · pid " + svc.pid : ""}</span>
            ${
              svc.message
                ? `<span class="meta" title="${escapeHtml(svc.message)}">${escapeHtml(svc.message)}</span>`
                : ""
            }
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
        const check = row.querySelector(".svc-check");
        if (check) {
          check.addEventListener("change", syncBatchBar);
          check.addEventListener("click", (ev) => ev.stopPropagation());
        }
        row.querySelector('[data-act="logs"]').addEventListener("click", () => {
          state.logService = svc.name;
          els.logTarget.textContent = `tail · ${svc.name}`;
          els.btnFetchLogs.disabled = false;
          els.serviceList.querySelectorAll(".service-item").forEach((el) => {
            el.classList.toggle("log-active", el.dataset.name === svc.name);
          });
          if (els.logAuto) els.logAuto.checked = true;
          if (els.logFollow) els.logFollow.checked = true;
          fetchLogs();
          syncLogPoll();
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
      syncBatchBar();
    } catch (e) {
      els.serviceEmpty.hidden = false;
      if (els.serviceBatchBar) els.serviceBatchBar.hidden = true;
      if (els.serviceCount) els.serviceCount.textContent = "0";
      syncBatchBar();
      showToast(e.message, true);
    }
  }

  function selectedServiceNames() {
    return Array.from(els.serviceList.querySelectorAll(".svc-check:checked"))
      .map((el) => el.dataset.name)
      .filter(Boolean);
  }

  function syncBatchBar() {
    const names = selectedServiceNames();
    const total = els.serviceList.querySelectorAll(".svc-check").length;
    if (els.batchSelectedCount) {
      els.batchSelectedCount.textContent = `已选 ${names.length}`;
    }
    const has = names.length > 0;
    if (els.btnBatchStart) els.btnBatchStart.disabled = !has;
    if (els.btnBatchStop) els.btnBatchStop.disabled = !has;
    if (els.btnBatchRestart) els.btnBatchRestart.disabled = !has;
    if (els.chkSelectAllServices) {
      els.chkSelectAllServices.checked = total > 0 && names.length === total;
      els.chkSelectAllServices.indeterminate = names.length > 0 && names.length < total;
    }
  }

  async function batchServiceAction(action) {
    const names = selectedServiceNames();
    if (!names.length || !state.selectedId) return;
    const label =
      action === "start" ? "启动" : action === "stop" ? "停止" : "重启";
    if (!confirm(`确认对选中的 ${names.length} 个服务执行「${label}」？`)) return;

    let ok = 0;
    let fail = 0;
    const errors = [];
    for (const name of names) {
      try {
        const path =
          action === "start"
            ? `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/start`
            : action === "stop"
              ? `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/stop`
              : `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(name)}/restart`;
        await api(path, { method: "POST" });
        ok += 1;
      } catch (e) {
        fail += 1;
        errors.push(`${name}: ${e.message}`);
      }
    }
    showToast(
      `批量${label}完成：成功 ${ok}` + (fail ? `，失败 ${fail}` : ""),
      fail > 0
    );
    if (errors.length) {
      const detail = errors.slice(0, 5).join("\n") + (errors.length > 5 ? `\n…共 ${errors.length} 条` : "");
      showToast(detail, true);
      console.warn(errors.join("\n"));
    }
    await loadServices();
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
    const err = validateServiceBody(body);
    if (err) {
      els.startError.textContent = err;
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
    const err = validateServiceBody(body);
    if (err) {
      els.startError.textContent = err;
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
      const data = await probeAgent(state.selectedId);
      renderAgents();
      els.pingResult.hidden = false;
      els.pingResult.classList.toggle("ok", !!data.reachable);
      els.pingResult.classList.toggle("bad", !data.reachable);
      const info = state.online.agents[state.selectedId];
      els.pingResult.textContent = JSON.stringify(
        { ...data, latency_ms: info && info.latencyMs },
        null,
        2
      );
      showToast(data.reachable ? "Agent 可达" : "Agent 不可达", !data.reachable);
    } catch (e) {
      renderAgents();
      showToast(e.message, true);
    }
  }

  async function pingProxy() {
    if (!state.selectedProxyId) return;
    try {
      const data = await probeProxy(state.selectedProxyId);
      renderProxies();
      els.proxyPingResult.hidden = false;
      els.proxyPingResult.classList.toggle("ok", !!data.reachable);
      els.proxyPingResult.classList.toggle("bad", !data.reachable);
      const info = state.online.proxies[state.selectedProxyId];
      els.proxyPingResult.textContent = JSON.stringify(
        { ...data, latency_ms: info && info.latencyMs },
        null,
        2
      );
      showToast(data.reachable ? "Proxy 可达" : "Proxy 不可达", !data.reachable);
    } catch (e) {
      renderProxies();
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

  async function deleteProxy() {
    if (!state.selectedProxyId) return;
    if (!confirm("确认删除该 Proxy？已导入的 Agent 会保留，仅解除关联。")) return;
    try {
      await api(`/api/v1/proxies/${state.selectedProxyId}`, { method: "DELETE" });
      showToast("已删除 Proxy");
      showWelcome();
      await loadProxies();
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

  async function fetchLogs(opts = {}) {
    const quiet = !!opts.quiet;
    if (!state.selectedId || !state.logService) return;
    if (state.logFetching) return;
    state.logFetching = true;
    const bytes = Number(els.logBytes.value) || 32768;
    try {
      const path = `/api/v1/agents/${state.selectedId}/services/${encodeURIComponent(
        state.logService
      )}/logs?bytes=${bytes}`;
      const res = await api(path);
      const data = unwrapAgentData(res);
      const text = typeof data === "string" ? data : JSON.stringify(data, null, 2);
      els.logContent.textContent = text;
      if (els.logFollow && els.logFollow.checked) {
        els.logContent.scrollTop = els.logContent.scrollHeight;
      }
    } catch (e) {
      if (!quiet) showToast(e.message, true);
    } finally {
      state.logFetching = false;
    }
  }

  function applyMe(me) {
    state.me = me;
    if (els.currentUser) {
      els.currentUser.textContent = me ? `${me.username} · ${me.role}` : "";
      els.currentUser.hidden = !me;
    }
    if (els.btnPassword) {
      els.btnPassword.hidden = !me || me.is_service || !me.user_id;
    }
    if (els.tabSystem) {
      const isAdmin = me && (me.role === "admin" || me.is_service);
      els.tabSystem.classList.toggle("hidden", !isAdmin);
    }
  }

  async function verifySession(token) {
    const res = await fetch("/api/v1/auth/me", {
      headers: { "X-AxleOps-Session": token },
    });
    if (res.status === 401) throw new Error("会话无效或已过期");
    if (!res.ok) {
      const text = await res.text();
      throw new Error(text || `HTTP ${res.status}`);
    }
    const body = await res.json();
    const me = body.data || body;
    applyMe(me);
    return me;
  }

  async function loadUsers() {
    const res = await api("/api/v1/users");
    const list = res.data || [];
    els.userList.innerHTML = "";
    list.forEach((u) => {
      const row = document.createElement("div");
      row.className = "service-item";
      const disabled = !!u.disabled;
      row.innerHTML = `
        <div class="info">
          <strong title="${escapeHtml(u.username)}">${escapeHtml(u.username)}</strong>
          <span class="meta" title="${escapeHtml(u.role)}${disabled ? " · 已禁用" : ""}">${escapeHtml(u.role)}${
        disabled ? " · 已禁用" : ""
      }</span>
        </div>
        <div class="row-actions">
          <button type="button" class="btn small" data-act="toggle">${disabled ? "启用" : "禁用"}</button>
        </div>`;
      row.querySelector("[data-act=toggle]").addEventListener("click", async () => {
        try {
          await api(`/api/v1/users/${encodeURIComponent(u.id)}/disabled`, {
            method: "PUT",
            body: JSON.stringify({ disabled: !disabled }),
          });
          showToast(disabled ? "已启用" : "已禁用");
          await loadUsers();
        } catch (e) {
          showToast(e.message, true);
        }
      });
      els.userList.appendChild(row);
    });
  }

  async function loadAudit() {
    const res = await api("/api/v1/audit-logs?limit=100");
    const list = res.data || [];
    els.auditList.innerHTML = "";
    els.auditEmpty.hidden = list.length > 0;
    list.forEach((a) => {
      const row = document.createElement("div");
      row.className = "service-item";
      const metaLine = `${a.username || ""} · ${a.resource_type || ""} ${a.resource_id || ""} · ${a.detail || ""}`;
      row.innerHTML = `
        <div class="info">
          <strong title="${escapeHtml(a.action || "")}">${escapeHtml(a.action || "")}</strong>
          <span class="meta" title="${escapeHtml(metaLine)}">${escapeHtml(metaLine)}</span>
          <span class="meta" title="${escapeHtml(a.created_at || "")}">${escapeHtml(a.created_at || "")}</span>
        </div>`;
      els.auditList.appendChild(row);
    });
  }

  function showPassword() {
    hideMainPanels();
    els.panelPassword.classList.remove("hidden");
    els.passwordError.hidden = true;
  }

  function showUsers() {
    setRail("system");
    hideMainPanels();
    els.panelUsers.classList.remove("hidden");
    els.usersError.hidden = true;
    loadUsers().catch((e) => showToast(e.message, true));
  }

  function showAudit() {
    setRail("system");
    hideMainPanels();
    els.panelAudit.classList.remove("hidden");
    loadAudit().catch((e) => showToast(e.message, true));
  }

  async function refreshAll() {
    await loadProxies();
    await loadAgents();
  }

  // Events
  if (els.btnToggleToken && els.loginPassword) {
    els.btnToggleToken.addEventListener("click", () => {
      const show = els.loginPassword.type === "password";
      els.loginPassword.type = show ? "text" : "password";
      els.btnToggleToken.textContent = show ? "隐藏" : "显示";
      els.btnToggleToken.setAttribute("aria-pressed", show ? "true" : "false");
    });
  }

  els.formLogin.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    els.loginError.hidden = true;
    const username = (els.loginUsername.value || "").trim();
    const password = els.loginPassword.value || "";
    try {
      const res = await fetch("/api/v1/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      const body = await res.json().catch(() => ({}));
      if (!res.ok || !body.ok) {
        throw new Error((body && body.message) || "登录失败");
      }
      const data = body.data;
      setToken(data.token);
      applyMe({
        user_id: data.user.id,
        username: data.user.username,
        role: data.user.role,
        is_service: false,
      });
      showApp();
      await refreshAll();
      startOnlinePoll();
      showWelcome();
    } catch (e) {
      els.loginError.textContent = e.message;
      els.loginError.hidden = false;
    }
  });

  els.btnLogout.addEventListener("click", async () => {
    try {
      await api("/api/v1/auth/logout", { method: "POST", body: "{}" });
    } catch (_) {
      /* ignore */
    }
    stopOnlinePoll();
    stopLogPoll();
    clearToken();
    applyMe(null);
    showLogin();
  });

  if (els.btnPassword) {
    els.btnPassword.addEventListener("click", showPassword);
  }
  if (els.btnCancelPassword) {
    els.btnCancelPassword.addEventListener("click", showWelcome);
  }
  if (els.formPassword) {
    els.formPassword.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      els.passwordError.hidden = true;
      const fd = new FormData(els.formPassword);
      try {
        await api("/api/v1/auth/change-password", {
          method: "POST",
          body: JSON.stringify({
            old_password: String(fd.get("old_password") || ""),
            new_password: String(fd.get("new_password") || ""),
          }),
        });
        clearToken();
        applyMe(null);
        showLogin();
        showToast("密码已更新，请重新登录");
      } catch (e) {
        els.passwordError.textContent = e.message;
        els.passwordError.hidden = false;
      }
    });
  }
  if (els.tabSystem) {
    els.tabSystem.addEventListener("click", () => {
      setRail("system");
      showUsers();
    });
  }
  if (els.btnShowUsers) els.btnShowUsers.addEventListener("click", showUsers);
  if (els.btnShowAudit) els.btnShowAudit.addEventListener("click", showAudit);
  if (els.btnCancelUsers) els.btnCancelUsers.addEventListener("click", showWelcome);
  if (els.btnCancelAudit) els.btnCancelAudit.addEventListener("click", showWelcome);
  if (els.btnReloadAudit) {
    els.btnReloadAudit.addEventListener("click", () => {
      loadAudit().catch((e) => showToast(e.message, true));
    });
  }
  if (els.formCreateUser) {
    els.formCreateUser.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      els.usersError.hidden = true;
      const fd = new FormData(els.formCreateUser);
      try {
        await api("/api/v1/users", {
          method: "POST",
          body: JSON.stringify({
            username: String(fd.get("username") || "").trim(),
            password: String(fd.get("password") || ""),
            role: String(fd.get("role") || "operator"),
          }),
        });
        els.formCreateUser.reset();
        showToast("用户已创建");
        await loadUsers();
      } catch (e) {
        els.usersError.textContent = e.message;
        els.usersError.hidden = false;
      }
    });
  }

  els.btnRefresh.addEventListener("click", async () => {
    try {
      await refreshAll();
      await probeAllOnline();
      if (state.rail === "overview") await loadOverview();
      showToast("已刷新");
    } catch (e) {
      showToast(e.message, true);
    }
  });

  if (els.tabOverview) {
    els.tabOverview.addEventListener("click", () => showOverview());
  }
  if (els.btnReloadOverview) {
    els.btnReloadOverview.addEventListener("click", () => loadOverview());
  }
  if (els.btnRefreshOverview) {
    els.btnRefreshOverview.addEventListener("click", () => loadOverview());
  }

  els.tabAgents.addEventListener("click", () => {
    setRail("agents");
    if (state.selectedId) {
      const agent = state.agents.find((a) => a.id === state.selectedId);
      if (agent) openAgentDetail(agent);
      else showWelcome();
    } else if (
      !els.panelRegister.classList.contains("hidden") ||
      !els.panelWelcome.classList.contains("hidden")
    ) {
      /* keep */
    } else {
      showWelcome();
    }
  });

  els.tabProxies.addEventListener("click", () => {
    setRail("proxies");
    if (state.selectedProxyId) {
      const proxy = state.proxies.find((p) => p.id === state.selectedProxyId);
      if (proxy) openProxyDetail(proxy);
      else showWelcome();
    } else if (!els.panelProxyRegister.classList.contains("hidden")) {
      /* keep */
    } else {
      showWelcome();
    }
  });

  els.btnShowRegister.addEventListener("click", showRegister);
  els.btnShowProxyRegister.addEventListener("click", showProxyRegister);

  els.btnCancelRegister.addEventListener("click", () => {
    if (state.selectedId) {
      const agent = state.agents.find((a) => a.id === state.selectedId);
      if (agent) openAgentDetail(agent);
      else showWelcome();
    } else {
      showWelcome();
    }
  });

  els.btnCancelProxyRegister.addEventListener("click", () => {
    if (state.selectedProxyId) {
      const proxy = state.proxies.find((p) => p.id === state.selectedProxyId);
      if (proxy) openProxyDetail(proxy);
      else showWelcome();
    } else {
      showWelcome();
    }
  });

  els.formRegister.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    els.registerError.hidden = true;
    clearFormInvalid(els.formRegister);
    const fd = new FormData(els.formRegister);
    const tags = String(fd.get("tags") || "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const name = String(fd.get("name") || "").trim();
    const base_url = String(fd.get("base_url") || "").trim();
    const token = String(fd.get("token") || "").trim();
    const nameEl = els.formRegister.elements.namedItem("name");
    const urlEl = els.formRegister.elements.namedItem("base_url");
    const tokenEl = els.formRegister.elements.namedItem("token");
    if (!name) {
      markInvalid(nameEl, true);
      els.registerError.textContent = "请填写名称";
      els.registerError.hidden = false;
      return;
    }
    const urlErr = validateBaseUrl(base_url, urlEl);
    if (urlErr) {
      els.registerError.textContent = urlErr;
      els.registerError.hidden = false;
      return;
    }
    if (!token) {
      markInvalid(tokenEl, true);
      els.registerError.textContent = "请填写 Token";
      els.registerError.hidden = false;
      return;
    }
    const body = { name, base_url, token, tags };
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

  els.formProxyRegister.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    els.proxyRegisterError.hidden = true;
    clearFormInvalid(els.formProxyRegister);
    const fd = new FormData(els.formProxyRegister);
    const name = String(fd.get("name") || "").trim();
    const base_url = String(fd.get("base_url") || "").trim();
    const token = String(fd.get("token") || "").trim();
    const notes = String(fd.get("notes") || "").trim();
    const nameEl = els.formProxyRegister.elements.namedItem("name");
    const urlEl = els.formProxyRegister.elements.namedItem("base_url");
    const tokenEl = els.formProxyRegister.elements.namedItem("token");
    if (!name) {
      markInvalid(nameEl, true);
      els.proxyRegisterError.textContent = "请填写名称";
      els.proxyRegisterError.hidden = false;
      return;
    }
    const urlErr = validateBaseUrl(base_url, urlEl);
    if (urlErr) {
      els.proxyRegisterError.textContent = urlErr;
      els.proxyRegisterError.hidden = false;
      return;
    }
    if (!token) {
      markInvalid(tokenEl, true);
      els.proxyRegisterError.textContent = "请填写 Proxy Token";
      els.proxyRegisterError.hidden = false;
      return;
    }
    const body = { name, base_url, token, notes };
    try {
      const res = await api("/api/v1/proxies", {
        method: "POST",
        body: JSON.stringify(body),
      });
      showToast("Proxy 已登记");
      els.formProxyRegister.reset();
      await loadProxies();
      if (res.data && res.data.id) {
        await selectProxy(res.data.id);
      }
    } catch (e) {
      els.proxyRegisterError.textContent = e.message;
      els.proxyRegisterError.hidden = false;
    }
  });

  els.btnPing.addEventListener("click", pingAgent);
  els.btnDeleteAgent.addEventListener("click", deleteAgent);
  if (els.btnRotateAgentToken) {
    els.btnRotateAgentToken.addEventListener("click", rotateAgentToken);
  }
  if (els.btnRotateProxyToken) {
    els.btnRotateProxyToken.addEventListener("click", rotateProxyToken);
  }
  els.btnPingProxy.addEventListener("click", pingProxy);
  els.btnDeleteProxy.addEventListener("click", deleteProxy);
  els.btnReloadUpstreams.addEventListener("click", loadUpstreams);
  els.btnImportAll.addEventListener("click", async (ev) => {
    setBusy(ev.currentTarget, true, "导入中…");
    try {
      await importAllUpstreams();
    } finally {
      setBusy(ev.currentTarget, false);
    }
  });
  if (els.formUpstream) {
    els.formUpstream.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      setBusy(els.btnSaveUpstream, true, "保存中…");
      try {
        await saveUpstream();
      } finally {
        setBusy(els.btnSaveUpstream, false);
      }
    });
  }
  if (els.btnResetUpstreamForm) {
    els.btnResetUpstreamForm.addEventListener("click", resetUpstreamForm);
  }
  els.btnReloadServices.addEventListener("click", loadServices);
  if (els.chkSelectAllServices) {
    els.chkSelectAllServices.addEventListener("change", () => {
      const on = els.chkSelectAllServices.checked;
      els.serviceList.querySelectorAll(".svc-check").forEach((el) => {
        el.checked = on;
      });
      syncBatchBar();
    });
  }
  async function runBatch(btn, action) {
    setBusy(btn, true, "…");
    try {
      await batchServiceAction(action);
    } finally {
      setBusy(btn, false);
    }
  }
  if (els.btnBatchStart) {
    els.btnBatchStart.addEventListener("click", (ev) => runBatch(ev.currentTarget, "start"));
  }
  if (els.btnBatchStop) {
    els.btnBatchStop.addEventListener("click", (ev) => runBatch(ev.currentTarget, "stop"));
  }
  if (els.btnBatchRestart) {
    els.btnBatchRestart.addEventListener("click", (ev) => runBatch(ev.currentTarget, "restart"));
  }
  els.btnFetchLogs.addEventListener("click", () => fetchLogs());
  if (els.logAuto) {
    els.logAuto.addEventListener("change", syncLogPoll);
  }
  if (els.logInterval) {
    els.logInterval.addEventListener("change", syncLogPoll);
  }
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
  if (els.btnWelcomeProxy) {
    els.btnWelcomeProxy.addEventListener("click", showProxyRegister);
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
      await verifySession(token);
      showApp();
      await refreshAll();
      startOnlinePoll();
      showOverview();
    } catch {
      clearToken();
      showLogin();
    }
  })();
})();
