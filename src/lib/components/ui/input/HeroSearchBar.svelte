<script lang="ts">
  // Grande barre de recherche d'en-tête : mots-clés, ou adresse collée (flux de podcast, radio).
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";

  type Accent = "violet" | "rose";

  let {
    value = $bindable(""),
    busy = false,
    placeholder,
    searchLabel,
    addLabel,
    addressIcon = "material-symbols:link-rounded",
    accent = "violet",
    onsubmit,
  }: {
    value?: string;
    busy?: boolean;
    placeholder: string;
    searchLabel: string;
    /** Libellé du bouton quand la saisie est une adresse. */
    addLabel: string;
    addressIcon?: string;
    accent?: Accent;
    onsubmit: (saisie: string, adresse: boolean) => void;
  } = $props();

  // Classes écrites en entier : Tailwind ne voit pas les noms composés.
  const ACCENTS: Record<Accent, { focus: string; icon: string; button: string; active: string }> = {
    violet: {
      focus: "focus-within:border-violet-500/70 focus-within:shadow-[0_0_0_4px_rgba(139,92,246,0.16),0_8px_28px_rgba(0,0,0,0.12)]",
      icon: "group-focus-within:text-violet-500 dark:group-focus-within:text-violet-300",
      button: "bg-violet-600",
      active: "hover:bg-violet-500 shadow-[0_6px_18px_rgba(139,92,246,0.35)]",
    },
    rose: {
      focus: "focus-within:border-rose-500/70 focus-within:shadow-[0_0_0_4px_rgba(244,63,94,0.16),0_8px_28px_rgba(0,0,0,0.12)]",
      icon: "group-focus-within:text-rose-500 dark:group-focus-within:text-rose-300",
      button: "bg-rose-600",
      active: "hover:bg-rose-500 shadow-[0_6px_18px_rgba(244,63,94,0.35)]",
    },
  };
  const a = $derived(ACCENTS[accent]);

  // Une adresse se reconnaît à son schéma ou à sa forme (domaine + chemin, sans espace).
  const adresse = $derived.by(() => {
    const s = value.trim();
    return /^(https?|feed|itpc|pcast|podcast):/i.test(s) || (!/\s/.test(s) && /^[\w-]+(\.[\w-]+)+\/\S*$/.test(s));
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (value.trim() && !busy) onsubmit(value.trim(), adresse);
  }
</script>

<form onsubmit={submit} role="search"
      class="group h-13 flex items-center gap-2 pl-4.5 pr-1.5 rounded-2xl border transition-[border-color,box-shadow,background-color] duration-200
             bg-(--rg-carte)/85 border-(--rg-bd) backdrop-blur-sm shadow-[0_8px_28px_rgba(0,0,0,0.08)] dark:shadow-[0_8px_28px_rgba(0,0,0,0.35)]
             hover:border-(--rg-bd2) {a.focus}">
  <Icon icon={adresse ? addressIcon : "material-symbols:search-rounded"} width="22" class="shrink-0 transition-colors text-(--rg-mu) {a.icon}" />
  <!-- Le contour du champ est porté par le formulaire : pas d'anneau du navigateur. -->
  <input bind:value data-focus-ring="none" spellcheck="false" autocomplete="off"
         class="flex-1 min-w-0 h-full bg-transparent border-0 outline-none focus:outline-none focus-visible:outline-none text-[15px] text-(--rg-tx) placeholder:text-(--rg-ph)"
         {placeholder} aria-label={searchLabel} />
  {#if value}
    <button type="button" onclick={() => (value = "")} aria-label={$t("common.clear")} title={$t("common.clear")}
            class="shrink-0 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer text-(--rg-mu) hover:text-(--rg-tx) hover:bg-(--rg-s2)">
      <Icon icon="material-symbols:close-rounded" width="18" />
    </button>
  {/if}
  <button type="submit" disabled={!value.trim() || busy}
          class="shrink-0 h-10 px-4.5 inline-flex items-center gap-2 rounded-xl text-sm font-bold transition text-white {a.button}
                 {value.trim() ? `cursor-pointer disabled:cursor-wait ${a.active}` : 'cursor-default'}">
    {#if busy}
      <Icon icon="material-symbols:progress-activity" width="18" class="animate-spin" />
    {:else}
      <Icon icon={adresse ? "material-symbols:add-rounded" : "material-symbols:search-rounded"} width="18" />
    {/if}
    <span class="max-sm:hidden">{adresse ? addLabel : searchLabel}</span>
  </button>
</form>
