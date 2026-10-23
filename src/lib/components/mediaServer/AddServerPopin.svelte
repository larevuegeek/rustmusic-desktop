<script lang="ts">
  // Ajout d'un serveur (OpenSubsonic, Jellyfin ou DLNA) : connexion, puis choix des bibliothèques à ajouter.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { serverStore, type ServerCredentials } from "#lib/stores/mediaServer/mediaServer.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import FormField from "#lib/components/ui/input/FormField.svelte";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import ChoiceTile from "#lib/components/ui/input/ChoiceTile.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import DlnaFolderPicker from "#lib/components/mediaServer/DlnaFolderPicker.svelte";
  import type { DlnaFound, ServerKind, ServerProbe } from "#lib/types/ui/mediaServer/MediaServer";

  let kind = $state<ServerKind>("subsonic");
  const jellyfin = $derived(kind === "jellyfin");
  const dlna = $derived(kind === "dlna");
  let url = $state("");
  let username = $state("");
  let password = $state("");
  let apiKey = $state("");
  let avecCle = $state(false);

  let probe = $state<ServerProbe | null>(null);
  let name = $state("");
  let choisis = $state<string[]>([]);
  /** DLNA : dossiers cochés, pris n'importe où dans l'arborescence. */
  let dossiers = $state<{ id: string; name: string }[]>([]);
  let busy = $state(false);

  // DLNA : les serveurs du réseau, cherchés dès que la tuile est choisie.
  let trouves = $state<DlnaFound[]>([]);
  let recherche = $state(false);
  async function discover() {
    recherche = true;
    try {
      trouves = await serverStore.discover();
    } catch {
      trouves = [];
    } finally {
      recherche = false;
    }
  }
  function choose(k: ServerKind) {
    if (busy) return;
    kind = k;
    erreur = null;
    if (k === "dlna" && !recherche && trouves.length === 0) void discover();
  }
  let erreur = $state<string | null>(null);

  const credentials = (): ServerCredentials => ({
    kind,
    url: url.trim(),
    username: username.trim(),
    password: avecCle ? null : password,
    apiKey: avecCle ? apiKey.trim() : null,
  });

  async function connect() {
    erreur = null;
    const missing = dlna ? false : avecCle ? !apiKey.trim() || (jellyfin && !username.trim()) : !username.trim() || (!jellyfin && !password);
    if (!url.trim() || missing) {
      erreur = $t("servers.fill_fields");
      return;
    }
    busy = true;
    try {
      probe = await serverStore.probe(credentials());
      name = probe.name;
      choisis = dlna ? [] : probe.folders.map((f) => f.id);
      dossiers = [];
    } catch (err) {
      erreur = errorMessage(err);
    } finally {
      busy = false;
    }
  }

  async function add() {
    if (dlna) choisis = dossiers.map((f) => f.id);
    if (!probe || choisis.length === 0) return;
    erreur = null;
    busy = true;
    try {
      const s = await serverStore.add({ ...credentials(), url: probe.url }, name.trim() || null, choisis);
      toasts.push({ type: "success", title: $t("servers.added").replace("{name}", s.name) });
      popinStore.close();
    } catch (err) {
      erreur = errorMessage(err);
    } finally {
      busy = false;
    }
  }

  function toggle(id: string) {
    choisis = choisis.includes(id) ? choisis.filter((x) => x !== id) : [...choisis, id];
  }

  const typeServeur = $derived(
    probe
      ? [
          probe.info.kind ? probe.info.kind.charAt(0).toUpperCase() + probe.info.kind.slice(1) : null,
          probe.info.version?.split(" ")[0],
          kind !== "subsonic" ? null : probe.info.open_subsonic ? "OpenSubsonic" : "Subsonic",
        ].filter(Boolean).join(" · ")
      : "",
  );
</script>

<div class="space-y-5">
  {#if !probe}
    <div class="grid grid-cols-3 gap-2.5 max-sm:grid-cols-1" role="radiogroup" aria-label={$t("servers.kind")}>
      {#each [["subsonic", "servers.kind_subsonic", "servers.intro"], ["jellyfin", "Jellyfin", "servers.intro_jellyfin"], ["dlna", "DLNA", "servers.intro_dlna"]] as [value, title, desc] (value)}
        <ChoiceTile title={value === "subsonic" ? $t(title) : title} desc={$t(desc)} selected={kind === value} onclick={() => choose(value as ServerKind)} />
      {/each}
    </div>

    {#if dlna}
      <!-- Serveurs annoncés sur le réseau -->
      <div class="flex flex-col gap-2">
        <div class="flex items-center gap-2">
          <p class="flex-1 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("servers.dlna_found")}</p>
          <GhostButton icon="material-symbols:refresh-rounded" label={$t("servers.dlna_search_again")} busy={recherche} disabled={busy || recherche} onclick={discover} />
        </div>
        {#if recherche && trouves.length === 0}
          <p class="flex items-center gap-2 text-sm text-(--rg-mu)">
            <Icon icon="material-symbols:progress-activity" width="16" class="animate-spin" />{$t("servers.dlna_searching")}
          </p>
        {:else}
          {#each trouves as s (s.id)}
            <button type="button" disabled={busy} onclick={() => { url = s.location; void connect(); }}
                    class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl border text-left cursor-pointer transition-colors border-(--rg-bd) hover:border-(--rg-gbd) hover:bg-(--rg-gbg)">
              <Icon icon="material-symbols:dns-outline-rounded" width="20" class="shrink-0 text-(--rg-mu)" />
              <span class="flex-1 min-w-0">
                <span class="block truncate text-sm font-semibold text-(--rg-tx)">{s.name}</span>
                <span class="block truncate text-xs text-(--rg-mu)">{[s.model, new URL(s.location).host].filter(Boolean).join(" · ")}</span>
              </span>
              <Icon icon="material-symbols:chevron-right-rounded" width="20" class="shrink-0 text-(--rg-mu)" />
            </button>
          {:else}
            <p class="text-sm text-(--rg-mu)">{$t("servers.dlna_none")}</p>
          {/each}
        {/if}
      </div>
    {/if}

    <FormField label={$t("servers.address")} for="srv_url" required={!dlna}>
      <FormInput id="srv_url" bind:value={url} mono placeholder={dlna ? "http://nas.local:8200" : jellyfin ? "http://nas.local:8096" : "https://music.example.com"} disabled={busy} oninput={() => (erreur = null)} onenter={connect} />
    </FormField>

    {#if dlna}
      <p class="flex gap-2 text-xs leading-relaxed text-(--rg-mu)">
        <Icon icon="material-symbols:info-outline-rounded" width="15" class="shrink-0 mt-0.5" />
        {$t("servers.dlna_address_note")}
      </p>
    {:else if avecCle}
      <div class="grid gap-4 max-sm:grid-cols-1 {jellyfin ? 'grid-cols-2' : ''}">
        {#if jellyfin}
          <FormField label={$t("servers.username")} for="srv_user" required>
            <FormInput id="srv_user" bind:value={username} mono disabled={busy} oninput={() => (erreur = null)} onenter={connect} />
          </FormField>
        {/if}
        <FormField label={$t("servers.api_key")} for="srv_key" required>
          <FormInput id="srv_key" bind:value={apiKey} mono password disabled={busy} oninput={() => (erreur = null)} onenter={connect} />
        </FormField>
      </div>
    {:else}
      <div class="grid grid-cols-2 gap-4 max-sm:grid-cols-1">
        <FormField label={$t("servers.username")} for="srv_user" required>
          <FormInput id="srv_user" bind:value={username} mono disabled={busy} oninput={() => (erreur = null)} onenter={connect} />
        </FormField>
        <FormField label={$t("servers.password")} for="srv_pw" required={!jellyfin}>
          <FormInput id="srv_pw" bind:value={password} password disabled={busy} oninput={() => (erreur = null)} onenter={connect} />
        </FormField>
      </div>
    {/if}

    {#if !dlna}
    <div class="flex flex-wrap items-start gap-x-4 gap-y-2">
      <p class="flex-1 min-w-60 flex gap-2 text-xs leading-relaxed text-(--rg-mu)">
        <Icon icon="material-symbols:lock-outline" width="15" class="shrink-0 mt-0.5" />
        {$t(`servers.${avecCle ? "api_key_note" : "password_note"}${jellyfin ? "_jellyfin" : ""}`)}
      </p>
      <button type="button" onclick={() => { avecCle = !avecCle; erreur = null; }}
              class="text-xs font-semibold cursor-pointer text-(--rg-gtx) hover:underline underline-offset-2">
        {avecCle ? $t("servers.use_password") : $t("servers.use_api_key")}
      </button>
    </div>
    {/if}
  {:else}
    <!-- Le serveur tel qu'il apparaîtra -->
    <div class="flex items-center gap-4 p-3.5 rounded-2xl border border-sky-500/30 bg-linear-135 from-sky-500/10 to-transparent">
      <span class="w-12 h-12 shrink-0 rounded-xl flex items-center justify-center bg-sky-500/15 border border-sky-500/30 text-sky-600 dark:text-sky-300">
        <Icon icon="material-symbols:cloud-outline-rounded" width="26" />
      </span>
      <div class="min-w-0">
        <p class="text-[15px] font-bold truncate text-(--rg-tx)">{name.trim() || probe.name}</p>
        <p class="mt-0.5 text-xs truncate text-(--rg-mu)">{typeServeur} · {probe.url}</p>
      </div>
      <Icon icon="material-symbols:check-circle-rounded" width="22" class="ml-auto shrink-0 text-(--rg-g)" />
    </div>

    <FormField label={$t("servers.name")} for="srv_name">
      <FormInput id="srv_name" bind:value={name} placeholder={probe.name} disabled={busy} onenter={add} />
    </FormField>

    <div class="flex flex-col gap-2">
      <p class="text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t(dlna ? "servers.dlna_choose" : "servers.choose_libraries")}</p>
      {#if dlna}
        <DlnaFolderPicker url={probe.url} bind:selected={dossiers} disabled={busy} />
      {:else if probe.folders.length === 0}
        <p class="text-sm text-(--rg-mu)">{$t("servers.no_folders")}</p>
      {:else}
        <div class="flex flex-col gap-1.5">
          {#each probe.folders as f (f.id)}
            <label class="flex items-center gap-3 px-3.5 py-2.5 rounded-xl border cursor-pointer transition-colors
                          {choisis.includes(f.id) ? 'border-(--rg-gbd) bg-(--rg-gbg)' : 'border-(--rg-bd) hover:border-(--rg-bd2)'}">
              <input type="checkbox" class="checkbox-app" checked={choisis.includes(f.id)} onchange={() => toggle(f.id)} disabled={busy} />
              <Icon icon="material-symbols:cloud-outline-rounded" width="18" class="shrink-0 text-(--rg-mu)" />
              <span class="flex-1 min-w-0 truncate text-sm font-semibold text-(--rg-tx)">{f.name}</span>
            </label>
          {/each}
        </div>
      {/if}
      <p class="text-xs leading-relaxed text-(--rg-mu)">{$t(dlna ? "servers.dlna_choose_hint" : "servers.libraries_note")}</p>
    </div>
  {/if}

  {#if erreur}
    <p class="flex items-center gap-2 px-4 py-3 rounded-xl border border-red-500/25 bg-red-500/8 text-sm text-red-600 dark:text-red-400">
      <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0" />
      {erreur}
    </p>
  {/if}
</div>

<PopinFooter
  bleed
  cancelLabel={probe ? $t("servers.back") : $t("common.cancel")}
  oncancel={() => (probe ? ((probe = null), (erreur = null)) : popinStore.close())}
  cancelDisabled={busy}
  submitLabel={probe ? $t("servers.add_libraries").replace("{n}", String(dlna ? dossiers.length : choisis.length)) : $t("servers.connect")}
  submitIcon={probe ? "material-symbols:add-rounded" : "material-symbols:link-rounded"}
  onsubmit={probe ? add : connect}
  submitDisabled={!!probe && (dlna ? dossiers.length : choisis.length) === 0}
  busy={busy}
/>
