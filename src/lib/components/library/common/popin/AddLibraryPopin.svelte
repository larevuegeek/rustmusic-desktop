<script lang="ts">
  import { errorMessage } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import FormField from "#lib/components/ui/input/FormField.svelte";
  import FormInput from "#lib/components/ui/input/FormInput.svelte";
  import PopinFooter from "#lib/components/ui/popin/PopinFooter.svelte";
  import type { Library } from "#lib/types/db/library/Library";

  // Avec `library` : renommer, sans description.
  let { library = null }: { library?: Library | null } = $props();

  // Valeur de départ seulement : la saisie prend la suite.
  // svelte-ignore state_referenced_locally
  let name = $state(library?.name ?? "");
  let description = $state("");
  let isSubmitting = $state(false);
  let errorName: string | null = $state(null);
  let globalError: string | null = $state(null);

  function close() {
    popinStore.close();
  }

  function resetErrors() {
    errorName = null;
    globalError = null;
  }

  async function submit() {
    resetErrors();
    if (!name.trim()) {
      errorName = "forms.name_required";
      return;
    }
    isSubmitting = true;
    try {
      if (library) {
        if (name.trim() !== library.name) await libraryStore.rename(library, name);
        close();
        return;
      }
      const profilId = $profilSelector.profilSelected?.id;
      if (!profilId) throw new Error("NO_PROFIL");

      await libraryStore.addLibrary(profilId, name, description || null);
      close();
    } catch (error) {
      const message = errorMessage(error);
      if (message.includes("UNIQUE") || message.includes("duplicate")) {
        errorName = "popins.library_name_taken";
      } else if (message === "NO_PROFIL") {
        globalError = "forms.no_profil";
      } else {
        globalError = "forms.error_retry";
      }
    } finally {
      isSubmitting = false;
    }
  }
</script>

<div class="space-y-6">
  <!-- La bibliothèque telle qu'elle apparaîtra -->
  <div class="flex items-center gap-4 p-3.5 rounded-2xl border border-(--rg-gbd) bg-linear-135 from-(--rg-gbg) to-transparent">
    <span class="w-12 h-12 shrink-0 rounded-xl flex items-center justify-center bg-(--rg-gbg) border border-(--rg-gbd) text-(--rg-g)">
      <Icon icon="material-symbols:library-music-outline-rounded" width="24" />
    </span>
    <div class="min-w-0">
      <p class="text-[15px] font-bold truncate text-(--rg-tx)">
        {name.trim() || (library?.name ?? $t('library.create_library'))}
      </p>
      <p class="mt-0.5 text-xs text-(--rg-mu)">
        {$t("home.tracks_n").replace("{n}", String(library?.total_tracks ?? 0))} · {$t("nav.library")}
      </p>
    </div>
  </div>

  <FormField label={$t("popins.library_name")} for="lib_name" required error={errorName ? $t(errorName) : ""}>
    <FormInput id="lib_name" bind:value={name} placeholder={$t("popins.library_name_placeholder")}
               invalid={!!errorName} disabled={isSubmitting} oninput={resetErrors} onenter={submit} />
  </FormField>

  {#if !library}
    <FormField label={$t("forms.description")} for="lib_desc" optional={$t("forms.optional")}>
      <FormInput id="lib_desc" multiline bind:value={description} placeholder={$t("forms.description_placeholder")} disabled={isSubmitting} />
    </FormField>
  {/if}

  {#if globalError}
    <p class="flex items-center gap-2 px-4 py-3 rounded-xl border border-red-500/25 bg-red-500/8 text-sm text-red-600 dark:text-red-400">
      <Icon icon="material-symbols:warning-outline-rounded" width="17" class="shrink-0" />
      {$t(globalError)}
    </p>
  {/if}
</div>

<PopinFooter
  bleed
  cancelLabel={$t("common.cancel")}
  oncancel={close}
  cancelDisabled={isSubmitting}
  submitLabel={library ? $t("selector.rename") : $t("library.create_library")}
  submitIcon={library ? "material-symbols:check-rounded" : "material-symbols:add-rounded"}
  onsubmit={submit}
  busy={isSubmitting}
/>
