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

let current = null;
let scrollTimeout = null;
let lastReportedProgression = 0;

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

function getScrollProgression() {
  try {
    const doc = page.contentDocument;
    if (!doc || !doc.body) return 0;
    
    const scrollTop = doc.documentElement.scrollTop || doc.body.scrollTop || 0;
    const scrollHeight = doc.documentElement.scrollHeight || doc.body.scrollHeight || 1;
    const clientHeight = doc.documentElement.clientHeight || doc.body.clientHeight || 0;
    
    const maxScroll = Math.max(0, scrollHeight - clientHeight);
    if (maxScroll === 0) return 0;
    
    return Math.min(1, Math.max(0, scrollTop / maxScroll));
  } catch (e) {
    return 0;
  }
}

async function reportProgression(progression) {
  if (!current) return;
  const diff = Math.abs(progression - lastReportedProgression);
  if (diff < 0.01) return;
  
  lastReportedProgression = progression;
  const state = await api("/api/command", {
    method: "POST",
    body: JSON.stringify({ type: "report_progression", progression }),
  });
  paintState(state);
}

function handleIframeScroll() {
  if (scrollTimeout) clearTimeout(scrollTimeout);
  scrollTimeout = setTimeout(() => {
    const progression = getScrollProgression();
    reportProgression(progression);
  }, 150);
}

function setupScrollListener() {
  try {
    const doc = page.contentDocument;
    if (doc) {
      doc.removeEventListener("scroll", handleIframeScroll);
      doc.addEventListener("scroll", handleIframeScroll, { passive: true });
    }
  } catch (e) {
    // Cross-origin or not ready
  }
}

page.addEventListener("load", () => {
  setupScrollListener();
  if (current && current.chapter_progress > 0) {
    restoreScrollPosition(current.chapter_progress);
  }
});

function restoreScrollPosition(progression) {
  try {
    const doc = page.contentDocument;
    if (!doc || !doc.body) return;
    
    const scrollHeight = doc.documentElement.scrollHeight || doc.body.scrollHeight || 0;
    const clientHeight = doc.documentElement.clientHeight || doc.body.clientHeight || 0;
    const maxScroll = Math.max(0, scrollHeight - clientHeight);
    
    const scrollTop = maxScroll * progression;
    doc.documentElement.scrollTop = scrollTop;
    doc.body.scrollTop = scrollTop;
  } catch (e) {
    // Cross-origin or not ready
  }
}

function formatProgress(value) {
  return `${Math.round(value * 100)}%`;
}

function paintState(state) {
  current = state;
  stage.hidden = false;
  markActiveBook(state.book_id);
  subtitle.textContent = [state.title, state.authors?.join(", ")].filter(Boolean).join(" · ");
  
  const chapterInfo = state.spine_len > 0
    ? `${state.spine_index + 1} / ${state.spine_len}`
    : "sem páginas";
  const chapterProg = formatProgress(state.chapter_progress ?? 0);
  const bookProg = formatProgress(state.book_progress ?? 0);
  progress.innerHTML = `<span class="chapter-progress">${chapterInfo}</span> · <span class="book-progress">📖 ${bookProg}</span>`;
  
  prev.disabled = !state.spine_len || state.spine_index <= 0;
  next.disabled = !state.spine_len || state.spine_index >= state.spine_len - 1;
  theme.value = state.theme || "material3";
  scale.value = String(state.font_scale ?? 1);
  lastReportedProgression = state.chapter_progress ?? 0;
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

const books = await api("/api/books");
renderShelf(books);
await openBook(books[0].id);
