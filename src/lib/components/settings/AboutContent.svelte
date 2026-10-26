<script lang="ts">
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import OptionGroup from "#lib/components/ui/input/OptionGroup.svelte";
  import OptionItem from "#lib/components/ui/input/OptionItem.svelte";
  import OptionBlock from "#lib/components/ui/input/OptionBlock.svelte";
  import GhostButton from "#lib/components/ui/button/GhostButton.svelte";
  import Badge from "#lib/components/ui/text/Badge.svelte";
  import { derniereVerification, updaterState } from "#lib/stores/updater/updater.store";
  import { checkForUpdate, downloadAndInstall } from "#lib/services/updater/updater.service";
  import { updateLabel } from "#lib/helper/updater/updateStatus";
  import { platformFeatures } from "#lib/stores/settings/platform.store";
  import { SUPPORT_LINKS } from "#lib/config/support";

  let licenceOuverte = $state(false);

  // Patreon d'abord : un soutien mensuel est ce qui fait durer le projet.
  const soutiens = $derived([
    { href: SUPPORT_LINKS.patreon, icon: "simple-icons:patreon", label: $t("support.patreon"), principal: true },
    { href: SUPPORT_LINKS.kofi, icon: "simple-icons:kofi", label: $t("support.kofi"), principal: false },
    { href: SUPPORT_LINKS.paypal, icon: "simple-icons:paypal", label: $t("support.paypal"), principal: false },
    { href: SUPPORT_LINKS.github, icon: "lucide:star", label: $t("support.star"), principal: false },
  ]);

  const etat = $derived($updaterState);
  // Flatpak : Flathub tient l'appli à jour, pas de vérification ici.
  const statut = $derived($platformFeatures.flatpak ? $t("settings.update_flathub") : updateLabel($t, etat, $derniereVerification));
  const installer = $derived(etat.kind === "available");
  const occupe = $derived(["checking", "downloading", "installing", "ready"].includes(etat.kind));

  // Versions majeures.mineures de package.json et Cargo.toml.
  const technologies = $derived([
    { categorie: $t("about.stack_app"), libs: [
      { name: "Tauri", version: "2.11", url: "https://tauri.app", icon: "simple-icons:tauri" },
    ] },
    { categorie: $t("about.stack_ui"), libs: [
      { name: "SvelteKit", version: "2.70", url: "https://svelte.dev/docs/kit", icon: "simple-icons:svelte" },
      { name: "Svelte", version: "5.56", url: "https://svelte.dev", icon: "simple-icons:svelte" },
      { name: "TypeScript", version: "6.0", url: "https://typescriptlang.org", icon: "simple-icons:typescript" },
      { name: "Tailwind CSS", version: "4.3", url: "https://tailwindcss.com", icon: "simple-icons:tailwindcss" },
      { name: "Vite", version: "8.1", url: "https://vite.dev", icon: "simple-icons:vite" },
      { name: "KarbonJS UI", version: "0.3", url: "https://npmjs.com/package/@karbonjs/ui-svelte", icon: "material-symbols:deployed-code-outline" },
    ] },
    { categorie: $t("about.stack_engine"), libs: [
      { name: "Rust", version: "2021", url: "https://rust-lang.org", icon: "simple-icons:rust" },
      { name: "SQLx", version: "0.8", url: "https://github.com/launchbadge/sqlx", icon: "material-symbols:database-outline" },
      { name: "SQLite", version: "3", url: "https://sqlite.org", icon: "simple-icons:sqlite" },
      { name: "Tokio", version: "1.52", url: "https://tokio.rs", icon: "material-symbols:bolt-outline-rounded" },
    ] },
    { categorie: $t("about.stack_audio"), libs: [
      { name: "Symphonia", version: "0.6", url: "https://github.com/pdeljanov/Symphonia", icon: "material-symbols:music-note-rounded" },
      { name: "CPAL", version: "0.18", url: "https://github.com/RustAudio/cpal", icon: "material-symbols:speaker-outline-rounded" },
      { name: "Rubato", version: "3.0", url: "https://github.com/HEnquist/rubato", icon: "material-symbols:graphic-eq-rounded" },
      { name: "WASAPI", version: "0.23", url: "https://github.com/HEnquist/wasapi-rs", icon: "material-symbols:settings-input-component-outline-rounded" },
    ] },
  ]);
</script>

<!-- ─── Carte d'identité ─── -->
<OptionGroup>
  <OptionBlock keywords="rustmusic version {$t('about.tagline')} {statut}" class="relative overflow-hidden flex items-center gap-5 p-6 max-sm:flex-col max-sm:items-start">
    <div class="pointer-events-none absolute -top-28 -right-20 w-72 h-72 rounded-full bg-(--rg-g) opacity-12 blur-[80px]" aria-hidden="true"></div>
    <div class="relative flex-1 min-w-0 leading-[1.2]">
      <p class="text-[30px] font-extrabold tracking-[-0.02em] text-(--rg-tx)">Rust<span class="text-(--rg-gtx)">Music</span></p>
      <p class="mt-1.5 text-sm text-(--rg-mu)">{$t("about.tagline")}</p>
      <div class="mt-3.5 flex flex-wrap items-center gap-2">
        <Badge tone="green">v{__APP_VERSION__}</Badge>
        <Badge tone="amber">{$t("settings.beta_badge")}</Badge>
        <span class="text-[13px] {installer ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}">{statut}</span>
      </div>
      <p class="mt-3.5 text-[13px] text-(--rg-mu)">
        {$t("about.developed_by")}
        <a href="https://larevuegeek.fr" target="_blank" rel="noopener noreferrer" class="font-semibold text-(--rg-tx2) hover:text-(--rg-gtx) transition-colors">LaRevueGeeK</a>
      </p>
    </div>
    {#if !$platformFeatures.flatpak}
    <div class="relative">
      <GhostButton
        icon={installer ? "material-symbols:download-rounded" : "material-symbols:sync-rounded"}
        label={installer ? $t("settings.update_install") : $t("about.check_updates")}
        busy={occupe}
        onclick={() => (installer ? downloadAndInstall() : checkForUpdate(false))}
      />
    </div>
    {/if}
  </OptionBlock>
</OptionGroup>

<!-- ─── Soutenir ─── -->
<OptionGroup foldKey="support.about_title" title={$t("support.about_title")} hint={$t("support.about_hint")}>
  <OptionBlock
    keywords="{$t('support.about_title')} patreon ko-fi paypal github don"
    class="relative overflow-hidden p-5"
  >
    <div class="pointer-events-none absolute -top-24 -right-16 w-60 h-60 rounded-full bg-amber-500 opacity-10 blur-[70px]" aria-hidden="true"></div>
    <p class="relative text-[13px] leading-relaxed text-(--rg-mu) max-w-xl">{$t("support.about_text")}</p>
    <div class="relative mt-4 flex flex-wrap gap-2">
      {#each soutiens as lien (lien.href)}
        <a
          href={lien.href}
          target="_blank"
          rel="noopener noreferrer"
          class="h-9 flex items-center gap-2 px-3.5 rounded-lg border transition-colors text-[13px] font-semibold
                 {lien.principal
                   ? 'bg-amber-500 border-amber-500 text-neutral-950 hover:bg-amber-400'
                   : 'bg-(--rg-creux) border-(--rg-line) hover:border-(--rg-bd2) text-(--rg-tx2) hover:text-(--rg-tx)'}"
        >
          <Icon icon={lien.icon} width="15" />
          {lien.label}
        </a>
      {/each}
    </div>
  </OptionBlock>
</OptionGroup>

<!-- ─── Technologies ─── -->
<OptionGroup foldKey="about.stack" title={$t("about.stack")} hint={$t("about.stack_hint")}>
  {#each technologies as groupe (groupe.categorie)}
    <OptionBlock
      keywords="{groupe.categorie} {groupe.libs.map((l) => l.name).join(' ')}"
      class="flex items-start gap-4 px-5 py-4 max-sm:flex-col max-sm:gap-2.5"
    >
      <p class="w-36 shrink-0 pt-1.5 text-[15px] font-semibold leading-[1.2] text-(--rg-tx)">{groupe.categorie}</p>
      <div class="flex-1 min-w-0 flex flex-wrap gap-1.5">
        {#each groupe.libs as lib (lib.name)}
          <a
            href={lib.url}
            target="_blank"
            rel="noopener noreferrer"
            class="h-8 flex items-center gap-1.5 pl-2.5 pr-3 rounded-lg border transition-colors
                   bg-(--rg-creux) border-(--rg-line) hover:border-(--rg-bd2)
                   text-[13px] font-semibold text-(--rg-tx2) hover:text-(--rg-tx)"
          >
            <Icon icon={lib.icon} width="15" class="text-(--rg-mu)" />
            {lib.name}
            <span class="font-mono text-[11px] font-normal text-(--rg-mu2)">{lib.version}</span>
          </a>
        {/each}
      </div>
    </OptionBlock>
  {/each}
</OptionGroup>

<!-- ─── Licence ─── -->
<OptionGroup foldKey="about.license" title={$t("about.license")} hint={$t("about.license_hint")}>
  <OptionItem title={$t("about.license_title")} desc={$t("about.license_summary")} keywords="licence copyright">
    <GhostButton
      icon={licenceOuverte ? "material-symbols:expand-less-rounded" : "material-symbols:description-outline-rounded"}
      label={licenceOuverte ? $t("about.license_hide") : $t("about.license_read")}
      onclick={() => (licenceOuverte = !licenceOuverte)}
    />
  </OptionItem>
  {#if licenceOuverte}
    <OptionBlock keywords="licence" class="px-5 pb-5 pt-4">
      <!-- GPL-3.0, comme le dépôt (LICENSE) : même texte dans toutes les langues de l'appli. -->
      <p class="text-[13px] leading-relaxed text-(--rg-mu)">{$t("about.license_text")}</p>
      <a href="https://www.gnu.org/licenses/gpl-3.0.html" target="_blank" rel="noopener noreferrer"
         class="mt-2 inline-flex items-center gap-1.5 text-[13px] font-semibold text-(--rg-gtx) hover:underline">
        <Icon icon="material-symbols:open-in-new-rounded" width="14" />{$t("about.license_full")}
      </a>
    </OptionBlock>
  {/if}
</OptionGroup>

<!-- ─── Crédits ─── -->
<OptionGroup foldKey="about.credits" title={$t("about.credits")}>
  <OptionItem title={$t("about.made_with")} desc={$t("about.credits_text")} />
  <OptionItem title={$t("about.icons")} desc={$t("about.icons_credits")} />
</OptionGroup>
