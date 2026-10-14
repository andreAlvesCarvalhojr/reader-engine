const shelf = document.getElementById("shelf");
const stage = document.getElementById("stage");
const tocEl = document.getElementById("toc");
const page = document.getElementById("page");
const prev = document.getElementById("prev");
const next = document.getElementById("next");
const progress = document.getElementById("progress");
const subtitle = document.getElementById("subtitle");
const theme = document.getElementById("theme");
const scale = document.getElementById("scale");
const searchInput = document.getElementById("search-input");
const searchBtn = document.getElementById("search-btn");
const searchResults = document.getElementById("search-results");

let current = null;

async function api(path, options) {
  const res = await fetch(path, {
    headers: { "content-type": "application/json", ...(options?.headers || {}) },
    ...options,
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  const type = res.headers.get("content-type") || "";
  if (type.includes("application/json")) return res.json();
  return res.text();
}

function renderShelf(books) {
  shelf.innerHTML = "";
  for (const book of books) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "book";
    btn.dataset.id = book.id;
    btn.innerHTML = `<p class="format">${book.format}</p><h2>${book.title}</h2>`;
    btn.addEventListener("click", () => openBook(book.id));
    shelf.appendChild(btn);
  }
}

function markActiveBook(id) {
  for (const btn of shelf.querySelectorAll(".book")) {
    btn.classList.toggle("active", btn.dataset.id === id);
  }
}

function renderToc(state) {
  tocEl.innerHTML = "";
  for (const entry of state.toc) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = entry.title;
    btn.setAttribute("aria-current", entry.spine_index === state.spine_index ? "true" : "false");
    btn.addEventListener("click", () => command({ type: "go_to", spine_index: entry.spine_index }));
    tocEl.appendChild(btn);
  }
}

async function refreshContent() {
  const html = await api("/api/content");
  page.srcdoc = html;
}

function paintState(state) {
  current = state;
  stage.hidden = false;
  markActiveBook(state.book_id);
  subtitle.textContent = [state.title, state.authors?.join(", ")].filter(Boolean).join(" · ");
  progress.textContent =
    state.spine_len > 0
      ? `${state.spine_index + 1} / ${state.spine_len}`
      : "sem páginas";
  prev.disabled = !state.spine_len || state.spine_index <= 0;
  next.disabled = !state.spine_len || state.spine_index >= state.spine_len - 1;
  theme.value = state.theme || "material3";
  scale.value = String(state.font_scale ?? 1);
  renderToc(state);
}

async function openBook(id) {
  const state = await api("/api/open", {
    method: "POST",
    body: JSON.stringify({ id }),
  });
  paintState(state);
  await refreshContent();
}

async function command(body) {
  const state = await api("/api/command", {
    method: "POST",
    body: JSON.stringify(body),
  });
  paintState(state);
  await refreshContent();
}

prev.addEventListener("click", () => command({ type: "prev" }));
next.addEventListener("click", () => command({ type: "next" }));
theme.addEventListener("change", () => command({ type: "set_theme", theme: theme.value }));
scale.addEventListener("change", () =>
  command({ type: "set_font_scale", scale: Number(scale.value) }),
);

document.addEventListener("keydown", (event) => {
  if (event.key === "ArrowRight") command({ type: "next" });
  if (event.key === "ArrowLeft") command({ type: "prev" });
});

async function performSearch() {
  const query = searchInput.value.trim();
  if (!query) {
    searchResults.innerHTML = "";
    return;
  }
  try {
    const results = await api(`/api/search?q=${encodeURIComponent(query)}`);
    renderSearchResults(results);
  } catch (e) {
    searchResults.innerHTML = `<li><span style="color: var(--danger-mute);">Erro: ${e.message}</span></li>`;
  }
}

function renderSearchResults(results) {
  searchResults.innerHTML = "";
  if (results.length === 0) {
    searchResults.innerHTML = '<li><span style="color: var(--danger-mute);">Nenhum resultado</span></li>';
    return;
  }
  for (const result of results) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    const chapterName = current?.toc?.find(t => t.spine_index === result.spine_index)?.title || `Página ${result.spine_index + 1}`;
    btn.innerHTML = `<span class="chapter">${chapterName}</span><span class="snippet">${escapeHtml(result.snippet)}</span>`;
    btn.addEventListener("click", () => goToSearchResult(result));
    li.appendChild(btn);
    searchResults.appendChild(li);
  }
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

async function goToSearchResult(result) {
  await command({ type: "go_to", spine_index: result.spine_index });
}

searchBtn.addEventListener("click", performSearch);
searchInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") performSearch();
});

const books = await api("/api/books");
renderShelf(books);
await openBook(books[0].id);
