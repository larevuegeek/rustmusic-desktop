<script lang="ts">
  // Télécommande d'ampli : activation, choix de l'ampli piloté.
  import { t } from "#lib/i18n";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import ToggleSwitch from "#lib/components/ui/input/ToggleSwitch.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import { settingsStore } from "#lib/stores/settings/settings.store";
  import { AMP_BRANDS, ampRemoteStore, type Amp } from "#lib/stores/audio/ampRemote.store";

  const actif = $derived($settingsStore.amp_remote === "true");
  const pilote = $derived($ampRemoteStore.view);

  // Première visite, télécommande active : on cherche les amplis.
  $effect(() => {
    if (actif && !$ampRemoteStore.searched && !$ampRemoteStore.searching) ampRemoteStore.discover();
  });

  // L'ampli piloté reste listé même s'il n'a pas répondu à cette recherche.
  const amplis = $derived.by(() => {
    const liste = [...$ampRemoteStore.found];
    if (pilote && !liste.some((a) => a.id === pilote.amp.id)) liste.unshift(pilote.amp);
    return liste;
  });

  async function selectAmp(amp: Amp | null) {
    try {
      await ampRemoteStore.select(amp);
    } catch (e) {
      console.error("[télécommande] choix :", e);
    }
  }
</script>

<OptionGroup title={$t("amp_remote.group")} hint={$t("amp_remote.group_hint")}>
  {#snippet action()}<Badge tone="amber">{$t("settings.beta_badge")}</Badge>{/snippet}
  <OptionItem
    title={$t("amp_remote.toggle")}
    desc={$t("amp_remote.toggle_desc")}
    keywords="onkyo pioneer integra denon marantz yamaha ampli télécommande remote"
    onclick={() => settingsStore.toggle("amp_remote")}
  >
    <ToggleSwitch checked={actif} label={$t("amp_remote.toggle")} onclick={() => settingsStore.toggle("amp_remote")} />
  </OptionItem>
</OptionGroup>

{#if actif}
  <OptionGroup title={$t("amp_remote.amps")} hint={$t("amp_remote.amps_hint")}>
    {#snippet action()}
      <GhostButton
        icon="material-symbols:refresh-rounded"
        label={$ampRemoteStore.searching ? $t("network_player.searching") : $t("network_player.search")}
        busy={$ampRemoteStore.searching}
        disabled={$ampRemoteStore.searching}
        onclick={() => ampRemoteStore.discover()}
      />
    {/snippet}

    {#if pilote}
      <OptionBlock keywords={pilote.amp.name} class="flex items-center gap-4 px-5 py-4 leading-[1.2]">
        <span class="relative shrink-0 flex w-2.5 h-2.5">
          {#if pilote.state.connected}<span class="absolute inset-0 rounded-full bg-(--rg-g) opacity-50 animate-ping"></span>{/if}
          <span class="relative w-2.5 h-2.5 rounded-full {pilote.state.connected ? 'bg-(--rg-g)' : 'bg-(--rg-am)'}"></span>
        </span>
        <div class="flex-1 min-w-0">
          <p class="text-[15px] font-semibold {pilote.state.connected ? 'text-(--rg-gtx)' : 'text-(--rg-am)'}">
            {(pilote.state.connected ? $t("amp_remote.connected") : $t("amp_remote.unreachable")).replace("{name}", pilote.amp.name)}
          </p>
          <p class="mt-0.75 text-[13px] text-(--rg-mu) truncate">{AMP_BRANDS[pilote.amp.brand]} · {pilote.amp.ip}</p>
        </div>
        <GhostButton icon="material-symbols:link-off-rounded" label={$t("amp_remote.stop")} onclick={() => selectAmp(null)} />
      </OptionBlock>
    {/if}

    {#each amplis.filter((a) => a.id !== pilote?.amp.id) as amp (amp.id)}
      <OptionItem title={amp.name} desc="{AMP_BRANDS[amp.brand]} · {amp.model} · {amp.ip}">
        <GhostButton icon="material-symbols:settings-remote-outline-rounded" label={$t("amp_remote.control")} onclick={() => selectAmp(amp)} />
      </OptionItem>
    {:else}
      {#if !pilote && $ampRemoteStore.searched && !$ampRemoteStore.searching}
        <OptionBlock keywords={$t("amp_remote.none")} class="px-5 py-4 text-[13px] text-(--rg-mu)">{$t("amp_remote.none")}</OptionBlock>
      {/if}
    {/each}
  </OptionGroup>
{/if}
