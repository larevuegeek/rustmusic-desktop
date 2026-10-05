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
  import { libelleMiseAJour } from "#lib/helper/updater/updateStatus";

  let licenceOuverte = $state(false);

  const etat = $derived($updaterState);
  const statut = $derived(libelleMiseAJour($t, etat, $derniereVerification));
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
    <div class="relative">
      <GhostButton
        icon={installer ? "material-symbols:download-rounded" : "material-symbols:sync-rounded"}
        label={installer ? $t("settings.update_install") : $t("about.check_updates")}
        busy={occupe}
        onclick={() => (installer ? downloadAndInstall() : checkForUpdate(false))}
      />
    </div>
  </OptionBlock>
</OptionGroup>

<!-- ─── Technologies ─── -->
<OptionGroup title={$t("about.stack")} hint={$t("about.stack_hint")}>
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
<OptionGroup title={$t("about.license")} hint={$t("about.license_hint")}>
  <OptionItem title={$t("about.license_title")} desc={$t("about.license_summary")} keywords="licence copyright">
    <GhostButton
      icon={licenceOuverte ? "material-symbols:expand-less-rounded" : "material-symbols:description-outline-rounded"}
      label={licenceOuverte ? $t("about.license_hide") : $t("about.license_read")}
      onclick={() => (licenceOuverte = !licenceOuverte)}
    />
  </OptionItem>
  {#if licenceOuverte}
    <OptionBlock keywords="licence" class="px-5 pb-5 pt-4">
      <pre class="whitespace-pre-wrap font-sans text-[13px] leading-relaxed text-(--rg-mu)">Copyright (c) 2026 LaRevueGeeK. Tous droits réservés.

1. AUTORISATION D'UTILISATION
Le logiciel RustMusic est mis à disposition gratuitement pour un usage personnel et non commercial. Vous êtes autorisé à télécharger, installer et utiliser le logiciel sur vos propres appareils.

2. RESTRICTIONS
Il est interdit de :
- Redistribuer, vendre ou sous-licencier le logiciel
- Modifier, décompiler ou désassembler le logiciel
- Utiliser le logiciel à des fins commerciales sans autorisation
- Supprimer ou modifier les mentions de droits d'auteur

3. PROPRIÉTÉ INTELLECTUELLE
Le logiciel, son code source et son interface restent la propriété exclusive de LaRevueGeeK.

4. ABSENCE DE GARANTIE
Le logiciel est fourni "tel quel", sans garantie d'aucune sorte.

5. LOGICIELS TIERS
RustMusic utilise des bibliothèques open-source sous licences MIT, Apache-2.0, BSD, MPL-2.0 et similaires.

6. CONTACT
contact@rustmusic.dev</pre>
    </OptionBlock>
  {/if}
</OptionGroup>

<!-- ─── Crédits ─── -->
<OptionGroup title={$t("about.credits")}>
  <OptionItem title={$t("about.made_with")} desc={$t("about.credits_text")} />
  <OptionItem title={$t("about.icons")} desc={$t("about.icons_credits")} />
</OptionGroup>
