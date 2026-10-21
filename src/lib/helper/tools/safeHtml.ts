// HTML venu d'un flux (descriptions de podcasts) : seules quelques balises de texte passent.
// Rien ne s'exécute ni ne se charge (pas d'image, de script, de style ni d'attribut d'événement).

const KEEP = new Set(["p", "br", "b", "strong", "i", "em", "u", "ul", "ol", "li", "blockquote", "a"]);
// Titres et blocs ramenés à un paragraphe ; leur texte reste.
const AS_PARAGRAPH = new Set(["h1", "h2", "h3", "h4", "h5", "h6", "div", "section", "article", "pre"]);
const DROP = new Set(["script", "style", "iframe", "object", "embed", "noscript", "template", "svg", "math", "head", "title", "form"]);

/** Fragment HTML nettoyé, à insérer avec `{@html}`. */
export function safeHtml(html: string | null | undefined): string {
  if (!html) return "";
  // Texte brut sans balise : les retours à la ligne font les paragraphes.
  if (!/<[a-z!/]/i.test(html)) {
    return html
      .split(/\n{2,}/)
      .map((p) => `<p>${escapeText(p).replace(/\n/g, "<br>")}</p>`)
      .join("");
  }
  // Document inerte : l'analyse n'exécute rien et ne charge aucune ressource.
  const source = new DOMParser().parseFromString(html, "text/html");
  const out = document.implementation.createHTMLDocument("");
  const root = out.createElement("div");
  copyChildren(source.body, root, out);
  return root.innerHTML;
}

function copyChildren(from: Node, to: Node, doc: Document) {
  for (const node of Array.from(from.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      to.appendChild(doc.createTextNode(node.textContent ?? ""));
      continue;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) continue;
    const el = node as Element;
    const tag = el.tagName.toLowerCase();
    if (DROP.has(tag)) continue;
    if (KEEP.has(tag) || AS_PARAGRAPH.has(tag)) {
      const copy = doc.createElement(AS_PARAGRAPH.has(tag) ? "p" : tag);
      if (tag === "a") {
        const href = el.getAttribute("href")?.trim() ?? "";
        // Seuls les liens web et e-mail : ils s'ouvrent dans le navigateur du système.
        if (/^(https?:|mailto:)/i.test(href)) copy.setAttribute("href", href);
        copy.setAttribute("rel", "noopener noreferrer");
      }
      copyChildren(el, copy, doc);
      // Paragraphe vide (fréquent dans les flux) : il ne laisserait qu'un blanc.
      if (copy.tagName === "P" && !copy.textContent?.trim()) continue;
      to.appendChild(copy);
    } else {
      // Balise inconnue (span, font, img…) : on garde son texte, pas la balise.
      copyChildren(el, to, doc);
    }
  }
}

function escapeText(s: string): string {
  return s.replace(/&(?![a-z#0-9]+;)/gi, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Adresse web (http/https) : seule ouverte depuis un flux, le reste ferait naviguer l'appli. */
export const isWebUrl = (u: string | null | undefined): u is string => !!u && /^https?:\/\//i.test(u.trim());
