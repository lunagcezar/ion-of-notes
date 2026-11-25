// Minimal browser client for the ion-of-notes v2 API (spec-153).
// No framework, no external origin, no inline script.
(() => {
  "use strict";

  const VERSION = "2026-11-08";
  const $ = (id) => document.getElementById(id);
  const state = { token: sessionStorage.getItem("ion-token") || "", page: null };
  let searchTimer = null;
  let searchController = null;
  let searchSeq = 0;

  function baseUrl() {
    const entered = $("server").value.trim().replace(/\/+$/, "");
    return entered || location.origin;
  }

  function status(text, target = "page-status") {
    $(target).textContent = text || "";
  }

  async function api(path, options = {}) {
    const headers = Object.assign(
      { "ion-of-notes-version": VERSION },
      options.body ? { "content-type": "application/json" } : {},
      state.token ? { authorization: `Bearer ${state.token}` } : {},
    );
    const response = await fetch(`${baseUrl()}${path}`, { ...options, headers });
    const text = await response.text();
    let body = null;
    try { body = text ? JSON.parse(text) : null; } catch { body = null; }
    if (!response.ok) {
      const message = (body && (body.error || (body.data && body.data.message))) || text ||
        `request failed (${response.status})`;
      throw new Error(message);
    }
    return body && Object.hasOwn(body, "data") ? body.data : body;
  }

  function showAuth() {
    $("auth").classList.remove("hidden");
    $("workspace").classList.add("hidden");
    $("logout").classList.add("hidden");
  }

  function showWorkspace() {
    $("auth").classList.add("hidden");
    $("workspace").classList.remove("hidden");
    $("logout").classList.remove("hidden");
  }

  async function signIn() {
    const email = $("email").value.trim();
    const password = $("password").value;
    if (!email || !password) return status("Email and password are required.", "auth-status");
    const register = $("register-mode").checked;
    try {
      const session = await api(register ? "/api/v2/auth/register" : "/api/v2/auth/login", {
        method: "POST",
        body: JSON.stringify(
          register
            ? { email, password, invite_token: $("invite-token").value.trim() }
            : { email, password },
        ),
      });
      state.token = session.token;
      sessionStorage.setItem("ion-token", state.token);
      $("password").value = "";
      status(register ? "Account created." : "Signed in.", "auth-status");
      showWorkspace();
      await loadPages();
    } catch (error) {
      sessionStorage.removeItem("ion-token");
      state.token = "";
      status(error.message, "auth-status");
    }
  }

  async function signOut() {
    if (state.token) {
      try { await api("/api/v2/auth/logout", { method: "POST" }); } catch { /* best effort */ }
    }
    sessionStorage.removeItem("ion-token");
    state.token = "";
    state.page = null;
    showAuth();
  }

  async function loadPages() {
    try {
      const pages = [];
      let cursor = null;
      do {
        const suffix = cursor ? `&start_cursor=${encodeURIComponent(cursor)}` : "";
        const list = await api(`/api/v2/pages?page_size=100${suffix}`);
        pages.push(...(list.results || []));
        cursor = list.next_cursor;
      } while (cursor && pages.length < 2000);
      const nav = $("pages");
      nav.textContent = "";
      for (const page of pages) {
        const link = document.createElement("a");
        link.className = "page-link";
        link.href = "#";
        link.textContent = page.title || "Untitled";
        link.addEventListener("click", (event) => {
          event.preventDefault();
          openPage(page.id);
        });
        nav.appendChild(link);
      }
      if (!state.page && pages.length) await openPage(pages[0].id);
    } catch (error) {
      status(error.message);
    }
  }

  async function openPage(id) {
    try {
      const page = await api(`/api/v2/pages/${encodeURIComponent(id)}`);
      state.page = page;
      $("page-title").textContent = page.title || "Untitled";
      for (const link of document.querySelectorAll(".page-link")) {
        link.classList.toggle("active", link.textContent === (page.title || "Untitled"));
      }
      renderBlocks(page);
    } catch (error) {
      status(error.message);
    }
  }

  function renderBlocks(page) {
    const host = $("blocks");
    host.textContent = "";
    const blocks = page.blocks || [];
    if (!blocks.length) {
      host.textContent = "This page has no blocks yet.";
      return;
    }
    renderBlockTree(host, blocks, 0);
  }

  function renderBlockTree(host, blocks, depth) {
    for (const block of blocks) {
      renderInto(host, block, depth);
      if (block.children && block.children.length) {
        renderBlockTree(host, block.children, depth + 1);
      }
    }
  }

  function renderInto(host, block, depth) {
    const row = document.createElement("div");
    row.className = "block";
    if (depth > 0) row.style.marginLeft = `${depth * 18}px`;
    const kind = document.createElement("div");
    kind.className = "kind";
    kind.textContent = (block.kind || "").replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
    const area = document.createElement("textarea");
    area.value = block.text || "";
    area.rows = Math.max(1, Math.ceil((block.text || "").length / 70));
    area.addEventListener("blur", () => saveBlock(block, area.value));
    row.append(kind, area);
    host.appendChild(row);
  }

  async function saveBlock(block, text) {
    if (text === (block.text || "")) return;
    try {
      await api(`/api/v2/blocks/${encodeURIComponent(block.id)}`, {
        method: "PATCH",
        body: JSON.stringify({ text }),
      });
      block.text = text;
      status("Saved.");
    } catch (error) {
      status(error.message);
    }
  }

  async function createPage() {
    const title = $("new-title").value.trim();
    if (!title) return;
    try {
      const page = await api("/api/v2/pages", {
        method: "POST",
        body: JSON.stringify({ title, icon: "", parent: null }),
      });
      $("new-title").value = "";
      await loadPages();
      await openPage(page.id);
    } catch (error) {
      status(error.message);
    }
  }

  function search() {
    const query = $("search").value.trim();
    clearTimeout(searchTimer);
    if (!query) {
      $("search-results").textContent = "";
      return;
    }
    searchTimer = setTimeout(() => runSearch(query, ++searchSeq), 300);
  }

  async function runSearch(query, sequence) {
    if (searchController) searchController.abort();
    const controller = new AbortController();
    searchController = controller;
    const host = $("search-results");
    try {
      const list = await api(
        `/api/v2/search?q=${encodeURIComponent(query)}&page_size=20`,
        { signal: controller.signal },
      );
      if (sequence !== searchSeq) return;
      host.textContent = "";
      for (const result of list.results || []) {
        const link = document.createElement("a");
        link.className = "page-link";
        link.href = "#";
        link.textContent = result.page_title || "Untitled";
        link.addEventListener("click", (event) => {
          event.preventDefault();
          openPage(result.page_id);
        });
        host.appendChild(link);
      }
      if (!(list.results || []).length) host.textContent = "No matches.";
    } catch (error) {
      if (error.name === "AbortError") return;
      status(error.message);
    }
  }

  async function changePassword() {
    const current = $("current-password").value;
    const next = $("new-password").value;
    if (next !== $("confirm-password").value) {
      return status("The new passwords do not match.", "account-status");
    }
    try {
      const session = await api("/api/v2/auth/password", {
        method: "POST",
        body: JSON.stringify({ current_password: current, new_password: next }),
      });
      state.token = session.token;
      sessionStorage.setItem("ion-token", state.token);
      $("current-password").value = "";
      $("new-password").value = "";
      $("confirm-password").value = "";
      status("Password changed; other sessions were signed out.", "account-status");
    } catch (error) {
      status(error.message, "account-status");
    }
  }

  async function resetMember() {
    try {
      await api("/api/v2/auth/reset", {
        method: "POST",
        body: JSON.stringify({
          email: $("reset-email").value.trim(),
          new_password: $("reset-password").value,
        }),
      });
      $("reset-password").value = "";
      status("Member password reset.", "account-status");
    } catch (error) {
      status(error.message, "account-status");
    }
  }

  $("server").value = location.origin;
  $("register-mode").addEventListener("change", () => {
    $("invite-row").classList.toggle("hidden", !$("register-mode").checked);
  });
  $("login").addEventListener("click", signIn);
  $("logout").addEventListener("click", signOut);
  $("create").addEventListener("click", createPage);
  $("search").addEventListener("input", search);
  $("change-password").addEventListener("click", changePassword);
  $("reset-member").addEventListener("click", resetMember);

  if (state.token) {
    showWorkspace();
    loadPages().catch(() => showAuth());
  } else {
    showAuth();
  }
})();
