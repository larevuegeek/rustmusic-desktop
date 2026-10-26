<script lang="ts">
  // ListenBrainz : le profil relie son compte (jeton personnel), ses écoutes y sont envoyées.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "#lib/i18n";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import { toasts } from "#lib/stores/ui/toast.store";
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";

  type Status = { user_name: string; now_playing: boolean; pending: number };

  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  let status = $state<Status | null>(null);
  let token = $state("");
  let busy = $state(false);
  let erreur = $state<string | null>(null);

  async function load() {
    if (profilId == null) return;
    status = await invoke<Status | null>("listenbrainz_status", { profilId }).catch(() => null);
  }
  $effect(() => {
    void profilId;
    void load();
  });

  async function connect() {
    if (profilId == null || !token.trim()) return;
    busy = true;
    erreur = null;
    try {
      const user = await invoke<string>("listenbrainz_connect", { profilId, token: token.trim() });
      token = "";
      await load();
      toasts.push({ type: "success", title: "ListenBrainz", message: $t("scrobble.connected_as").replace("{user}", user) });
    } catch (e) {
      erreur = errorMessage(e);
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    if (profilId == null) return;
    await invoke("listenbrainz_disconnect", { profilId });
    status = null;
  }

  async function toggleNowPlaying() {
    if (profilId == null || !status) return;
    const on = !status.now_playing;
    status = { ...status, now_playing: on };
    await invoke("listenbrainz_set_now_playing", { profilId, on });
  }
</script>

<OptionGroup foldKey="online-listenbrainz" title="ListenBrainz" hint={$t("scrobble.group_hint")}>
  {#if status}
    <OptionBlock keywords="listenbrainz scrobble écoutes historique compte" class="flex items-center gap-3.5 px-5 py-4">
      <span class="w-10 h-10 shrink-0 rounded-xl flex items-center justify-center border bg-orange-500/12 border-orange-500/30 text-orange-600 dark:text-orange-300">
        <Icon icon="material-symbols:graphic-eq-rounded" width="22" />
      </span>
      <div class="flex-1 min-w-0">
        <p class="text-[15px] font-semibold truncate text-(--rg-tx)">
          {$t("scrobble.connected_as").replace("{user}", status.user_name)}
        </p>
        <p class="mt-0.5 text-[12.5px] text-(--rg-mu)">
          {status.pending
            ? $t("scrobble.pending").replace("{n}", status.pending.toLocaleString($currentLocale))
            : $t("scrobble.up_to_date")}
          · <a href="https://listenbrainz.org/user/{encodeURIComponent(status.user_name)}/" target="_blank" rel="noreferrer"
               class="font-semibold text-(--rg-gtx) hover:underline underline-offset-2">{$t("scrobble.see_profile")}</a>
        </p>
      </div>
      <GhostButton icon="material-symbols:link-off-rounded" label={$t("scrobble.disconnect")} onclick={disconnect} />
    </OptionBlock>
    <OptionItem title={$t("scrobble.now_playing")} desc={$t("scrobble.now_playing_desc")} keywords="listenbrainz en cours" onclick={toggleNowPlaying}>
      <ToggleSwitch checked={status.now_playing} label={$t("scrobble.now_playing")} onclick={toggleNowPlaying} />
    </OptionItem>
  {:else}
    <OptionBlock keywords="listenbrainz scrobble écoutes historique compte jeton" class="flex flex-col gap-3 px-5 py-4">
      <div>
        <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("scrobble.connect_title")}</p>
        <p class="mt-0.75 text-[13px] text-(--rg-mu) text-pretty">
          {$t("scrobble.connect_desc")}
          <a href="https://listenbrainz.org/settings/" target="_blank" rel="noreferrer"
             class="font-semibold text-(--rg-gtx) hover:underline underline-offset-2">{$t("scrobble.get_token")}</a>
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-2.5">
        <div class="flex-1 min-w-60">
          <FormInput id="lb_token" bind:value={token} mono password placeholder={$t("scrobble.token_placeholder")} disabled={busy}
                     invalid={!!erreur} oninput={() => (erreur = null)} onenter={connect} />
        </div>
        <GhostButton icon="material-symbols:link-rounded" label={$t("scrobble.connect")} busy={busy} disabled={!token.trim()} onclick={connect} />
      </div>
      {#if erreur}
        <p class="flex items-center gap-1.5 text-xs text-red-600 dark:text-red-400"><Icon icon="material-symbols:error-outline-rounded" width="15" />{erreur}</p>
      {/if}
    </OptionBlock>
  {/if}
  <OptionBlock keywords={$t("scrobble.privacy_title")} class="flex gap-3.5 px-5 py-4 leading-[1.25]">
    <Icon icon="material-symbols:public" width="20" class="shrink-0 mt-px text-(--rg-mu)" />
    <div class="min-w-0">
      <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("scrobble.privacy_title")}</p>
      <p class="mt-0.75 text-[13px] text-(--rg-mu) text-pretty">{$t("scrobble.privacy_desc")}</p>
    </div>
  </OptionBlock>
</OptionGroup>
