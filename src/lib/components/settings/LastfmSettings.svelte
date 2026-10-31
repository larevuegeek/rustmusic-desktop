<script lang="ts">
  // Last.fm : le profil autorise RustMusic dans le navigateur ; ses écoutes y partent ensuite.
  // Clé d'API de RustMusic si cette version en a une, sinon (ou au choix) celle de l'utilisateur.
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy } from "svelte";
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

  type Status = { user_name: string; now_playing: boolean; pending: number; own_key: boolean };

  const profilId = $derived($profilSelector.profilSelected?.id ?? null);
  let status = $state<Status | null>(null);
  let integree = $state(true);
  let propre = $state(false);
  let cle = $state("");
  let secret = $state("");
  let busy = $state(false);
  let attente = $state<string | null>(null); // page d'autorisation ouverte
  let erreur = $state<string | null>(null);
  let confirmer = $state(false);
  let sondage: ReturnType<typeof setInterval> | null = null;
  const CODES = ["unreachable", "bad_key", "expired", "no_key"];
  // Sans clé intégrée, la clé personnelle est obligatoire.
  const avecCle = $derived(propre || !integree);
  const pret = $derived(!avecCle || (cle.trim() !== "" && secret.trim() !== ""));

  async function load() {
    if (profilId == null) return;
    status = await invoke<Status | null>("lastfm_status", { profilId }).catch(() => null);
  }
  $effect(() => {
    void profilId;
    void load();
  });
  invoke<boolean>("lastfm_has_builtin_key").then((b) => (integree = b)).catch(() => (integree = false));

  const traduire = (e: unknown) => {
    const code = errorMessage(e);
    return CODES.includes(code) ? $t(`lastfm.error_${code}`) : code;
  };

  function arreter() {
    if (sondage) clearInterval(sondage);
    sondage = null;
    attente = null;
  }
  onDestroy(() => {
    arreter();
    if (profilId != null) invoke("lastfm_cancel", { profilId }).catch(() => {});
  });

  async function connect() {
    if (profilId == null || !pret) return;
    busy = true;
    erreur = null;
    try {
      const url = await invoke<string>("lastfm_begin", {
        profilId,
        apiKey: avecCle ? cle.trim() : null,
        apiSecret: avecCle ? secret.trim() : null,
      });
      attente = url;
      await openUrl(url);
      // L'utilisateur autorise dans son navigateur : on demande toutes les 3 s, 10 min au plus.
      const debut = Date.now();
      const id = profilId;
      sondage = setInterval(async () => {
        if (Date.now() - debut > 10 * 60_000) {
          arreter();
          erreur = $t("lastfm.error_expired");
          return;
        }
        try {
          const user = await invoke<string | null>("lastfm_finish", { profilId: id });
          if (!user) return;
          arreter();
          cle = secret = "";
          propre = false;
          await load();
          toasts.push({ type: "success", title: "Last.fm", message: $t("scrobble.connected_as").replace("{user}", user) });
        } catch (e) {
          arreter();
          erreur = traduire(e);
        }
      }, 3000);
    } catch (e) {
      erreur = traduire(e);
    } finally {
      busy = false;
    }
  }

  function annuler() {
    arreter();
    if (profilId != null) invoke("lastfm_cancel", { profilId }).catch(() => {});
  }

  // Deux temps : le premier clic prévient (les écoutes en attente partent avec le compte).
  async function disconnect() {
    if (profilId == null) return;
    if (!confirmer) {
      confirmer = true;
      return;
    }
    confirmer = false;
    await invoke("lastfm_disconnect", { profilId });
    status = null;
  }

  async function toggleNowPlaying() {
    if (profilId == null || !status) return;
    const on = !status.now_playing;
    status = { ...status, now_playing: on };
    await invoke("lastfm_set_now_playing", { profilId, on });
  }

  const lien = "font-semibold text-(--rg-gtx) hover:underline underline-offset-2";
</script>

<OptionGroup foldKey="online-lastfm" title="Last.fm" hint={$t("lastfm.group_hint")}>
  {#if status}
    <OptionBlock keywords="last.fm lastfm scrobble écoutes historique compte" class="flex items-center gap-3.5 px-5 py-4">
      <span class="w-10 h-10 shrink-0 rounded-xl flex items-center justify-center border bg-red-500/12 border-red-500/30 text-red-600 dark:text-red-300">
        <Icon icon="simple-icons:lastdotfm" width="20" />
      </span>
      <div class="flex-1 min-w-0">
        <p class="text-[15px] font-semibold truncate text-(--rg-tx)">
          {$t("scrobble.connected_as").replace("{user}", status.user_name)}
        </p>
        {#if confirmer}
          <p class="mt-0.5 text-[12.5px] font-medium text-red-600 dark:text-red-400 text-pretty">
            {status.pending
              ? $t("scrobble.disconnect_pending").replace("{n}", status.pending.toLocaleString($currentLocale))
              : $t("lastfm.disconnect_kept")}
          </p>
        {:else}
          <p class="mt-0.5 text-[12.5px] text-(--rg-mu)">
            {status.pending
              ? $t("scrobble.pending").replace("{n}", status.pending.toLocaleString($currentLocale))
              : $t("scrobble.up_to_date")}
            {#if status.own_key}· {$t("lastfm.own_key_used")}{/if}
            · <a href="https://www.last.fm/user/{encodeURIComponent(status.user_name)}" target="_blank" rel="noreferrer" class={lien}>{$t("scrobble.see_profile")}</a>
          </p>
        {/if}
      </div>
      {#if confirmer}
        <GhostButton label={$t("common.cancel")} onclick={() => (confirmer = false)} />
      {/if}
      <GhostButton icon="material-symbols:link-off-rounded" label={$t("scrobble.disconnect")} danger={confirmer} onclick={disconnect} />
    </OptionBlock>
    <OptionItem title={$t("scrobble.now_playing")} desc={$t("lastfm.now_playing_desc")} keywords="last.fm en cours" onclick={toggleNowPlaying}>
      <ToggleSwitch checked={status.now_playing} label={$t("scrobble.now_playing")} onclick={toggleNowPlaying} />
    </OptionItem>

  {:else if attente}
    <!-- En attente de l'autorisation dans le navigateur. -->
    <OptionBlock keywords="last.fm lastfm autoriser" class="flex items-center gap-3.5 px-5 py-4">
      <Icon icon="material-symbols:progress-activity" width="22" class="shrink-0 animate-spin text-(--rg-gtx)" />
      <div class="flex-1 min-w-0">
        <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("lastfm.waiting_title")}</p>
        <p class="mt-0.5 text-[12.5px] text-(--rg-mu) text-pretty">{$t("lastfm.waiting_desc")}</p>
      </div>
      <GhostButton icon="material-symbols:open-in-new-rounded" label={$t("lastfm.reopen")} onclick={() => attente && openUrl(attente)} />
      <GhostButton label={$t("common.cancel")} onclick={annuler} />
    </OptionBlock>

  {:else}
    <OptionBlock keywords="last.fm lastfm scrobble écoutes historique compte clé api" class="flex flex-col gap-3 px-5 py-4">
      <div>
        <p class="text-[15px] font-semibold text-(--rg-tx)">{$t("lastfm.connect_title")}</p>
        <p class="mt-0.75 text-[13px] text-(--rg-mu) text-pretty">{$t("lastfm.connect_desc")}</p>
      </div>

      {#if avecCle}
        <!-- Clé personnelle : obligatoire sans clé intégrée, sinon au choix. -->
        <div class="rounded-xl border border-(--rg-bd) bg-(--rg-s2)/50 px-4 py-3 flex flex-col gap-2.5">
          {#if !integree}<p class="text-[13px] font-medium text-(--rg-tx2) text-pretty">{$t("lastfm.no_builtin")}</p>{/if}
          <ol class="list-decimal pl-5 text-[13px] text-(--rg-mu) space-y-1 marker:text-(--rg-mu2)">
            <li>
              <a href="https://www.last.fm/api/account/create" target="_blank" rel="noreferrer" class={lien}>{$t("lastfm.step_create")}</a>
              {$t("lastfm.step_create_hint")}
            </li>
            <li>{$t("lastfm.step_paste")}</li>
            <li>{$t("lastfm.step_connect")}</li>
          </ol>
          <div class="flex flex-wrap gap-2.5">
            <div class="flex-1 min-w-52">
              <FormInput id="lfm_key" bind:value={cle} mono placeholder={$t("lastfm.api_key")} disabled={busy} invalid={!!erreur} oninput={() => (erreur = null)} />
            </div>
            <div class="flex-1 min-w-52">
              <FormInput id="lfm_secret" bind:value={secret} mono password placeholder={$t("lastfm.api_secret")} disabled={busy} invalid={!!erreur} oninput={() => (erreur = null)} onenter={connect} />
            </div>
          </div>
        </div>
      {/if}

      <div class="flex flex-wrap items-center gap-2.5">
        <GhostButton icon="material-symbols:link-rounded" label={$t("lastfm.connect")} busy={busy} disabled={!pret} onclick={connect} />
        {#if integree}
          <button type="button" class="text-[13px] {lien} cursor-pointer" onclick={() => { propre = !propre; erreur = null; }}>
            {propre ? $t("lastfm.use_builtin") : $t("lastfm.use_own_key")}
          </button>
        {/if}
      </div>
      {#if erreur}
        <p class="flex items-center gap-1.5 text-xs text-red-600 dark:text-red-400"><Icon icon="material-symbols:error-outline-rounded" width="15" />{erreur}</p>
      {/if}
    </OptionBlock>
  {/if}
</OptionGroup>
