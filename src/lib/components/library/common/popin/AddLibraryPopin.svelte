<script lang="ts">
  import { messageErreur } from "#lib/helper/tools/errorTools";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { libraryStore } from "#lib/stores/library/library.store";
  import { profilSelector } from "#lib/stores/profil/profil.store";
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";

  let name = $state("");
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
      const profilId = $profilSelector.profilSelected?.id;
      if (!profilId) throw new Error("NO_PROFIL");

      await libraryStore.addLibrary(profilId, name, description || null);
      close();
    } catch (error) {
      const message = messageErreur(error);
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

<div class="space-y-5">

  <!-- Preview -->
  <div class="flex items-center gap-4 p-3 rounded-xl
              bg-emerald-500/5 dark:bg-emerald-500/8">
    <div class="w-12 h-12 rounded-xl shrink-0 flex items-center justify-center
                bg-emerald-500/15 border border-emerald-500/20">
      <Icon icon="lucide:library" width="20" height="20"
            class="text-emerald-500" />
    </div>
    <div class="min-w-0">
      <p class="text-sm font-semibold text-neutral-900 dark:text-neutral-100 truncate">
        {name.trim() || $t('library.create_library')}
      </p>
      <p class="text-xs text-neutral-500 dark:text-neutral-400">
        {$t("home.tracks_n").replace("{n}", "0")} · {$t("nav.library")}
      </p>
    </div>
  </div>

  <!-- Séparateur -->
  <div class="h-px bg-neutral-200 dark:bg-neutral-700/50"></div>

  <!-- Nom -->
  <div class="flex flex-col gap-1.5">
    <label for="lib_name" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("popins.library_name")} <span class="text-red-500">*</span>
    </label>
    <input
      id="lib_name"
      class="w-full rounded-xl border px-4 py-3 text-sm
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             transition-all duration-200
             focus:outline-none focus:ring-2 focus:border-transparent
             {errorName
               ? 'border-red-400/60 focus:ring-red-500/40'
               : 'border-neutral-200 dark:border-neutral-700 focus:ring-emerald-500/40 hover:border-neutral-300 dark:hover:border-neutral-600'}"
      placeholder={$t("popins.library_name_placeholder")}
      bind:value={name}
      oninput={resetErrors}
      disabled={isSubmitting}
    />
    {#if errorName}
      <p class="flex items-center gap-1.5 text-xs text-red-500">
        <Icon icon="heroicons:exclamation-circle" class="w-3.5 h-3.5 shrink-0" />
        {$t(errorName)}
      </p>
    {/if}
  </div>

  <!-- Description -->
  <div class="flex flex-col gap-1.5">
    <label for="lib_desc" class="text-[11px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
      {$t("forms.description")} <span class="normal-case font-normal tracking-normal">{$t("forms.optional")}</span>
    </label>
    <textarea id="lib_desc"
      class="w-full rounded-xl border px-4 py-3 text-sm resize-none
             bg-neutral-50 dark:bg-neutral-800/80
             text-neutral-900 dark:text-neutral-100
             placeholder-neutral-400 dark:placeholder-neutral-500
             border-neutral-200 dark:border-neutral-700
             transition-all duration-200
             hover:border-neutral-300 dark:hover:border-neutral-600
             focus:outline-none focus:ring-2 focus:ring-emerald-500/40 focus:border-transparent"
      placeholder={$t("forms.description_placeholder")}
      rows="2"
      bind:value={description}
      disabled={isSubmitting}
    ></textarea>
  </div>

  <!-- Erreur globale -->
  {#if globalError}
    <div class="flex items-center gap-2.5 rounded-xl border
                bg-red-50 border-red-200 dark:bg-red-900/15 dark:border-red-800/50
                px-4 py-3">
      <Icon icon="heroicons:exclamation-triangle" class="w-4 h-4 shrink-0 text-red-500" />
      <p class="text-sm text-red-600 dark:text-red-400">{$t(globalError)}</p>
    </div>
  {/if}

  <!-- Actions -->
  <div class="flex justify-end gap-3 pt-1">
    <button
      type="button"
      class="px-4 py-2 rounded-lg text-sm font-medium cursor-pointer
            text-neutral-500 dark:text-neutral-400
            hover:text-neutral-700 dark:hover:text-neutral-200
            hover:bg-neutral-100 dark:hover:bg-neutral-800
            transition-all duration-150
            disabled:opacity-50 disabled:cursor-not-allowed"
      onclick={close}
      disabled={isSubmitting}
    >
      {$t('common.cancel')}
    </button>

    <button
      type="button"
      class="flex items-center gap-2 px-5 py-2 rounded-lg text-sm font-semibold cursor-pointer
            bg-emerald-600 text-white shadow-sm shadow-emerald-900/20
            hover:bg-emerald-500 hover:shadow-md hover:shadow-emerald-900/25
            active:bg-emerald-700 active:shadow-none active:scale-[0.97]
            transition-all duration-150
            disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
      onclick={submit}
      disabled={isSubmitting}
    >
      {#if isSubmitting}
        <Icon icon="lucide:loader-2" width="14" height="14" class="animate-spin" />
        {$t('common.loading')}
      {:else}
        <Icon icon="lucide:plus" width="14" height="14" />
        {$t('library.create_library')}
      {/if}
    </button>
  </div>
</div>
