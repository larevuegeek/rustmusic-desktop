<script lang="ts">
  // Pastille ronde d'un artiste : sa photo si on la connaît, sinon ses initiales sur une teinte tirée du nom.
  import CoverImg from "./CoverImg.svelte";

  // `deux` : initiales du premier et du dernier mot (« AG »), pour distinguer une grille de noms qui commencent pareil.
  let { name, path = null, deux = false, class: cls = "w-8 h-8 text-[12px]" }: { name: string; path?: string | null; deux?: boolean; class?: string } = $props();

  const mots = $derived(name.trim().split(/\s+/).filter((m) => /\p{L}/u.test(m)));
  const initiales = $derived(
    (deux && mots.length > 1 ? [mots[0], mots[mots.length - 1]] : [mots[0] ?? name.trim()]).map((m) => [...m.replace(/^\P{L}+/u, "")][0] ?? "").join("").toUpperCase(),
  );
  const hue = $derived([...name.trim().toLowerCase()].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));
</script>

<span class="relative shrink-0 rounded-full overflow-hidden flex items-center justify-center font-bold select-none
             bg-[oklch(0.92_0.04_var(--h))] text-[oklch(0.42_0.1_var(--h))] dark:bg-[oklch(0.34_0.05_var(--h))] dark:text-[oklch(0.88_0.06_var(--h))] {cls}"
      style="--h: {hue}" aria-hidden="true">
  <!-- L'initiale reste dessous : visible tant que la photo charge, ou si elle manque. -->
  {initiales}
  {#if path}<CoverImg {path} alt="" size="1x" class="absolute inset-0 w-full h-full object-cover" />{/if}
</span>
