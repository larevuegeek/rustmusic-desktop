<script lang="ts">
import Icon from "@iconify/svelte";
import type { Snippet } from "svelte";

/** Une ligne de la barre : vignette, titre, sous-titre et menu « … ». */
let { titre, sousTitre = "", actif = false, enLecture = false, replie = false, onclick, onmenu, vignette }: {
  titre: string;
  sousTitre?: string;
  actif?: boolean;
  enLecture?: boolean;
  replie?: boolean;
  onclick: () => void;
  onmenu?: (x: number, y: number) => void;
  vignette: Snippet;
} = $props();

function ouvrirMenu(e: MouseEvent) {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  onmenu?.(r.left, r.bottom + 4);
}
</script>

<div
  class="group relative {replie ? 'flex justify-center' : ''}"
  oncontextmenu={(e) => { if (!onmenu) return; e.preventDefault(); onmenu(e.clientX, e.clientY); }}
  role="presentation"
>
  <button
    type="button"
    class="flex items-center gap-3 rounded-lg text-left cursor-pointer transition-colors
           {replie ? 'p-0' : 'w-full p-1 pr-9 hover:bg-(--sb-hover)'}"
    {onclick}
    title={titre}
    aria-current={actif ? 'page' : undefined}
  >
    <span class="shrink-0 overflow-hidden rounded-md {replie ? 'w-11 h-11' : 'w-10 h-10'}
                 {replie && (actif || enLecture) ? 'ring-2 ring-(--sb-g)' : ''}">
      {@render vignette()}
    </span>

    {#if !replie}
      <span class="flex flex-col gap-px min-w-0 flex-1">
        <b class="text-sm font-bold truncate {actif || enLecture ? 'text-(--sb-g)' : 'text-(--sb-tx)'}">{titre}</b>
        {#if sousTitre}
          <small class="text-xs truncate text-(--sb-mu)">{sousTitre}</small>
        {/if}
      </span>
      {#if enLecture}
        <Icon icon="material-symbols:graphic-eq-rounded" width="20" class="shrink-0 text-(--sb-g)" />
      {/if}
    {/if}
  </button>

  <!-- Frère du bouton et non enfant : un bouton dans un bouton n'est pas du HTML valide. -->
  {#if onmenu && !replie}
    <button
      type="button"
      class="absolute right-1 top-1/2 -translate-y-1/2 w-7.5 h-7.5 rounded-lg
             flex items-center justify-center cursor-pointer transition-opacity
             opacity-0 group-hover:opacity-100 focus-visible:opacity-100
             text-(--sb-mu) hover:text-(--sb-tx) hover:bg-(--sb-s2)"
      onclick={ouvrirMenu}
      aria-label={titre}
      title={titre}
    >
      <Icon icon="material-symbols:more-horiz" width="20" />
    </button>
  {/if}
</div>
