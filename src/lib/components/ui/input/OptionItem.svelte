<script lang="ts">
  // Ligne d'une carte `OptionGroup`. Avec `onclick`, toute la ligne agit
  // (interrupteur) ; masquée quand la recherche des réglages ne la trouve pas.
  import type { Snippet } from "svelte";
  import { correspond, rechercheReglages } from "$lib/stores/ui/settingsSearch.store";

  let {
    title,
    desc,
    keywords,
    value,
    onclick,
    disabled = false,
    lead,
    children,
  }: {
    title: string;
    desc?: string;
    /** Mots en plus pour la recherche, jamais affichés. */
    keywords?: string;
    /** Valeur technique (chemin, adresse…) en petite ligne mono, tronquée, entière au survol. */
    value?: string;
    onclick?: () => void;
    /** Estompée et inerte : un autre réglage la conditionne. */
    disabled?: boolean;
    /** Avant le texte : pastille, icône… */
    lead?: Snippet;
    children?: Snippet;
  } = $props();

  const visible = $derived(correspond($rechercheReglages, title, desc, keywords));
</script>

<!-- La ligne double l'interrupteur à la souris ; au clavier, c'est lui qui a le focus. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  data-reglage
  hidden={!visible}
  class="flex items-center gap-4 px-5 py-4 leading-[1.2] transition-colors
         {disabled ? 'opacity-45' : onclick ? 'cursor-pointer hover:bg-(--rg-hover)' : ''}"
  onclick={disabled ? undefined : onclick}
>
  {#if lead}{@render lead()}{/if}
  <div class="flex-1 min-w-0">
    <p class="text-[15px] font-semibold text-(--rg-tx)">{title}</p>
    {#if desc}<p class="mt-0.75 text-[13px] leading-[1.25] text-(--rg-mu) text-pretty">{desc}</p>{/if}
    {#if value}<p class="mt-1.5 font-mono text-xs text-(--rg-mu2) truncate" title={value}>{value}</p>{/if}
  </div>
  {#if children}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="shrink-0 flex items-center gap-2" onclick={(e) => onclick && e.stopPropagation()}>
      {@render children()}
    </div>
  {/if}
</div>
