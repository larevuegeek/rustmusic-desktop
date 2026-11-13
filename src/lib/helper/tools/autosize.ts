// Zone de texte qui grandit avec son contenu : tout le texte reste visible, sans défilement interne.
// `use:autosize={valeur}` : la valeur passée réajuste la hauteur quand elle change de l'extérieur.
export function autosize(node: HTMLTextAreaElement, _value?: unknown) {
  const fit = () => {
    node.style.height = "auto";
    node.style.height = `${node.scrollHeight}px`;
  };
  fit();
  node.addEventListener("input", fit);
  // La largeur change (panneau redimensionné) : le retour à la ligne aussi.
  const ro = new ResizeObserver(fit);
  ro.observe(node);
  return {
    update: () => queueMicrotask(fit),
    destroy() {
      node.removeEventListener("input", fit);
      ro.disconnect();
    },
  };
}
