// Embarque les seules icônes Iconify citées dans src/ : l'appli n'a pas de réseau
// (la CSP bloque l'API Iconify), et les packs entiers pèseraient des dizaines de Mo.
// @ts-nocheck — module Node exécuté par Vite.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import { getIcons } from "@iconify/utils";

const ID = "virtual:icones";
const RESOLU = "\0" + ID;
const PACKS = ["material-symbols-light", "material-symbols", "simple-icons", "radix-icons", "heroicons", "lucide", "mynaui", "tabler", "uit", "ph"];
// « pack:nom » partout dans le code, chaînes et ternaires compris ; pas « graph:… ».
const MOTIF = new RegExp(`(?<![\\w-])(${PACKS.join("|")}):([a-z0-9]+(?:-[a-z0-9]+)*)`, "g");

function lister(dossier, fichiers = []) {
  for (const nom of readdirSync(dossier)) {
    const chemin = join(dossier, nom);
    if (statSync(chemin).isDirectory()) lister(chemin, fichiers);
    else if (/\.(svelte|ts|js)$/.test(nom)) fichiers.push(chemin);
  }
  return fichiers;
}

export function icones({ racine = "src" } = {}) {
  const require = createRequire(import.meta.url);
  return {
    name: "rustmusic-icones",
    resolveId(id) {
      if (id === ID) return RESOLU;
    },
    load(id) {
      if (id !== RESOLU) return;
      const voulues = new Map();
      for (const fichier of lister(racine)) {
        for (const [, pack, nom] of readFileSync(fichier, "utf8").matchAll(MOTIF)) {
          if (!voulues.has(pack)) voulues.set(pack, new Set());
          voulues.get(pack).add(nom);
        }
      }
      const collections = [];
      for (const [pack, noms] of voulues) {
        const donnees = JSON.parse(readFileSync(require.resolve(`@iconify-json/${pack}/icons.json`), "utf8"));
        const extrait = getIcons(donnees, [...noms], true);
        if (!extrait) continue;
        if (extrait.not_found?.length) this.warn(`icônes introuvables (${pack}) : ${extrait.not_found.join(", ")}`);
        delete extrait.not_found;
        collections.push(extrait);
      }
      return `export default ${JSON.stringify(collections)};`;
    },
  };
}
