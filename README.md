<div align="center">

# PDFMaker

### HTML &amp; CSS → PDF, powered by its own native rendering engine

**No headless browser. No Chromium. No WebView.**
PDFMaker parses HTML, computes the CSS cascade, lays out the page, and draws every glyph,
gradient, and shape with its own engine written in **Rust** — so it's small, fast, fully
self‑contained, and produces identical output everywhere it runs.

[**🌐 Website**](https://pdfmaker.ink) &nbsp;·&nbsp;
[**📖 Documentation**](https://pdfmaker.ink/documentation.html) &nbsp;·&nbsp;
[**🚀 Try it in your browser**](https://pdfmaker.ink/web/index.html) &nbsp;·&nbsp;
[**⬇ Download**](https://pdfmaker.ink/download.html)

</div>

---

## ✨ Why PDFMaker

Most HTML‑to‑PDF tools shell out to a headless browser (Chromium, Puppeteer, wkhtmltopdf).
PDFMaker is different — it's a **real, standalone rendering engine**:

- 🦀 **Own engine, pure Rust** — no browser, no Node.js, no native dependencies.
- ⚡ **Fast & lightweight** — a single binary; deterministic, pixel‑consistent output.
- 🌍 **Runs everywhere** — Windows desktop, Android, and right in the browser via WebAssembly.
- 🔒 **100% offline** — your documents never leave your machine.

## 🧩 Features

| Category | What's supported |
|---|---|
| **Layout** | Block / inline / inline‑block, **Flexbox**, **CSS Grid**, **multi‑column**, `position` (relative/absolute/fixed), `float`, `overflow` clipping |
| **Typography** | Web‑safe + **`@font-face` web fonts** (local & remote), `font-weight/style/stretch/variant`, `letter/word‑spacing`, `text-align: justify`, `text-shadow`, `text-decoration`, drop caps (`::first-letter`) |
| **Color & Backgrounds** | Named/hex/`rgb()`/`rgba()`, **linear / radial / conic gradients**, multi‑layer backgrounds, `background-clip: text` (gradient text) |
| **Borders & Effects** | `border-radius`, per‑side borders, **`box-shadow`**, `opacity`, **`mix-blend-mode`**, **`clip-path`**, **`filter`** (blur, grayscale, sepia…), **`backdrop-filter` (real frosted glass)** |
| **Transforms** | `translate`, `rotate`, `scale`, `skew`, `matrix`, `transform-origin` |
| **Tables** | `border-collapse`, `border-spacing`, `colspan`/`rowspan`, striped rows, rounded clipping |
| **Lists & Content** | Many `list-style-type`s, **CSS counters** & generated content (`::before`) |
| **Graphics** | Inline **SVG** (paths, gradients, shapes), raster images (PNG/JPEG/GIF/BMP/WebP), full‑color **emoji** |
| **Internationalization** | **CJK** (中文 · 日本語 · 한국어), **Arabic / Hebrew (RTL)** with shaping & bidi, Cyrillic, vertical `writing-mode` |
| **Paged media** | `@page` sizes (A4, Letter, custom…), margins, automatic & forced page breaks, `@media print` |
| **PDF features** | Embedded subsetted fonts, clickable link annotations, password **encryption**, and tools to **merge / extract / delete / compress** PDFs |

➡️ See the full, illustrated property catalogue (each feature rendered straight to PDF) at
**[pdfmaker.ink/documentation.html](https://pdfmaker.ink/documentation.html)**.

## 🖼 Showcase

A few documents rendered entirely by PDFMaker — straight from HTML/CSS, no browser involved.
Sources live in Examples.

<table>
  <tr>
    <td width="50%" align="center">
    <img width="957" height="1349" alt="Example16" src="https://github.com/user-attachments/assets/0055fdc5-6c5e-4fad-82e2-68ee95c528e7" />
      <br><b>Analytics Dashboard</b><br><sub>Dark mode · gradients · KPI cards</sub>
    </td>
    <td width="50%" align="center">
      <img width="959" height="1351" alt="Example9" src="https://github.com/user-attachments/assets/a764ea15-fc8d-47ef-9a4b-f2e5e3795e73" />
      <br><b>Professional Invoice</b><br><sub>Clean tables · business layout</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <img width="958" height="866" alt="Example25" src="https://github.com/user-attachments/assets/02916462-edf5-4e53-9f9f-ea951b43e8df" />
      <br><b>Creative Vision</b><br><sub>Bold type · vivid gradients</sub>
    </td>
    <td width="50%" align="center">
      <img width="1110" height="944" alt="Example28" src="https://github.com/user-attachments/assets/d61551f2-72de-4ef6-b631-6f509cc92a8d" />
      <br><b>Annual Performance Report</b><br><sub>Charts · tables · corporate style</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" align="center">
      <img width="954" height="1350" alt="Example18" src="https://github.com/user-attachments/assets/fb5c87b2-dd23-409a-966c-23362c35dd2e" />
      <br><b>Arabic (RTL)</b><br><sub>Right‑to‑left shaping &amp; bidi</sub>
    </td>
    <td width="50%" align="center">
      <img width="1112" height="1367" alt="Example17" src="https://github.com/user-attachments/assets/09ada836-2669-4dc3-a609-4a0e99c0a00c" />
      <br><b>Chinese (CJK)</b><br><sub>Embedded CJK fonts</sub>
    </td>
  </tr>
</table>

## 🚀 Quick start

**Try it now** — no install — in the browser app: **[pdfmaker.ink/web](https://pdfmaker.ink/web/index.html)**.

Or use the desktop CLI:

```bash
pdfmaker -i document.html -c styles.css -o output.pdf
# choose a paper size (A4, Letter, Legal, or WIDTHxHEIGHT in points)
pdfmaker -i invoice.html -p A4 -o invoice.pdf
```

## ⬇ Get PDFMaker

| Platform | Download |
|---|---|
| 🪟 **Windows** | [pdfmaker.exe](https://github.com/sorainnosia/PDFMaker/releases/download/1.0/pdfmaker.exe) |
| 🤖 **Android** | [Google Play](https://play.google.com/store/apps/details?id=com.pdfmaker.johnkenedy) |
| 🌐 **Web (WASM)** | [pdfmaker.ink/web](https://pdfmaker.ink/web/index.html) |

## 🔗 Links

- **Website:** https://pdfmaker.ink
- **Documentation:** https://pdfmaker.ink/documentation.html
- **Features:** https://pdfmaker.ink/features.html
- **Examples:** https://pdfmaker.ink/examples.html

<div align="center"><sub>Built with 🦀 Rust — HTML to beautiful PDF, anywhere.</sub></div>
