import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from '@tailwindcss/vite';
import { icones } from "./scripts/vite-icones.js";
// @ts-expect-error node:fs is a nodejs module (no @types/node installed)
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf-8"));

// Le CSS d'un composant Svelte est un module virtuel (`X.svelte?svelte&type=style…`)
// que vite-plugin-svelte sert depuis la compilation du composant. S'il est demandé
// avant elle (styles mis en ligne par SvelteKit côté serveur, où rien n'est
// compilé avec `ssr = false`, ou rechargement en pleine course), le plugin ne
// répond rien : Vite lit alors le fichier .svelte brut et le passe à Tailwind,
// qui le refuse (« Invalid declaration: `<script lang="ts">…` »). On répond un
// CSS vide à la place ; le composant apporte ses styles quand il est compilé.
/** @type {import("vite").Plugin} */
const stylesSvelteManquants = {
  name: "rustmusic:styles-svelte-manquants",
  enforce: "post",
  load(id) {
    if (/\.svelte\?(?:.*&)?type=style\b/.test(id)) return "";
  },
};

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    tailwindcss(),
    icones(),
    // Depuis SvelteKit 3, la config passe ici (`svelte.config.js` n'est plus lu).
    // Tauri n'a pas de serveur Node pour le SSR : adapter-static avec un repli
    // sur index.html met le site en mode SPA.
    // See: https://svelte.dev/docs/kit/single-page-apps
    // See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
    sveltekit({
      preprocess: vitePreprocess(),
      adapter: adapter({
        fallback: "index.html",
      }),
    }),
    stylesSvelteManquants,
  ],

  // Version de l'app injectée depuis package.json (affichée dans les réglages)
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
