<script lang="ts">
  // Épisodes téléchargés du profil, par émission : place occupée, écoute, suppression.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "#lib/i18n";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { player } from "#lib/stores/player/player.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { readableSize } from "#lib/helper/tools/sizeTools";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import { playerService } from "#lib/services/player/player.service";
  import { playEpisode, deleteEpisodeDownload } from "#lib/actions/podcast/PodcastAction";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import PodcastCover from "#lib/components/podcast/PodcastCover.svelte";
  import HeaderGlow from "#lib/components/ui/deco/HeaderGlow.svelte";
  import type { DownloadView, EpisodeView, PodcastView } from "#lib/types/ui/podcast/Podcast";

  const actif = $derived($settingsStore.podcasts === "true");
  const profilId = $derived($profilSelector.profilSelected?.id ?? null);

  let liste = $state<DownloadView[] | null>(null);
  async function load() {
    if (profilId == null) return;
    try {
      liste = await invoke<DownloadView[]>("get_podcast_downloads", { profilId });
    } catch (e) {
      console.error("[podcasts] téléchargements :", e);
      liste = [];
    }
  }
  $effect(() => { if (actif && profilId != null) load(); });

  const formatSize = (n: number) => readableSize(n, $currentLocale) || "0";
  const formatNumber = (n: number) => n.toLocaleString($currentLocale);
  const total = $derived((liste ?? []).reduce((s, d) => s + d.size, 0));
  const ecoutes = $derived((liste ?? []).filter((d) => d.played));
  const tailleEcoutes = $derived(ecoutes.reduce((s, d) => s + d.size, 0));

  // Regroupés par émission, dans l'ordre reçu (plus récents parus d'abord).
  const groupes = $derived.by(() => {
    const m = new Map<number, { id: number; titre: string; cover: string | null; items: DownloadView[]; size: number }>();
    for (const d of liste ?? []) {
      const g = m.get(d.podcast_id) ?? { id: d.podcast_id, titre: d.podcast_title, cover: d.cover_path, items: [], size: 0 };
      g.items.push(d);
      g.size += d.size;
      m.set(d.podcast_id, g);
    }
    return [...m.values()];
  });

  // Barre d'espace : les plus gros d'abord, une couleur chacun, le reste en « Autres ».
  const COULEURS = ["#8b5cf6", "#d946ef", "#6366f1", "#0ea5e9", "#14b8a6"];
  const parTaille = $derived([...groupes].sort((a, b) => b.size - a.size));
  const couleurDe = $derived(new Map(parTaille.slice(0, COULEURS.length).map((g, i) => [g.id, COULEURS[i]])));
  const segments = $derived.by(() => {
    const s = parTaille.slice(0, COULEURS.length).map((g) => ({ cle: String(g.id), titre: g.titre, size: g.size, couleur: couleurDe.get(g.id)! }));
    const reste = parTaille.slice(COULEURS.length).reduce((n, g) => n + g.size, 0);
    if (reste > 0) s.push({ cle: "autres", titre: $t("podcasts.others"), size: reste, couleur: "var(--rg-mu2)" });
    return s;
  });

  /** Suppression en attente de confirmation : `all`, `played`, ou l'id d'une émission. */
  let confirmer = $state<"all" | "played" | number | null>(null);

  async function clearDownloads(podcastId: number | null, playedOnly: boolean) {
    if (profilId == null) return;
    try {
      const [n, octets] = await invoke<[number, number]>("podcast_delete_downloads", { profilId, podcastId, playedOnly });
      toasts.push({ type: "success", title: $t("podcasts.freed").replace("{n}", formatNumber(n)).replace("{size}", formatSize(octets)) });
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    }
    confirmer = null;
    await load();
  }

  async function deleteDownload(d: DownloadView) {
    try {
      await deleteEpisodeDownload(d.id);
      liste = (liste ?? []).filter((x) => x.id !== d.id);
    } catch (e) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(e) });
    }
  }

  const playingPath = $derived($player.pathFile);
  const playing = $derived($player.status === "playing");

  // Lire depuis ici : la file suit l'ordre de la liste, émission comprise.
  function play(d: DownloadView, suite: DownloadView[]) {
    if (playingPath === d.file_path) {
      playerService.handleTogglePlay();
      return;
    }
    const podcast = { title: d.podcast_title, cover_path: d.cover_path } as PodcastView;
    const ep = (x: DownloadView) => ({ ...x, path: x.file_path }) as unknown as EpisodeView;
    playEpisode(podcast, ep(d), suite.map(ep));
  }

  const btn = "h-10 px-4 inline-flex items-center gap-2 rounded-xl border cursor-pointer text-sm font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx)";
  const rouge = "inline-flex items-center gap-1.5 rounded-lg cursor-pointer font-bold bg-red-600 text-white hover:bg-red-500";
</script>

{#if !actif}
  <PageState icon="material-symbols:podcasts-rounded" message={$t("podcasts.disabled")} lien={{ href: "/settings/online", label: $t("podcasts.disabled_link") }} />
{:else if liste === null}
  <PageState chargement message={$t("common.loading")} />
{:else}
<div class="h-full overflow-y-auto scrollbar-app">
  <header class="relative px-4 md:px-8 pt-6 pb-8 overflow-hidden">
    <HeaderGlow />
    <div class="relative">
      <a href="/podcasts" class="inline-flex items-center gap-1.5 text-[13.5px] font-semibold text-(--rg-tx2) hover:text-(--rg-tx)">
        <Icon icon="material-symbols:arrow-back-rounded" width="18" />{$t("podcasts.title")}
      </a>
      <div class="mt-5 flex items-center gap-5">
        <span class="w-20 h-20 max-sm:w-14 max-sm:h-14 shrink-0 rounded-[22px] flex items-center justify-center text-white
                     bg-linear-to-br from-violet-500 to-fuchsia-600 shadow-[0_14px_34px_rgba(139,92,246,0.35)]">
          <Icon icon="material-symbols:download-for-offline-rounded" width="40" />
        </span>
        <div class="min-w-0">
          <h1 class="text-[44px] max-sm:text-3xl font-extrabold tracking-[-0.03em] leading-none text-(--rg-tx)">{$t("podcasts.downloads")}</h1>
          <p class="mt-2.5 text-[15px] text-(--rg-tx2)">{$t("podcasts.downloads_tagline")}</p>
        </div>
      </div>
    </div>
  </header>

  <div class="px-4 md:px-8 pb-24">
    {#if liste.length === 0}
      <!-- ─── Rien de téléchargé ─── -->
      <div class="mt-2 flex flex-col items-center text-center gap-4 px-6 py-14 rounded-3xl border border-dashed border-(--rg-bd2)">
        <span class="w-18 h-18 rounded-full flex items-center justify-center bg-violet-500/12 text-violet-600 dark:text-violet-300 ring-8 ring-violet-500/5">
          <Icon icon="material-symbols:cloud-download-outline-rounded" width="36" />
        </span>
        <div>
          <h2 class="text-[20px] font-extrabold text-(--rg-tx)">{$t("podcasts.downloads_empty_title")}</h2>
          <p class="mt-1.5 max-w-md text-sm leading-relaxed text-(--rg-mu)">{$t("podcasts.no_downloads")}</p>
        </div>
        <a href="/podcasts" class="mt-1 h-10.5 px-5 inline-flex items-center gap-2 rounded-full text-sm font-bold transition bg-violet-600 text-white hover:bg-violet-500">
          <Icon icon="material-symbols:podcasts-rounded" width="19" />{$t("podcasts.browse_podcasts")}
        </a>
      </div>
    {:else}
      <!-- ─── Bilan : chiffres, espace par émission, ménage ─── -->
      <section class="p-5 md:p-6 rounded-3xl border border-(--rg-bd) bg-(--rg-carte)">
        <div class="flex flex-wrap items-end gap-x-10 gap-y-4">
          <div>
            <p class="text-[30px] font-extrabold leading-none tabular-nums text-(--rg-tx)">{formatNumber(liste.length)}</p>
            <p class="mt-1.5 text-[13px] text-(--rg-mu)">{$t(liste.length === 1 ? "podcasts.episodes_one" : "podcasts.episodes_n")}</p>
          </div>
          <div>
            <p class="text-[30px] font-extrabold leading-none tabular-nums text-(--rg-tx)">{formatSize(total)}</p>
            <p class="mt-1.5 text-[13px] text-(--rg-mu)">{$t("podcasts.on_disk")}</p>
          </div>
          <div>
            <p class="text-[30px] font-extrabold leading-none tabular-nums text-(--rg-tx)">{formatNumber(ecoutes.length)}</p>
            <p class="mt-1.5 text-[13px] text-(--rg-mu)">
              {$t("podcasts.stat_played")}{#if ecoutes.length > 0}{" · "}<span class="font-semibold text-violet-600 dark:text-violet-300">{$t("podcasts.reclaimable").replace("{size}", formatSize(tailleEcoutes))}</span>{/if}
            </p>
          </div>
        </div>

        <h2 class="mt-6 mb-2.5 text-[12px] font-bold uppercase tracking-widest text-(--rg-mu)">{$t("podcasts.space_by_show")}</h2>
        <div class="flex h-3 rounded-full overflow-hidden gap-0.5 bg-(--rg-s2)">
          {#each segments as s (s.cle)}
            <span class="h-full min-w-1" style="flex: {s.size} 1 0; background: {s.couleur}" title="{s.titre} · {formatSize(s.size)}"></span>
          {/each}
        </div>
        <ul class="mt-3 flex flex-wrap gap-x-5 gap-y-1.5">
          {#each segments as s (s.cle)}
            <li class="flex items-center gap-2 min-w-0 text-[13px]">
              <span class="w-2.5 h-2.5 shrink-0 rounded-full" style="background: {s.couleur}"></span>
              <span class="truncate max-w-48 text-(--rg-tx2)">{s.titre}</span>
              <span class="tabular-nums text-(--rg-mu)">{formatSize(s.size)}</span>
            </li>
          {/each}
        </ul>

        <div class="mt-6 pt-5 border-t border-(--rg-line) flex flex-wrap items-center gap-2.5">
          {#if confirmer === "all" || confirmer === "played"}
            <span class="min-h-10 py-1.5 pl-4 pr-1.5 inline-flex flex-wrap items-center gap-2 rounded-xl border text-[13px] bg-red-500/8 border-red-500/35 text-(--rg-tx)">
              <Icon icon="material-symbols:warning-outline-rounded" width="18" class="text-red-500" />
              {confirmer === "all" ? $t("podcasts.delete_all_confirm") : $t("podcasts.delete_played_confirm")}
              <button type="button" onclick={() => (confirmer = null)} class="h-7.5 px-2.5 rounded-lg cursor-pointer font-semibold text-(--rg-tx2) hover:bg-(--rg-s2)">{$t("common.cancel")}</button>
              <button type="button" onclick={() => clearDownloads(null, confirmer === "played")} class="h-7.5 pl-2 pr-3 {rouge}">
                <Icon icon="material-symbols:delete-outline-rounded" width="17" />{$t("common.delete")}
              </button>
            </span>
          {:else}
            {#if ecoutes.length > 0}
              <button type="button" onclick={() => (confirmer = "played")} class="{btn} hover:border-violet-400/60 hover:text-violet-600 dark:hover:text-violet-300">
                <Icon icon="material-symbols:done-all-rounded" width="18" />{$t("podcasts.delete_played").replace("{n}", formatNumber(ecoutes.length))}
              </button>
            {/if}
            <button type="button" onclick={() => (confirmer = "all")} class="{btn} hover:border-red-400/60 hover:text-red-500">
              <Icon icon="material-symbols:delete-outline-rounded" width="18" />{$t("podcasts.delete_all")}
            </button>
          {/if}
        </div>
      </section>

      <!-- ─── Par émission ─── -->
      {#each groupes as g (g.id)}
        <section class="mt-5 rounded-3xl border border-(--rg-bd) bg-(--rg-carte) overflow-hidden">
          <div class="flex items-center gap-4 p-4 md:px-5">
            <a href="/podcasts/{g.id}" class="flex items-center gap-4 min-w-0 flex-1 group">
              <PodcastCover path={g.cover} size="1x" icone={22} class="w-14 h-14 shrink-0 rounded-xl shadow-[0_6px_16px_rgba(0,0,0,0.18)]" />
              <span class="min-w-0">
                <span class="block truncate text-[16.5px] font-extrabold text-(--rg-tx) group-hover:underline underline-offset-2">{g.titre}</span>
                <span class="mt-0.5 flex items-center gap-2 text-[13px] text-(--rg-mu)">
                  <span class="w-2 h-2 shrink-0 rounded-full" style="background: {couleurDe.get(g.id) ?? 'var(--rg-mu2)'}"></span>
                  {formatNumber(g.items.length)} {$t(g.items.length === 1 ? "podcasts.episodes_one" : "podcasts.episodes_n")} · {formatSize(g.size)}
                </span>
              </span>
            </a>
            {#if confirmer === g.id}
              <button type="button" onclick={() => (confirmer = null)} class="h-8.5 px-3 rounded-lg cursor-pointer text-[13px] font-semibold text-(--rg-tx2) hover:bg-(--rg-s2)">{$t("common.cancel")}</button>
              <button type="button" onclick={() => clearDownloads(g.id, false)} class="h-8.5 pl-2.5 pr-3.5 text-[13px] {rouge}">
                <Icon icon="material-symbols:delete-outline-rounded" width="17" />{$t("common.delete")}
              </button>
            {:else}
              <button type="button" onclick={() => (confirmer = g.id)} title={$t("podcasts.delete_podcast_downloads")} aria-label={$t("podcasts.delete_podcast_downloads")}
                      class="w-9.5 h-9.5 shrink-0 flex items-center justify-center rounded-full border cursor-pointer transition-colors border-(--rg-bd2) text-(--rg-tx2) hover:border-red-400/60 hover:text-red-500">
                <Icon icon="material-symbols:delete-outline-rounded" width="19" />
              </button>
            {/if}
          </div>
          <ul>
            {#each g.items as d, i (d.id)}
              {@const enLecture = playingPath === d.file_path}
              {@const progression = !d.played && d.position > 0 && d.duration ? Math.min(1, d.position / d.duration) : 0}
              <li class="group flex items-center gap-3.5 px-4 md:px-5 py-3 border-t border-(--rg-line) transition-colors hover:bg-(--rg-s2)/50">
                <button type="button" onclick={() => play(d, g.items.slice(i + 1))} title={enLecture && playing ? $t("podcasts.pause") : $t("podcasts.play")}
                        aria-label={enLecture && playing ? $t("podcasts.pause") : $t("podcasts.play")}
                        class="shrink-0 w-10 h-10 rounded-full flex items-center justify-center cursor-pointer transition
                               {enLecture
                                 ? 'bg-violet-600 text-white shadow-[0_6px_18px_rgba(139,92,246,0.4)]'
                                 : 'bg-(--rg-s2) text-(--rg-tx) hover:bg-violet-600 hover:text-white'}">
                  <Icon icon={enLecture && playing ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="22" />
                </button>
                <span class="flex-1 min-w-0">
                  <span class="block truncate text-[14.5px] font-semibold
                               {enLecture ? 'text-violet-600 dark:text-violet-300' : d.played ? 'text-(--rg-mu)' : 'text-(--rg-tx)'}">{d.title}</span>
                  <span class="mt-0.5 flex items-center gap-2 min-w-0 text-[12.5px] text-(--rg-mu)">
                    <span class="truncate">{[episodeDate(d.published_at, $currentLocale), episodeDuration(d.duration)].filter(Boolean).join(" · ")}</span>
                    {#if d.played}
                      <span class="shrink-0 h-5 px-2 inline-flex items-center gap-1 rounded-full text-[11px] font-bold bg-(--rg-s2) text-(--rg-tx2)">
                        <Icon icon="material-symbols:check-rounded" width="13" />{$t("podcasts.played")}
                      </span>
                    {:else if progression > 0}
                      <span class="shrink-0 w-16 h-1 rounded-full overflow-hidden bg-(--rg-s2)">
                        <span class="block h-full rounded-full bg-violet-500" style="width: {progression * 100}%"></span>
                      </span>
                    {/if}
                  </span>
                </span>
                <span class="shrink-0 text-[12.5px] tabular-nums text-(--rg-mu)">{formatSize(d.size)}</span>
                <button type="button" onclick={() => deleteDownload(d)} title={$t("podcasts.delete_download")} aria-label={$t("podcasts.delete_download")}
                        class="shrink-0 w-8.5 h-8.5 flex items-center justify-center rounded-full cursor-pointer transition text-(--rg-mu)
                               opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-red-500/10 hover:text-red-500">
                  <Icon icon="material-symbols:delete-outline-rounded" width="18" />
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    {/if}
  </div>
</div>
{/if}

