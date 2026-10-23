<script lang="ts">
  // Petit nuage d'un morceau ou d'un album venu d'un serveur : bleu, rouge quand le serveur ne répond pas.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { serverStore } from "#lib/stores/mediaServer/mediaServer.store";
  import { offlineStore } from "#lib/stores/mediaServer/offline.store";
  import { remoteServerId } from "#lib/types/db/library/Library";
  import { currentSource } from "#lib/stores/source/source.store";

  let { path = null, libraryId = null, serverId: server = null, size = 14, label = false, class: classes = "" }: {
    /** Chemin d'un morceau (`remote://…`). */
    path?: string | null;
    /** Ou la bibliothèque (album, artiste) : distante si elle a un serveur. */
    libraryId?: number | null;
    /** Ou le serveur lui-même (playlist d'un serveur). */
    serverId?: number | null;
    size?: number;
    /** Le nom du serveur à côté (en-têtes). */
    label?: boolean;
    class?: string;
  } = $props();

  const serverId = $derived(
    server ?? remoteServerId(path) ?? (libraryId != null ? $libraryStore.libraries.find((l) => l.id === libraryId)?.server_id ?? null : null),
  );
  const name = $derived(serverId == null ? "" : $libraryStore.libraries.find((l) => l.server_id === serverId)?.server_name ?? "");
  const kept = $derived(!!path && $offlineStore.ready.has(path));
  const pending = $derived(!!path && $offlineStore.pending.has(path));
  // Serveur injoignable : rouge, sauf un morceau gardé hors ligne (toujours jouable).
  const offline = $derived(serverId != null && $serverStore.online[serverId] === false && !kept);
  const icon = $derived(kept ? "material-symbols:cloud-done-outline-rounded" : pending ? "material-symbols:cloud-download-outline-rounded" : "material-symbols:cloud-outline-rounded");
  // Dans la source de ce serveur, tout vient de lui : seuls « hors ligne » et « en attente » renseignent.
  const visible = $derived(serverId != null && (kept || pending || label || $currentSource !== serverId));
  const title = $derived(
    kept ? $t("servers.offline_kept") : pending ? $t("servers.offline_pending")
      : offline ? $t("servers.offline") : $t("servers.streamed_from").replace("{name}", name),
  );
</script>

{#if visible}
  <span class="shrink-0 inline-flex items-center gap-1.5 align-[-2px] {offline ? 'text-red-500' : 'text-sky-600 dark:text-sky-400'} {classes}"
        {title}>
    <Icon {icon} width={size} />
    {#if label && name}<span class="normal-case tracking-normal font-semibold">{name}</span>{/if}
  </span>
{/if}
