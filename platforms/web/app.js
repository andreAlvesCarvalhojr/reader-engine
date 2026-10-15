import init, { WasmReaderEngine } from "./pkg/reader_wasm.js";

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
const bookmarkToggle = document.getElementById("bookmark-toggle");
const bookmarksBar = document.getElementById("bookmarks-bar");
const bookmarksList = document.getElementById("bookmarks-list");
const searchInput = document.getElementById("search-input");
const searchBtn = document.getElementById("search-btn");
const searchResults = document.getElementById("search-results");
const fileInput = document.getElementById("file-input");
const bootError = document.getElementById("boot-error");

const SAMPLES = [
  { id: "noite-clara", title: "Noite Clara", format: "JSON", url: "./samples/noite-clara.json", name: "noite-clara.json" },
  { id: "sample-epub", title: "EPUB Fixture", format: "EPUB", url: "./samples/sample.epub", name: "sample.epub" },
];

const engine = new WasmReaderEngine();
let session = null;
let current = null;
let activeId = null;

function showBootError(message) {
  bootError.hidden = false;
  bootError.textContent = message;
}

function formatProgress(value) {
  return `${Math.round(value * 100)}%`;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

function renderShelf() {
  shelf.innerHTML = "";
  for (const book of SAMPLES) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "book";
    btn.dataset.id = book.id;
    btn.innerHTML = `<p class="format">${book.format}</p><h2>${book.title}</h2>`;
    btn.addEventListener("click", () => openSample(book));
    shelf.appendChild(btn);
  }
  const openBtn = document.createElement("button");
  openBtn.type = "button";
  openBtn.className = "book open-file";
  openBtn.innerHTML = `<p class="format">LOCAL</p><h2>Abrir arquivo</h2>`;
  openBtn.addEventListener("click", () => fileInput.click());
  shelf.appendChild(openBtn);
}

function markActiveBook(id) {
  activeId = id;
  for (const btn of shelf.querySelectorAll(".book")) {
    btn.classList.toggle("active", btn.dataset.id === id);
  }
}

function renderToc(state) {
  tocEl.innerHTML = "";
  for (const entry of state.toc || []) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = entry.title;
    btn.setAttribute("aria-current", entry.spine_index === state.spine_index ? "true" : "false");
    btn.addEventListener("click", () => {
      paintState(session.goTo(entry.spine_index));
      refreshContent();
    });
    tocEl.appendChild(btn);
  }
}

function renderBookmarks(state) {
  bookmarksList.innerHTML = "";
  const bookmarks = state.bookmarks || [];
  if (bookmarks.length === 0) {
    bookmarksBar.hidden = true;
    return;
  }
  bookmarksBar.hidden = false;
  for (const bm of bookmarks) {
    const chip = document.createElement("div");
    chip.className = "bookmark-chip";
    if (bm.spine_index === state.spine_index) chip.classList.add("current");
    chip.innerHTML = `
      <span class="bookmark-label">${escapeHtml(bm.label || `Página ${bm.spine_index + 1}`)}</span>
      <button type="button" class="remove-btn" title="Remover">×</button>
    `;
    chip.querySelector(".bookmark-label").addEventListener("click", () => {
      paintState(session.goToBookmark(bm.id));
      refreshContent();
    });
    chip.querySelector(".remove-btn").addEventListener("click", (e) => {
      e.stopPropagation();
      session.removeBookmark(bm.id);
      paintState(session.snapshot());
    });
    bookmarksList.appendChild(chip);
  }
}

function paintState(state) {
  current = state;
  stage.hidden = false;
  if (activeId) markActiveBook(activeId);
  subtitle.textContent = [state.title, state.authors?.join(", ")].filter(Boolean).join(" · ");

  const chapterInfo = state.spine_len > 0
    ? `${state.spine_index + 1} / ${state.spine_len}`
    : "sem páginas";
  progress.innerHTML =
    `<span class="chapter-progress">${chapterInfo}</span> · ` +
    `<span class="book-progress">${formatProgress(state.book_progress ?? 0)}</span>`;

  prev.disabled = !state.spine_len || state.spine_index <= 0;
  next.disabled = !state.spine_len || state.spine_index >= state.spine_len - 1;
  bookmarkToggle.disabled = !state.spine_len;
  bookmarkToggle.textContent = state.has_bookmark_at_current ? "★" : "☆";
  bookmarkToggle.classList.toggle("active", !!state.has_bookmark_at_current);
  theme.value = state.theme || "material3";
  scale.value = String(state.font_scale ?? 1);
  renderToc(state);
  renderBookmarks(state);
}

function refreshContent() {
  if (!session) return;
  page.srcdoc = session.currentHtml();
}

async function openBytes(name, bytes, sampleId) {
  session = engine.open(name, bytes);
  markActiveBook(sampleId);
  paintState(session.snapshot());
  refreshContent();
  try {
    const key = `reader-bookmarks:${sampleId || name}`;
    const saved = localStorage.getItem(key);
    if (saved) {
      paintState(session.importBookmarksJson(saved));
    }
  } catch {
    // ignore corrupt local storage
  }
}

async function openSample(book) {
  const res = await fetch(book.url);
  if (!res.ok) throw new Error(`Falha ao carregar ${book.url}`);
  const buffer = await res.arrayBuffer();
  await openBytes(book.name, new Uint8Array(buffer), book.id);
}

fileInput.addEventListener("change", async () => {
  const file = fileInput.files?.[0];
  if (!file) return;
  const buffer = await file.arrayBuffer();
  await openBytes(file.name, new Uint8Array(buffer), null);
  fileInput.value = "";
});

prev.addEventListener("click", () => {
  paintState(session.prev());
  refreshContent();
});
next.addEventListener("click", () => {
  paintState(session.next());
  refreshContent();
});
bookmarkToggle.addEventListener("click", () => {
  session.toggleBookmark(null);
  paintState(session.snapshot());
  if (activeId || current?.book_id) {
    const key = `reader-bookmarks:${activeId || current.book_id}`;
    localStorage.setItem(key, session.exportBookmarksJson());
  }
});
theme.addEventListener("change", () => {
  paintState(session.setTheme(theme.value));
  refreshContent();
});
scale.addEventListener("change", () => {
  paintState(session.setFontScale(Number(scale.value)));
  refreshContent();
});

document.addEventListener("keydown", (event) => {
  if (!session) return;
  if (event.key === "ArrowRight") {
    paintState(session.next());
    refreshContent();
  }
  if (event.key === "ArrowLeft") {
    paintState(session.prev());
    refreshContent();
  }
  if (event.key === "b" || event.key === "B") bookmarkToggle.click();
});

window.addEventListener("message", (event) => {
  if (!session || !event.data || event.data.type !== "reader-progress") return;
  paintState(session.reportProgression(event.data.progression));
});

async function performSearch() {
  const query = searchInput.value.trim();
  if (!query || !session) {
    searchResults.innerHTML = "";
    return;
  }
  const results = session.search(query);
  searchResults.innerHTML = "";
  if (!results.length) {
    searchResults.innerHTML = '<li><span style="color: var(--danger-mute);">Nenhum resultado</span></li>';
    return;
  }
  for (const result of results) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    const chapterName =
      current?.toc?.find((t) => t.spine_index === result.spine_index)?.title ||
      `Página ${result.spine_index + 1}`;
    btn.innerHTML =
      `<span class="chapter">${escapeHtml(chapterName)}</span>` +
      `<span class="snippet">${escapeHtml(result.snippet)}</span>`;
    btn.addEventListener("click", () => {
      paintState(session.goToWithProgression(result.spine_index, result.progression));
      refreshContent();
    });
    li.appendChild(btn);
    searchResults.appendChild(li);
  }
}

searchBtn.addEventListener("click", performSearch);
searchInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") performSearch();
});

try {
  await init();
  renderShelf();
  await openSample(SAMPLES[0]);
} catch (err) {
  console.error(err);
  showBootError(
    "Falha ao carregar o WASM. Rode `./scripts/build-web.sh` e sirva platforms/web por HTTP (não file://).",
  );
}
