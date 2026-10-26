<script lang="ts">
  // Serveurs du profil (OpenSubsonic, Jellyfin, DLNA) : chacun avec ses bibliothèques distantes, activable ou retiré.
  import Icon from "@iconify/svelte";
  import { t, currentLocale } from "#lib/i18n";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { invoke } from "@tauri-apps/api/core";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import AddServerPopin from "#lib/components/mediaServer/AddServerPopin.svelte";
  import ServerFoldersPopin from "#lib/components/mediaServer/ServerFoldersPopin.svelte";
  import StatusDot from "#lib/components/ui/status/StatusDot.svelte";
  import SelectField from "#lib/components/ui/input/SelectField.svelte";
  import TypedConfirmDialog from "#lib/components/ui/dialog/TypedConfirmDialog.svelte";
  import { offlineStore } from "#lib/stores/mediaServer/offline.store";
  import { readableSize } from "#lib/helper/tools/sizeTools";
  import type { ServerEntry } from "#lib/types/ui/mediaServer/MediaServer";
  import { remoteServerId } from "#lib/types/db/library/Library";
  import { settingsStore } from "#lib/stores/settings/settings.store";

  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  $effect(() => {
    if (profilId != null && $serverStore.profilId !== profilId) serverStore.load(profilId).then(() => serverStore.check());
  });
  const servers = $derived($serverStore.profilId === profilId ? $serverStore.servers : []);

  // Qualité de lecture : l'original, ou un transcodage fait par le serveur (« mp3-320 »).
  const qualities = $derived([
    { value: "raw", label: $t("servers.quality_original") },
    { value: "mp3-320", label: "MP3 320 kbit/s" },
    { value: "mp3-192", label: "MP3 192 kbit/s" },
    { value: "mp3-128", label: "MP3 128 kbit/s" },
  ]);
  const qualityOf = (s: ServerEntry) => (s.transcode_format ? `${s.transcode_format}-${s.transcode_bitrate ?? 192}` : "raw");
  function setQuality(s: ServerEntry, value: string) {
    const [format, bitrate] = value === "raw" ? [null, null] : [value.split("-")[0], Number(value.split("-")[1])];
    run(() => serverStore.setTranscode(s.id, format, bitrate));
  }

  const openFolders = (s: ServerEntry) =>
    popinStore.open($t("servers.folders_title").replace("{name}", s.name), ServerFoldersPopin, { server: s }, { size: "lg", icon: "material-symbols:folder-open-outline-rounded" });

  const addServer = () =>
    popinStore.open($t("servers.add"), AddServerPopin, {}, { size: "lg", icon: "material-symbols:cloud-outline-rounded" });

  const describe = (s: ServerEntry) =>
    [s.server_type ? s.server_type.charAt(0).toUpperCase() + s.server_type.slice(1) : null, s.server_version?.split(" ")[0], s.username, s.url]
      .filter(Boolean)
      .join(" · ");

  // Synchronise les bibliothèques du serveur, l'une après l'autre (progression dans la barre d'état).
  let synchro = $state<number | null>(null);
  async function sync(s: ServerEntry) {
    synchro = s.id;
    try {
      for (const l of s.libraries) await invoke("rescan_library", { libraryId: l.id });
      await serverStore.load();
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    } finally {
      synchro = null;
    }
  }

  const syncedAt = (s: ServerEntry) =>
    s.last_sync_at ? new Date(s.last_sync_at.replace(" ", "T") + "Z").toLocaleString($currentLocale, { dateStyle: "medium", timeStyle: "short" }) : null;

  /** Serveur à retirer, en attente de confirmation. */
  let retrait = $state<ServerEntry | null>(null);
  let confirmOpen = $state(false);
  const askRemove = (s: ServerEntry) => {
    retrait = s;
    confirmOpen = true;
  };
  async function remove() {
    if (!retrait) return;
    try {
      await serverStore.remove(retrait.id);
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
      throw err;
    }
  }
  // Tous les morceaux hors ligne du serveur, prêts ou en attente.
  async function clearOffline(s: ServerEntry) {
    const paths = [...$offlineStore.ready, ...$offlineStore.pending].filter((p) => remoteServerId(p) === s.id);
    await run(() => offlineStore.remove(paths));
  }
  const offlineLabel = (s: ServerEntry) => {
    const o = $offlineStore.byServer[s.id];
    if (!o) return null;
    const parts = [$t("servers.offline_summary").replace("{n}", (o.ready + o.pending).toLocaleString($currentLocale)).replace("{size}", readableSize(o.bytes, $currentLocale))];
    if (o.pending) parts.push($t("servers.offline_waiting").replace("{n}", o.pending.toLocaleString($currentLocale)));
    return parts.join(" · ");
  };

  const trackCount = (s: ServerEntry) => s.libraries.reduce((n, l) => n + l.total_tracks, 0);
  // Phrase coupée autour de son nom en gras ({b}).
  const boldParts = (cle: string) => [...$t(cle).split("{b}"), ""];

  async function run(action: () => Promise<void>) {
    try {
      await action();
    } catch (err) {
      toasts.push({ type: "error", title: $t("notify.error"), message: String(err) });
    }
  }
</script>

<OptionGroup foldKey="servers.group" title={$t("servers.group")} hint={$t("servers.group_hint")}>
  {#snippet action()}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>{/snippet}

  {#each servers as s (s.id)}
    <OptionBlock keywords="serveur subsonic opensubsonic navidrome jellyfin dlna minidlna {s.name} {s.url}" class="px-5 py-4 flex flex-col gap-3">
      <div class="flex items-center gap-3.5">
        <span class="relative w-10 h-10 shrink-0 rounded-xl flex items-center justify-center border transition-opacity
                     bg-sky-500/12 border-sky-500/30 text-sky-600 dark:text-sky-300 {s.enabled ? '' : 'opacity-45'}">
          <Icon icon="material-symbols:cloud-outline-rounded" width="22" />
        </span>
        <div class="flex-1 min-w-0">
          <p class="text-[15px] font-semibold truncate {s.enabled ? 'text-(--rg-tx)' : 'text-(--rg-mu)'}">{s.name}</p>
          <p class="mt-0.5 text-[12.5px] truncate text-(--rg-mu)" title={describe(s)}>
            {#if s.enabled}<StatusDot online={$serverStore.online[s.id]} class="mr-1.5 -mt-px" />{/if}{describe(s)}
          </p>
        </div>
        {#if s.enabled && s.kind !== "dlna"}
          <span class="max-sm:hidden" title={$t("servers.quality_hint")}>
            <SelectField value={qualityOf(s)} options={qualities} label={$t("servers.quality")} onchange={(v) => setQuality(s, v)} />
          </span>
        {/if}
        <ToggleSwitch checked={s.enabled} label={$t("servers.enabled")} onclick={() => run(() => serverStore.setEnabled(s.id, !s.enabled))} />
      </div>

      <div class="flex flex-wrap items-center gap-1.5 pl-13.5">
        {#each s.libraries as l (l.id)}
          <span class="h-7 px-2.5 inline-flex items-center gap-1.5 rounded-full text-[12.5px] font-medium bg-(--rg-s2) text-(--rg-tx2)">
            <Icon icon="material-symbols:cloud-outline-rounded" width="14" class="text-(--rg-mu)" />{l.name}
            <span class="text-(--rg-mu)">· {l.total_tracks.toLocaleString($currentLocale)}</span>
          </span>
        {/each}
        <span class="flex-1"></span>
        {#if s.enabled}
          <span class="text-[12px] text-(--rg-mu) max-md:hidden">
            {syncedAt(s) ? $t("servers.synced_at").replace("{date}", syncedAt(s) ?? "") : $t("servers.never_synced")}
          </span>
          <GhostButton icon="material-symbols:create-new-folder-outline-rounded" label={$t("servers.folders")} onclick={() => openFolders(s)} />
          <GhostButton icon="material-symbols:sync-rounded" label={$t("servers.sync")} busy={synchro === s.id} disabled={synchro !== null}
                       onclick={() => sync(s)} />
        {/if}
        <GhostButton icon="material-symbols:delete-outline-rounded" label={$t("servers.remove")} onclick={() => askRemove(s)} />
      </div>

      <!-- Playlists venues du serveur : épinglées ou non (Navidrome importe un .m3u par album). -->
      {#if s.enabled && s.kind !== "dlna"}
      <div class="flex items-center gap-3 pl-13.5">
        <span class="flex-1 min-w-0">
          <span class="block text-[13px] font-medium text-(--rg-tx)">{$t("servers.pin_playlists")}</span>
          <span class="block text-xs text-(--rg-mu) text-pretty">{$t("servers.pin_playlists_hint")}</span>
        </span>
        <ToggleSwitch checked={s.pin_playlists} label={$t("servers.pin_playlists")} onclick={() => run(() => serverStore.setPinPlaylists(s.id, !s.pin_playlists))} />
      </div>
      {/if}

      {#if offlineLabel(s)}
      <div class="flex flex-wrap items-center gap-2 pl-13.5 text-[12.5px] text-(--rg-tx2)">
        <Icon icon="material-symbols:cloud-done-outline-rounded" width="16" class="text-sky-600 dark:text-sky-400" />
        <span>{offlineLabel(s)}</span>
        <span class="flex-1"></span>
        <GhostButton icon="material-symbols:cloud-off-outline-rounded" label={$t("servers.offline_clear")} onclick={() => clearOffline(s)} />
      </div>
      {/if}
    </OptionBlock>
  {:else}
    <OptionBlock keywords="serveur subsonic opensubsonic navidrome jellyfin dlna minidlna gonic airsonic" class="flex gap-3.5 px-5 py-4 leading-[1.25]">
      <Icon icon="material-symbols:cloud-outline-rounded" width="20" class="shrink-0 mt-px text-(--rg-mu)" />
      <p class="text-[13px] text-(--rg-mu) text-pretty">{$t("servers.empty")}</p>
    </OptionBlock>
  {/each}

  <TypedConfirmDialog bind:open={confirmOpen} title={$t("servers.remove_title")} confirmLabel={$t("servers.remove_title")} onconfirm={remove}>
    {#if retrait}
      {@const intro = boldParts("servers.remove_intro")}
      <p class="leading-relaxed text-(--rg-tx2) text-pretty">{intro[0]}<span class="font-bold text-(--rg-tx)">{retrait.name}</span>{intro[1]}</p>
      <div class="flex flex-wrap gap-1.5">
        {#each retrait.libraries as l (l.id)}
          <span class="h-7 px-2.5 inline-flex items-center gap-1.5 rounded-full text-[12.5px] font-medium bg-(--rg-s2) text-(--rg-tx2)">
            <Icon icon="material-symbols:cloud-outline-rounded" width="14" class="text-(--rg-mu)" />{l.name}
            <span class="text-(--rg-mu)">· {l.total_tracks.toLocaleString($currentLocale)}</span>
          </span>
        {/each}
      </div>
      <ul class="px-4 py-3 rounded-2xl space-y-2 border border-(--rg-bd) bg-(--rg-creux) text-(--rg-tx2)">
        {#each ["catalog", "traces", "playing", "cache"] as item (item)}
          <li class="flex items-start gap-2.5">
            <span class="w-1.5 h-1.5 mt-1.75 rounded-full bg-red-500/70 shrink-0"></span>
            <span>{$t(`servers.remove_item_${item}`).replace("{n}", trackCount(retrait).toLocaleString($currentLocale))}</span>
          </li>
        {/each}
      </ul>
      <p class="flex items-center gap-2 text-xs text-(--rg-mu)">
        <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0" />
        {$t("servers.remove_kept")}
      </p>
    {/if}
  </TypedConfirmDialog>

  {#if servers.length}
    <!-- Vue « Toutes les sources » : quelle version ouvrir quand un résultat existe en double. -->
    <OptionBlock keywords="double doublon version locale serveur toutes les sources" class="flex items-center gap-4 px-5 py-4">
      <span class="flex-1 min-w-0">
        <span class="block text-[14px] font-semibold text-(--rg-tx)">{$t("sources.duplicate")}</span>
        <span class="block text-xs text-(--rg-mu) text-pretty">{$t("sources.duplicate_hint")}</span>
      </span>
      <SelectField value={$settingsStore.duplicate_version} label={$t("sources.duplicate")}
                   options={[{ value: "local", label: $t("sources.duplicate_local") }, { value: "server", label: $t("sources.duplicate_server") }]}
                   onchange={(v) => settingsStore.set("duplicate_version", v)} />
    </OptionBlock>
  {/if}

  <OptionBlock keywords="ajouter serveur subsonic navidrome jellyfin dlna" class="flex justify-end px-5 py-3">
    <GhostButton icon="material-symbols:add-rounded" label={$t("servers.add")} onclick={addServer} />
  </OptionBlock>
</OptionGroup>
