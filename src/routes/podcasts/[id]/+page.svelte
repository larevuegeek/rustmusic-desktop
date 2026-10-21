<script lang="ts">
  // Fiche d'un podcast : en-tête, reprise de l'écoute, épisodes par mois.
  import Icon from "@iconify/svelte";
  import { untrack, onDestroy } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { t, currentLocale } from "#lib/i18n";
  import { podcastStore } from "#lib/stores/podcast/podcast.store";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { player } from "#lib/stores/player/player.store";
  import { playerService } from "#lib/services/player/player.service";
  import { playEpisode, playEpisodeNext, enqueueEpisode, downloadEpisode, deleteEpisodeDownload } from "#lib/actions/podcast/PodcastAction";
  import { episodeDate, episodeDuration } from "#lib/helper/tools/podcastFormat";
  import { isWebUrl } from "#lib/helper/tools/safeHtml";
  import DetailPage from "#lib/components/ui/layout/DetailPage.svelte";
  import PageState from "#lib/components/ui/layout/PageState.svelte";
  import SegmentedControl from "#lib/components/ui/input/SegmentedControl.svelte";
  import SafeHtml from "#lib/components/ui/text/SafeHtml.svelte";
  import Menu from "#lib/components/ui/menu/Menu.svelte";
  import MenuItem from "#lib/components/ui/menu/MenuItem.svelte";
  import PodcastCover from "#lib/components/podcast/PodcastCover.svelte";
  import PodcastEpisodeRow from "#lib/components/podcast/PodcastEpisodeRow.svelte";
  import type { EpisodeView, PodcastView } from "#lib/types/ui/podcast/Podcast";

  const id = $derived(Number(page.params.id));
  const actif = $derived($settingsStore.podcasts === "true");

  let podcast = $state<PodcastView | null>(null);
  let episodes = $state<EpisodeView[]>([]);
  let etat = $state<"chargement" | "pret" | "introuvable">("chargement");

  async function load(podcastId: number) {
    etat = "chargement";
    // Autre émission : l'ancienne ne reste pas affichée (ses boutons viseraient la nouvelle).
    if (podcast?.id !== podcastId) {
      podcast = null;
      episodes = [];
    }
    try {
      const [p, e] = await Promise.all([
        invoke<PodcastView | null>("get_podcast", { podcastId }),
        invoke<EpisodeView[]>("get_podcast_episodes", { podcastId }),
      ]);
      if (podcastId !== id) return;
      podcast = p;
      episodes = e;
      etat = p ? "pret" : "introuvable";
    } catch (err) {
      console.error("[podcast] chargement :", err);
      if (podcastId === id) etat = "introuvable";
    }
  }
  $effect(() => { load(id); });
  const reloadEpisodes = () => invoke<EpisodeView[]>("get_podcast_episodes", { podcastId: id }).then((e) => (episodes = e)).catch(() => {});

  const formatNumber = (n: number) => n.toLocaleString($currentLocale);
  const nonEcoutes = $derived(episodes.filter((e) => !e.played).length);

  // ─── Filtre, ordre, tranches ───
  type Filtre = "all" | "unplayed";
  let filtre = $state<Filtre>("all");
  // Plus anciens d'abord : pour les séries qui se suivent (épisode 1, 2, 3…).
  let anciens = $state(false);
  const visibles = $derived.by(() => {
    const l = filtre === "unplayed" ? episodes.filter((e) => !e.played) : episodes;
    return anciens ? [...l].reverse() : l;
  });
  // Les longues archives (des milliers d'épisodes) s'affichent par tranches.
  const TRANCHE = 60;
  let montres = $state(TRANCHE);
  $effect(() => { void [filtre, anciens, id]; montres = TRANCHE; });

  // Regroupés par mois de parution.
  const mois = $derived.by(() => {
    const groupes: { cle: string; titre: string; items: EpisodeView[] }[] = [];
    for (const e of visibles.slice(0, montres)) {
      const cle = e.published_at?.slice(0, 7) ?? "";
      let g = groupes.at(-1);
      if (!g || g.cle !== cle) {
        const titre = cle
          ? new Date(`${cle}-01T00:00:00Z`).toLocaleDateString($currentLocale, { month: "long", year: "numeric", timeZone: "UTC" })
          : $t("podcasts.undated");
        g = { cle, titre, items: [] };
        groupes.push(g);
      }
      g.items.push(e);
    }
    return groupes;
  });

  let descriptionOuverte = $state(false);

  // ─── À reprendre ou à écouter : l'épisode commencé, sinon le plus récent non écouté ───
  const aEcouter = $derived(
    episodes.find((e) => !e.played && e.position > 0) ?? episodes.find((e) => !e.played) ?? episodes[0] ?? null,
  );
  const reprise = $derived(!!aEcouter && !aEcouter.played && aEcouter.position > 0 && (aEcouter.duration == null || aEcouter.position < aEcouter.duration));

  // L'épisode du lecteur : son bouton devient pause.
  function playbackStateOf(e: EpisodeView): "playing" | "paused" | null {
    if (playingPath !== e.path) return null;
    return playingStatus === "playing" ? "playing" : playingStatus === "paused" ? "paused" : null;
  }

  async function play(e: EpisodeView) {
    if (!podcast) return;
    if (playbackStateOf(e)) {
      await playerService.handleTogglePlay();
      return;
    }
    // La suite : ce qui vient après dans la liste affichée.
    const i = visibles.findIndex((x) => x.id === e.id);
    await playEpisode(podcast, e, i >= 0 ? visibles.slice(i + 1) : []);
  }

  // Relus quand le lecteur change de morceau, de pause ou s'arrête (pas à chaque position).
  const playingPath = $derived($player.pathFile ?? null);
  const playingStatus = $derived($player.status);
  let precedent: string | null = null;
  $effect(() => {
    const chemin = playingPath;
    void playingStatus;
    const avant = precedent;
    precedent = chemin;
    const isOwnEpisode = (c: string | null) => !!c && untrack(() => episodes).some((e) => e.path === c);
    if (isOwnEpisode(chemin) || isOwnEpisode(avant)) {
      invoke<EpisodeView[]>("get_podcast_episodes", { podcastId: untrack(() => id) })
        .then((e) => (episodes = e))
        .catch(() => {});
    }
  });

  // ─── Actions sur les épisodes ───
  async function togglePlayed(e: EpisodeView) {
    const played = !e.played;
    try {
      await invoke("podcast_set_played", { episodeId: e.id, played });
      episodes = episodes.map((x) => (x.id === e.id ? { ...x, played, is_new: false, position: 0 } : x));
      podcast = await podcastStore.reloadOne(id);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  async function markAllPlayed() {
    try {
      await invoke("podcast_set_all_played", { podcastId: id });
      episodes = episodes.map((x) => ({ ...x, played: true, is_new: false, position: 0 }));
      podcast = await podcastStore.reloadOne(id);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  // Téléchargements lancés d'ici : la ligne tourne jusqu'au signal de fin.
  const telechargements = new SvelteSet<number>();
  const unlistenDownloaded = listen<number>("podcast-downloaded", (e) => {
    telechargements.delete(e.payload);
    reloadEpisodes();
  });
  onDestroy(() => { unlistenDownloaded.then((u) => u()); });

  async function download(e: EpisodeView) {
    telechargements.add(e.id);
    try {
      await downloadEpisode(e.id);
    } catch (err) {
      telechargements.delete(e.id);
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  async function deleteDownload(e: EpisodeView) {
    try {
      await deleteEpisodeDownload(e.id);
      await reloadEpisodes();
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  // ─── Podcast ───
  let verification = $state(false);
  async function refresh() {
    verification = true;
    try {
      const n = await podcastStore.refresh(id);
      await load(id);
      toasts.push({
        type: "info",
        title: n === 0 ? $t("podcasts.up_to_date") : `${formatNumber(n)} ${$t(n === 1 ? "podcasts.new_one" : "podcasts.new_n")}`,
      });
    } catch (err) {
      toasts.push({ type: "error", title: $t("podcasts.error_refresh"), message: String(err) });
    } finally {
      verification = false;
    }
  }

  async function togglePinned() {
    if (!podcast) return;
    const pinned = !podcast.pinned;
    try {
      await podcastStore.setPinned(id, pinned);
      podcast = { ...podcast, pinned };
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  let abonnement = $state(false);
  let confirmer = $state(false);
  async function unfollow() {
    try {
      await podcastStore.unfollow(id);
      goto("/podcasts");
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }

  const pastille = "h-7 px-3 inline-flex items-center gap-1.5 rounded-full text-[12.5px] font-semibold bg-(--rg-s2)/80 text-(--rg-tx2) backdrop-blur-sm";
  const rond = "w-11 h-11 rounded-full flex items-center justify-center cursor-pointer transition-colors border border-(--rg-bd2) text-(--rg-tx2) bg-(--rg-carte)/60 backdrop-blur-sm hover:border-(--rg-bd2) hover:text-(--rg-tx) hover:bg-(--rg-s2)";
</script>

{#if !actif}
  <PageState icon="material-symbols:podcasts-rounded" message={$t("podcasts.disabled")} lien={{ href: "/settings/online", label: $t("podcasts.disabled_link") }} />
{:else if etat === "chargement" && !podcast}
  <PageState chargement message={$t("common.loading")} />
{:else if etat === "introuvable" || !podcast}
  <PageState message={$t("podcasts.not_found")} lien={{ href: "/podcasts", label: $t("podcasts.title") }} />
{:else}
<DetailPage image={podcast.cover_path} retourHref="/podcasts" retourLabel={$t("podcasts.title")} playLabel={$t("podcasts.play")}
            onplay={aEcouter ? () => aEcouter && play(aEcouter) : undefined}>
  {#snippet mini()}
    <PodcastCover path={podcast?.cover_path ?? null} size="1x" icone={16} class="w-8.5 h-8.5 shrink-0 rounded-[7px]" />
    <b class="truncate text-[15px] text-(--rg-tx)">{podcast?.title}</b>
  {/snippet}
  {#snippet barre()}
    <a href="/podcasts/downloads" title={$t("podcasts.downloads")} aria-label={$t("podcasts.downloads")}
       class="w-8 h-8 flex items-center justify-center rounded-lg transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
      <Icon icon="material-symbols:download-for-offline-outline-rounded" width="20" />
    </a>
    <button type="button" onclick={refresh} disabled={verification} title={$t("podcasts.refresh")} aria-label={$t("podcasts.refresh")}
            class="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer transition-colors text-(--rg-mu) hover:bg-(--rg-s2) hover:text-(--rg-tx) disabled:cursor-wait">
      <Icon icon={verification ? "material-symbols:progress-activity" : "material-symbols:refresh-rounded"} width="20" class={verification ? "animate-spin" : ""} />
    </button>
  {/snippet}

  <div class="relative flex flex-wrap items-end gap-8 pt-3 pb-8">
    <PodcastCover path={podcast.cover_path} size="full" icone={64}
                  class="w-56 @max-[760px]:w-44 aspect-square shrink-0 rounded-[18px] shadow-[0_24px_60px_rgba(0,0,0,0.3)] dark:shadow-[0_24px_60px_rgba(0,0,0,0.55)]" />

    <div class="flex-[1_1_360px] min-w-0 flex flex-col gap-3.5">
      <p class="text-xs font-bold tracking-widest uppercase text-(--rg-tx2)">{$t("podcasts.kicker_podcast")}</p>
      <div>
        <h1 class="font-extrabold leading-[1.02] tracking-[-0.03em] text-balance line-clamp-2 text-(--rg-tx)
                   {podcast.title.length > 32 ? 'text-[38px] @max-[760px]:text-[28px]' : 'text-[48px] @max-[760px]:text-[34px]'}" title={podcast.title}>{podcast.title}</h1>
        {#if podcast.author}<p class="mt-1.5 text-[16px] font-semibold text-(--rg-tx2)">{podcast.author}</p>{/if}
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <span class={pastille}><Icon icon="material-symbols:podcasts-rounded" width="15" />{formatNumber(podcast.episode_count)} {$t(podcast.episode_count === 1 ? "podcasts.episodes_one" : "podcasts.episodes_n")}</span>
        {#if nonEcoutes > 0}
          <span class={pastille}><span class="w-1.5 h-1.5 rounded-full bg-violet-500"></span>{$t(nonEcoutes === 1 ? "podcasts.unplayed_one" : "podcasts.unplayed_n").replace("{n}", formatNumber(nonEcoutes))}</span>
        {/if}
        {#if podcast.latest_at}
          <span class={pastille}><Icon icon="material-symbols:schedule-outline-rounded" width="15" />{$t("podcasts.latest_on").replace("{date}", episodeDate(podcast.latest_at, $currentLocale))}</span>
        {/if}
      </div>

      <div class="flex flex-wrap items-center gap-2.5 pt-1.5">
        {#if aEcouter}
          <button type="button" onclick={() => aEcouter && play(aEcouter)}
                  class="h-11 pl-4 pr-5.5 inline-flex items-center gap-2 rounded-full cursor-pointer text-[14px] font-bold transition
                         bg-violet-600 text-white shadow-[0_8px_22px_rgba(139,92,246,0.35)] hover:bg-violet-500">
            <Icon icon="material-symbols:play-arrow-rounded" width="24" />
            {reprise ? $t("podcasts.resume") : $t("podcasts.play")}
          </button>
        {/if}

        <!-- L'abonnement : se désabonner y est rangé, avec confirmation. -->
        <div class="relative">
          <button type="button" onclick={() => { abonnement = !abonnement; confirmer = false; }} aria-expanded={abonnement}
                  class="h-11 pl-3.5 pr-3 inline-flex items-center gap-1.5 rounded-full cursor-pointer text-[14px] font-semibold transition-colors
                         border border-(--rg-bd2) bg-(--rg-carte)/60 backdrop-blur-sm text-(--rg-tx) hover:bg-(--rg-s2)">
            <Icon icon="material-symbols:check-rounded" width="19" class="text-(--rg-gtx)" />{$t("podcasts.following")}
            <Icon icon="material-symbols:expand-more-rounded" width="18" class="text-(--rg-mu)" />
          </button>
          <Menu bind:open={abonnement} align="left" class="w-72">
            {#if confirmer}
              <p class="px-2.5 pt-1.5 pb-2 text-[13px] leading-snug text-(--rg-tx2)">{$t("podcasts.unsubscribe_confirm")}</p>
              <MenuItem icon="material-symbols:do-not-disturb-on-outline-rounded" danger onclick={unfollow}>{$t("podcasts.unsubscribe")}</MenuItem>
              <MenuItem icon="material-symbols:close-rounded" onclick={() => (confirmer = false)}>{$t("common.cancel")}</MenuItem>
            {:else}
              <MenuItem icon="material-symbols:refresh-rounded" onclick={() => { abonnement = false; refresh(); }}>{$t("podcasts.refresh")}</MenuItem>
              <MenuItem icon="material-symbols:do-not-disturb-on-outline-rounded" danger onclick={() => (confirmer = true)}>{$t("podcasts.unsubscribe")}</MenuItem>
            {/if}
          </Menu>
        </div>

        <button type="button" onclick={togglePinned} aria-pressed={podcast.pinned}
                title={podcast.pinned ? $t("podcasts.unfavorite") : $t("podcasts.favorite")}
                aria-label={podcast.pinned ? $t("podcasts.unfavorite") : $t("podcasts.favorite")}
                class="{rond} {podcast.pinned ? 'text-amber-500! dark:text-amber-300! border-amber-400/50!' : ''}">
          <Icon icon={podcast.pinned ? "material-symbols:star-rounded" : "material-symbols:star-outline-rounded"} width="21" />
        </button>
        {#if nonEcoutes > 0}
          <button type="button" onclick={markAllPlayed} title={$t("podcasts.mark_all_played")} aria-label={$t("podcasts.mark_all_played")} class={rond}>
            <Icon icon="material-symbols:done-all-rounded" width="20" />
          </button>
        {/if}
        {#if isWebUrl(podcast.link)}
          <a href={podcast.link} title={$t("podcasts.website")} aria-label={$t("podcasts.website")} class={rond}>
            <Icon icon="material-symbols:language" width="20" />
          </a>
        {/if}
      </div>
    </div>
  </div>

  {#if podcast.description}
    <div class="max-w-190 pb-8">
      <SafeHtml html={podcast.description} class="text-[14.5px] leading-relaxed text-(--rg-tx2) {descriptionOuverte ? '' : 'line-clamp-2'}" />
      <button type="button" onclick={() => (descriptionOuverte = !descriptionOuverte)} class="mt-1.5 text-[13px] font-semibold cursor-pointer text-(--rg-tx) hover:underline underline-offset-2">
        {descriptionOuverte ? $t("podcasts.show_less") : $t("podcasts.show_more")}
      </button>
    </div>
  {/if}

  <!-- ─── Reprendre l'écoute ─── -->
  {#if reprise && aEcouter}
    {@const avance = Math.min(100, (aEcouter.position / (aEcouter.duration ?? aEcouter.position)) * 100)}
    <button type="button" onclick={() => aEcouter && play(aEcouter)}
            class="group w-full max-w-190 mb-9 flex items-center gap-4 p-4 pr-5 rounded-2xl text-left cursor-pointer transition-colors
                   border border-violet-500/25 bg-violet-500/8 hover:bg-violet-500/14">
      <PodcastCover path={podcast.cover_path} size="1x" icone={22} class="w-16 h-16 shrink-0 rounded-xl" />
      <span class="flex-1 min-w-0">
        <span class="block text-[11.5px] font-bold uppercase tracking-widest text-violet-600 dark:text-violet-300">{$t("podcasts.resume_listening")}</span>
        <span class="mt-0.5 block truncate text-[15.5px] font-bold text-(--rg-tx)">{aEcouter.title}</span>
        <span class="mt-2 flex items-center gap-3">
          <span class="flex-1 max-w-80 h-1.5 rounded-full bg-(--rg-s2) overflow-hidden" aria-hidden="true">
            <span class="block h-full rounded-full bg-violet-500" style="width: {avance}%"></span>
          </span>
          <span class="shrink-0 text-[12.5px] font-semibold text-(--rg-tx2)">
            {$t("podcasts.remaining").replace("{t}", episodeDuration((aEcouter.duration ?? 0) - aEcouter.position))}
          </span>
        </span>
      </span>
      <span class="shrink-0 w-12 h-12 rounded-full flex items-center justify-center bg-violet-600 text-white shadow-[0_6px_18px_rgba(139,92,246,0.4)] transition-transform group-hover:scale-105">
        <Icon icon={playbackStateOf(aEcouter) === "playing" ? "material-symbols:pause-rounded" : "material-symbols:play-arrow-rounded"} width="26" />
      </span>
    </button>
  {/if}

  <div>
    <div class="flex flex-wrap items-center gap-3 pb-2">
      <h2 class="text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx)">{$t("podcasts.episodes_title")}</h2>
      <span class="flex-1"></span>
      <button type="button" onclick={() => (anciens = !anciens)}
              class="h-8.5 px-3 inline-flex items-center gap-1.5 rounded-[10px] text-[13px] font-semibold cursor-pointer transition-colors text-(--rg-tx2) hover:bg-(--rg-s2) hover:text-(--rg-tx)">
        <Icon icon="material-symbols:swap-vert-rounded" width="18" />{anciens ? $t("podcasts.oldest_first") : $t("podcasts.newest_first")}
      </button>
      <SegmentedControl variant="discret" label={$t("podcasts.episodes_title")} value={filtre} onchange={(v) => (filtre = v as Filtre)}
                        options={[
                          { value: "all", label: $t("podcasts.filter_all") },
                          { value: "unplayed", label: $t("podcasts.filter_unplayed") },
                        ]} />
    </div>

    {#if visibles.length === 0}
      <p class="py-6 text-sm text-(--rg-mu)">{$t("podcasts.no_unplayed")}</p>
    {:else}
      {#each mois as g (g.cle)}
        <h3 class="mt-6 mb-1 flex items-center gap-3 text-[12px] font-bold uppercase tracking-widest text-(--rg-mu)">
          {g.titre}<span class="flex-1 h-px bg-(--rg-line)"></span>
        </h3>
        <ul class="flex flex-col gap-0.5">
          {#each g.items as e (e.id)}
            <PodcastEpisodeRow episode={e} lecture={playbackStateOf(e)} telechargement={telechargements.has(e.id)}
                               onplay={() => play(e)} ontoggleplayed={() => togglePlayed(e)}
                               onnext={() => podcast && playEpisodeNext(podcast, e)} onenqueue={() => podcast && enqueueEpisode(podcast, e)}
                               ondownload={() => download(e)} ondelete={() => deleteDownload(e)} />
          {/each}
        </ul>
      {/each}
      {#if visibles.length > montres}
        <button type="button" onclick={() => (montres += TRANCHE)}
                class="mt-6 h-10 px-5 rounded-full border cursor-pointer text-[13px] font-semibold transition-colors border-(--rg-bd2) text-(--rg-tx) hover:bg-(--rg-s2)">
          {$t("podcasts.show_more_episodes").replace("{n}", formatNumber(Math.min(TRANCHE, visibles.length - montres)))}
        </button>
      {/if}
    {/if}
  </div>
</DetailPage>
{/if}
