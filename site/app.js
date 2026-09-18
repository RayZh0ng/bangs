// The page shows the app rather than describing it: the notch opens and closes
// on its own, the lyric fills in word by word, and the download buttons point
// at whatever the newest GitHub release actually contains.

const REPO = "gxlself/bangs";
const RELEASES = `https://github.com/${REPO}/releases`;

/* ---------- The notch demo ---------- */

const notch = document.querySelector("[data-notch]");
const compactLyric = document.querySelector("[data-lyric]");
const panelLyric = document.querySelector("[data-panel-lyric]");
const progress = document.querySelector("[data-progress]");

const LINE = "我曾将青春翻涌成她";
const COMPACT_LINE = "也曾指尖弹出盛夏";

/** Fills a line in from the left, one character at a time. */
function karaoke(element, text, sung) {
  const cut = Math.max(0, Math.min(text.length, Math.round(sung * text.length)));
  element.innerHTML = `<b>${text.slice(0, cut)}</b><span>${text.slice(cut)}</span>`;
}

let started = performance.now();
function frame(now) {
  const beat = ((now - started) / 4200) % 1;
  if (compactLyric) karaoke(compactLyric, COMPACT_LINE, beat);
  if (panelLyric) karaoke(panelLyric, LINE, beat);
  if (progress) progress.style.width = `${18 + beat * 52}%`;
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

// Open, hold, close — and let the pointer take over when it arrives.
let auto = true;
let phase = 0;
setInterval(() => {
  if (!auto || !notch) return;
  phase = (phase + 1) % 2;
  notch.classList.toggle("is-open", phase === 1);
}, 3600);

if (notch) {
  const screen = notch.closest(".screen__body");
  screen?.addEventListener("pointerenter", () => {
    auto = false;
    notch.classList.add("is-open");
  });
  screen?.addEventListener("pointerleave", () => {
    auto = true;
    phase = 0;
    notch.classList.remove("is-open");
  });
}

/* ---------- Feature card karaoke ---------- */

const cardLine = document.querySelector("[data-karaoke] span");
if (cardLine) {
  const text = cardLine.textContent.trim();
  const holder = cardLine.parentElement;
  const tick = (now) => {
    karaoke(holder, text, ((now / 3400) % 1));
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

/* ---------- Reveal on scroll ---------- */

const observer = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      }
    }
  },
  { rootMargin: "-40px" },
);
document.querySelectorAll(".card, .release, .code, .section__title").forEach((node) => observer.observe(node));

/* ---------- The latest release ---------- */

// In preference order: the friendly installer first, the alternative second.
const MAC = [/\.dmg$/i, /\.app\.zip$/i];
const WINDOWS = [/setup\.exe$/i, /\.msi$/i];

function bytes(size) {
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

function applyAsset(patterns, links, meta, suffix) {
  return (release) => {
    const asset = patterns
      .map((pattern) => release.assets.find((item) => pattern.test(item.name)))
      .find(Boolean);
    for (const link of links) {
      link.href = asset ? asset.browser_download_url : `${RELEASES}/latest`;
    }
    if (meta) {
      meta.textContent = asset ? `${suffix} · ${bytes(asset.size)}` : `${suffix} · 见 Releases`;
    }
  };
}

async function loadRelease() {
  const text = document.querySelector("[data-version-text]");
  try {
    const response = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) throw new Error(String(response.status));
    const release = await response.json();
    const version = (release.tag_name || "").replace(/^v/, "");

    if (text) text.textContent = `最新版本 v${version} · ${new Date(release.published_at).toLocaleDateString("zh-CN")}`;

    applyAsset(
      MAC,
      [document.querySelector("[data-download-mac]"), document.querySelector("[data-mac-link]")].filter(Boolean),
      document.querySelector("[data-mac-meta]"),
      "Apple 芯片 · macOS 12+",
    )(release);

    applyAsset(
      WINDOWS,
      [document.querySelector("[data-download-win]"), document.querySelector("[data-win-link]")].filter(Boolean),
      document.querySelector("[data-win-meta]"),
      "Windows 10 / 11 · 需要 WebView2",
    )(release);

    const note = document.querySelector("[data-download-note]");
    if (note) note.textContent = `${release.assets.length} 个安装包 · 两个平台同一套代码`;
  } catch {
    // No releases yet, or GitHub is rate limiting: the plain links still work.
    if (text) text.textContent = "从 GitHub Releases 下载";
    document
      .querySelectorAll("[data-download-mac], [data-download-win], [data-mac-link], [data-win-link]")
      .forEach((link) => {
        link.href = `${RELEASES}/latest`;
      });
  }
}

loadRelease();
