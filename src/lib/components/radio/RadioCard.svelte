<script lang="ts">
  // Carte d'une station : logo, nom, genres ; étoile des favoris, lecture au survol, menu « … ».
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { radioStore, findFavorite } from "#lib/stores/radio/radio.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import RadioLogo from "./RadioLogo.svelte";
  import RadioContextMenu from "#lib/components/ui/contextmenu/RadioContextMenu.svelte";
  import type { RadioResult } from "#lib/types/ui/radio/Radio";

  let { station, logoPath = null, rang = null, onplay }: {
    station: RadioResult;
    /** Logo en cache d'une favorite. */
    logoPath?: string | null;
    /** Place au classement, affichée sur le logo. */
    rang?: number | null;
    onplay: () => void;
  } = $props();

  const favorite = $derived(findFavorite($radioStore.radios, station));
  const genres = $derived(station.tags.slice(0, 2).map((g) => g.charAt(0).toUpperCase() + g.slice(1)).join(" · "));
  const format = $derived([station.codec, station.bitrate ? `${station.bitrate} kbit/s` : null].filter(Boolean).join(" "));

  let busy = $state(false);
  async function toggleFavorite(e: MouseEvent) {
    e.stopPropagation();
    busy = true;
    try {
      if (favorite) await radioStore.remove(favorite.id);
      else await radioStore.add(station);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    } finally {
      busy = false;
    }
  }

  // « … » (sous le bouton) ou clic droit (sous le pointeur).
  let menu = $state<{ x: number; y: number; top: number } | null>(null);
  function openMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = e.type === "contextmenu" ? { x: e.clientX, y: e.clientY, top: e.clientY } : { x: r.left, y: r.bottom + 4, top: r.top - 4 };
  }
</script>

<div class="group relative min-w-0" role="presentation" oncontextmenu={openMenu}>
  <button type="button" onclick={onplay} title={format || undefined} class="w-full min-w-0 flex flex-col gap-2.5 text-left cursor-pointer">
    <span class="relative block">
      <RadioLogo path={logoPath} favicon={station.favicon} icone={44}
                 class="aspect-square w-full rounded-2xl shadow-[0_10px_26px_rgba(0,0,0,0.16)] dark:shadow-[0_10px_26px_rgba(0,0,0,0.5)] transition-transform duration-200 group-hover:-translate-y-0.5" />
      {#if rang != null}
        <span class="absolute left-2 top-2 min-w-7 h-7 px-1.5 inline-flex items-center justify-center rounded-full text-[12px] font-extrabold tabular-nums bg-black/60 text-white backdrop-blur-sm">{rang}</span>
      {/if}
      {#if station.reduced_quality}
        <span class="absolute left-2 bottom-2 h-5.5 px-2 inline-flex items-center rounded-full text-[10.5px] font-bold bg-amber-500/90 text-black" title={$t("radios.reduced_quality")}>AAC+</span>
      {/if}
      <span class="absolute right-2.5 bottom-2.5 w-11 h-11 rounded-full flex items-center justify-center text-white bg-rose-600 shadow-[0_8px_20px_rgba(244,63,94,0.45)]
                   opacity-0 translate-y-1 transition group-hover:opacity-100 group-hover:translate-y-0" aria-hidden="true">
        <Icon icon="material-symbols:play-arrow-rounded" width="26" />
      </span>
    </span>
    <span class="min-w-0 pl-0.5 pr-8">
      <span class="block truncate text-[14.5px] font-bold text-(--rg-tx)">{station.name}</span>
      <span class="block truncate text-[12.5px] text-(--rg-mu)">{genres || format || "\u00a0"}</span>
    </span>
  </button>
  <!-- Frères de la carte : un bouton dans un bouton n'est pas du HTML valide. -->
  <button type="button" onclick={toggleFavorite} disabled={busy} aria-pressed={!!favorite}
          title={favorite ? $t("radios.unfavorite") : $t("radios.favorite")} aria-label={favorite ? $t("radios.unfavorite") : $t("radios.favorite")}
          class="absolute right-2 top-2 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition backdrop-blur-sm bg-black/55 disabled:cursor-wait
                 {favorite ? 'text-amber-300' : 'text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-black/75'}">
    <Icon icon={favorite ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="19" />
  </button>
  <button type="button" onclick={openMenu} title={$t("common.menu")} aria-label={$t("common.menu")} aria-expanded={menu !== null}
          class="absolute right-0 bottom-0.5 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)
                 {menu ? 'bg-(--rg-s2) text-(--rg-tx)' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}">
    <Icon icon="material-symbols:more-horiz" width="20" />
  </button>
</div>

{#if menu}
  <RadioContextMenu x={menu.x} y={menu.y} top={menu.top} {station} {logoPath} {onplay} onclose={() => (menu = null)} />
{/if}
