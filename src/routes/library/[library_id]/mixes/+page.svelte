<script lang="ts">
// Tous les mix de la bibliothèque : pour vous, les vôtres, un par genre, un par décennie.
import { page } from "$app/state";
import { onDestroy } from "svelte";
import Icon from "@iconify/svelte";
import { invoke } from "@tauri-apps/api/core";
import { t } from "$lib/i18n";
import { libraryHeader } from "$lib/stores/library/libraryHeader";
import { libraryContentStore } from "$lib/stores/library/libraryContent.store";
import { playlistStore } from "$lib/stores/playlist/playlist.store";
import { renouveler } from "$lib/stores/mix/mix.store";
import { mixPourVous, mixGenres, mixDecennies, mixCrees, type SpecMix } from "$lib/config/mixes";
import { actionsMix, nouveauMix } from "$lib/actions/mix/MixAction";
import LibraryImportingLoader from "$lib/components/library/common/loader/LibraryImportingLoader.svelte";
import { importAffiche } from "$lib/stores/library/importProgress.store";
import ToolbarButton from "$lib/components/ui/button/ToolbarButton.svelte";
import MixCard from "$lib/components/mix/MixCard.svelte";
import type { GenreMix } from "$lib/types/ui/library/genre/GenreMix";
import type { LibraryStats } from "$lib/types/ui/library/stats/LibraryStats";

const libraryId = $derived(Number(page.params.library_id));

let genres = $state<GenreMix[]>([]);
let hires = $state(0);
$effect(() => {
  const id = libraryId;
  let perime = false;
  genres = [];
  hires = 0;
  invoke<GenreMix[]>("get_mix_genres", { libraryId: id }).then((g) => { if (!perime) genres = g; }).catch(() => {});
  invoke<LibraryStats>("get_library_stats", { libraryId: id }).then((s) => { if (!perime) hires = s.quality_hires ?? 0; }).catch(() => {});
  return () => { perime = true; };
});

const pourVous = $derived(mixPourVous(genres, hires, $t));
const perso = $derived(mixCrees($playlistStore.playlists, $t));
// Les genres déjà mis en avant ne reviennent pas plus bas.
const parGenre = $derived(mixGenres(genres, $t).filter((m) => !pourVous.some((p) => p.cle === m.cle)));
const decennies = $derived(mixDecennies($libraryContentStore.albums, $t).reverse());

const total = $derived(pourVous.length + perso.length + parGenre.length + decennies.length);
$effect(() => {
  const [n, p] = [total, perso.length];
  libraryHeader.update(() => ({
    action: { cle: "mix_view.create", icone: "material-symbols:add-rounded", lancer: async () => nouveauMix() },
    chiffres: [
      { n, un: "library_head.mixes_one", plusieurs: "library_head.mixes_n" },
      { n: p, un: "library_head.own_mixes_one", plusieurs: "library_head.own_mixes_n" },
    ],
  }));
});
onDestroy(() => libraryHeader.update((h) => ({ ...h, action: null, chiffres: null })));

const grille = "grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-x-5 gap-y-7";
</script>

{#snippet titre(texte: string, sous: string)}
  <div class="flex items-baseline gap-3 mb-4">
    <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{texte}</h2>
    <span class="text-[13px] text-(--rg-mu)">{sous}</span>
  </div>
{/snippet}

{#snippet cartes(liste: SpecMix[], variante: "carre" | "decennie")}
  {#each liste as s (s.cle)}
    <MixCard {libraryId} cle={s.cle} source={s.source} type={s.type} nom={s.nom} aide={s.aide} h={s.h}
             {variante} pre={s.pre} court={s.court} actions={actionsMix(s, libraryId)} />
  {/each}
{/snippet}

{#if $importAffiche}
  <LibraryImportingLoader />
{:else}
  <div class="flex flex-col h-full">
    <!-- ─── Outils ─── -->
    <div class="shrink-0 flex flex-wrap items-center gap-2.5 pl-4 md:pl-8 pr-4 md:pr-14 py-3">
      <span class="flex-1 text-[13px] text-(--rg-mu)">{$t("mix_view.for_you_desc")}</span>
      <ToolbarButton icon="material-symbols:refresh-rounded" label={$t("mix_view.redraw_all")} onclick={() => renouveler(libraryId)} />
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto scrollbar-app pl-4 md:pl-8 pr-10 md:pr-14 pb-16">
      <div class="@container flex flex-col gap-12 pt-3">
        {#if pourVous.length > 0}
          <section>
            {@render titre($t("home.for_you"), $t("home.for_you_desc"))}
            <div class={grille}>{@render cartes(pourVous, "carre")}</div>
          </section>
        {/if}

        <section>
          {@render titre($t("mix_view.yours"), $t("mix_view.yours_desc"))}
          <div class={grille}>
            {@render cartes(perso, "carre")}
            <!-- Créer : une carte vide à la place d'un mix. -->
            <button type="button" onclick={nouveauMix}
                    class="group min-w-0 flex flex-col gap-3 text-left cursor-pointer">
              <span class="aspect-square w-full rounded-2xl border-2 border-dashed flex flex-col items-center justify-center gap-3 px-5 text-center
                           border-(--rg-bd2) text-(--rg-mu) transition-colors group-hover:border-(--rg-g) group-hover:text-(--rg-gtx)">
                <span class="w-14 h-14 rounded-full flex items-center justify-center bg-(--rg-gbg) text-(--rg-g)">
                  <Icon icon="material-symbols:add-rounded" width="30" />
                </span>
                <span class="text-[15px] font-bold text-(--rg-tx)">{$t("mix_view.create")}</span>
              </span>
              <span class="px-0.5 text-[13px] leading-snug text-(--rg-mu)">{$t("mix_view.create_desc")}</span>
            </button>
          </div>
        </section>

        {#if parGenre.length > 0}
          <section>
            {@render titre($t("mix_view.by_genre"), $t("mix_view.by_genre_desc"))}
            <div class={grille}>{@render cartes(parGenre, "carre")}</div>
          </section>
        {/if}

        {#if decennies.length > 0}
          <section>
            {@render titre($t("home.decades"), $t("home.decades_desc"))}
            <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-x-5 gap-y-7">{@render cartes(decennies, "decennie")}</div>
          </section>
        {/if}
      </div>
    </div>
  </div>
{/if}
