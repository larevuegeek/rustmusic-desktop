<script lang="ts">
  // Lecteur en mode radio : station, titre en cours, direct ou différé ; Précédent / Suivant parcourent les favorites.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { player } from "#lib/stores/player/player.store";
  import { radioPlayer } from "#lib/stores/radio/radioPlayer.store";
  import { radioStore, findFavorite } from "#lib/stores/radio/radio.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import LoadingRing from "#lib/components/ui/loader/LoadingRing.svelte";
  import RadioLogo from "#lib/components/radio/RadioLogo.svelte";
  import PlayerSoundBar from "./PlayerSoundBar.svelte";

  let { ib, gros }: { ib: string; gros: string } = $props();

  const station = $derived($radioPlayer.station);
  const playing = $derived($radioPlayer.status === "playing");
  const enAttente = $derived($radioPlayer.status === "connecting" || (playing && $player.isPreparing));
  // Le retard vient des pauses ; en deçà de 3 s, inutile de proposer le direct.
  const differe = $derived($radioPlayer.behindSecs >= 3);
  const favorite = $derived(station ? findFavorite($radioStore.radios, station) : null);
  const auMoinsDeux = $derived($radioStore.radios.length > 1);
  const genres = $derived((station?.tags ?? []).slice(0, 2).map((g) => g.charAt(0).toUpperCase() + g.slice(1)).join(" · "));

  function retard(secs: number) {
    const s = Math.round(secs);
    const m = Math.floor(s / 60);
    return m > 0 ? `−${m}:${String(s % 60).padStart(2, "0")}` : `−${s} s`;
  }

  async function toggleFavorite() {
    if (!station) return;
    try {
      if (favorite) await radioStore.remove(favorite.id);
      else await radioStore.add(station);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }
</script>

{#if station}
  <div
    class="grid grid-cols-[minmax(220px,1fr)_minmax(300px,1.4fr)_minmax(max-content,1fr)] items-center gap-7 max-[1250px]:gap-5 h-23 pl-2 pr-5
           max-lg:grid-cols-[minmax(0,1fr)_auto] max-lg:h-auto max-lg:py-3 max-lg:gap-x-3 max-lg:gap-y-2"
  >
    <div class="flex items-center gap-4 min-w-0">
      <div class="relative shrink-0">
        <RadioLogo path={$radioPlayer.logoPath} favicon={station.favicon} icone={30} class="lecteur-pochette w-19 h-19 rounded-xl" />
        {#if playing}
          <span class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-rose-500 ring-2 ring-white dark:ring-[#141816] animate-pulse"></span>
        {/if}
      </div>
      <div class="min-w-0 flex flex-col gap-0.75 leading-tight">
        <span class="min-w-0 truncate text-[17px] font-bold tracking-[-0.01em] text-(--lc-tx)" title={station.name}>{station.name}</span>
        <span class="min-w-0 truncate text-sm font-medium {$radioPlayer.title ? 'text-(--lc-acc)' : 'text-(--lc-mu)'}" title={$radioPlayer.title ?? genres}>
          {$radioPlayer.title ?? (genres || $t("radios.title"))}
        </span>
        {#if differe}
          <span class="self-start h-5 px-2 inline-flex items-center gap-1 rounded-full text-[10.5px] font-bold bg-amber-500/15 text-amber-700 dark:text-amber-300">
            <Icon icon="material-symbols:history-rounded" width="12" />{$t("radios.delayed")} {retard($radioPlayer.behindSecs)}
          </span>
        {/if}
      </div>
    </div>

    <div class="justify-self-center w-full max-w-155 flex flex-col items-center gap-1.5 min-w-0 max-lg:max-w-none max-lg:col-span-2 max-lg:row-start-2">
      <div class="flex items-center gap-1.5">
        <!-- Pendant du bouton Stop : la lecture reste au centre. -->
        <span class="w-9.5 shrink-0" aria-hidden="true"></span>
        <button type="button" class="{gros} disabled:opacity-35 disabled:cursor-default" disabled={!auMoinsDeux}
                title={$t("radios.prev_favorite")} aria-label={$t("radios.prev_favorite")} onclick={() => radioPlayer.prev()}>
          <Icon icon="material-symbols-light:skip-previous-rounded" width="32" />
        </button>
        <button
          type="button"
          class="lecteur-lecture relative w-12.5 h-12.5 mx-2 shrink-0 flex items-center justify-center rounded-full cursor-pointer
                 bg-(--lc-play) text-(--lc-play-tx) transition-transform hover:scale-105 active:scale-95"
          title={enAttente ? $t("radios.connecting") : playing ? $t("player.pause") : $t("player.play")}
          aria-label={playing ? $t("player.pause") : $t("player.play")}
          aria-busy={enAttente}
          onclick={() => radioPlayer.toggle()}
        >
          <Icon icon={playing ? "material-symbols-light:pause-rounded" : "material-symbols-light:play-arrow-rounded"} width="32" />
          <LoadingRing actif={enAttente} />
        </button>
        <button type="button" class="{gros} disabled:opacity-35 disabled:cursor-default" disabled={!auMoinsDeux}
                title={$t("radios.next_favorite")} aria-label={$t("radios.next_favorite")} onclick={() => radioPlayer.next()}>
          <Icon icon="material-symbols-light:skip-next-rounded" width="32" />
        </button>
        <button type="button" class={ib} title={$t("player.stop")} aria-label={$t("player.stop")} onclick={() => radioPlayer.stop()}>
          <Icon icon="material-symbols-light:stop-outline-rounded" width="22" />
        </button>
      </div>
      <!-- À la place de la barre de progression : le direct, ou le retard et le moyen de le rattraper. -->
      <div class="flex items-center justify-center gap-3 w-full h-5 text-[11px] text-(--lc-mu)">
        {#if differe}
          <button type="button" onclick={() => radioPlayer.goLive()}
                  class="h-6 px-2.5 inline-flex items-center gap-1.5 rounded-full cursor-pointer text-[11.5px] font-bold transition-colors bg-rose-600 text-white hover:bg-rose-500">
            <Icon icon="material-symbols:fast-forward-rounded" width="14" />{$t("radios.go_live")}
            <span class="font-mono font-medium tabular-nums opacity-80">{retard($radioPlayer.behindSecs)}</span>
          </button>
        {:else}
          <span class="flex-1 h-1 rounded-full bg-linear-to-r from-transparent via-rose-500/50 to-rose-500"></span>
          <span class="font-semibold uppercase tracking-wider text-rose-600 dark:text-rose-300">{$t("radios.live")}</span>
          <span class="flex-1 h-1 rounded-full bg-linear-to-l from-transparent via-rose-500/50 to-rose-500"></span>
        {/if}
      </div>
    </div>

    <div class="flex items-center justify-end gap-0.5 max-lg:col-start-2 max-lg:row-start-1">
      <button type="button" class="{ib} {favorite ? 'text-amber-500! dark:text-amber-300!' : ''}" aria-pressed={!!favorite}
              title={favorite ? $t("radios.unfavorite") : $t("radios.favorite")} aria-label={favorite ? $t("radios.unfavorite") : $t("radios.favorite")}
              onclick={toggleFavorite}>
        <Icon icon={favorite ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="22" />
      </button>
      <span class="w-px h-5.5 mx-2 shrink-0 bg-(--lc-trait) max-md:hidden"></span>
      <PlayerSoundBar />
    </div>
  </div>
{/if}
