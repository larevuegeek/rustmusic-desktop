<script lang="ts">
  // Pied de la navigation des réglages : version et mises à jour.
  import { t } from "#lib/i18n";
  import { derniereVerification, updaterState } from "#lib/stores/updater/updater.store";
  import { checkForUpdate, downloadAndInstall } from "#lib/services/updater/updater.service";
  import { libelleMiseAJour } from "#lib/helper/updater/updateStatus";

  const etat = $derived($updaterState);

  const statut = $derived(libelleMiseAJour($t, etat, $derniereVerification));

  const installer = $derived(etat.kind === "available");
  const occupe = $derived(["checking", "downloading", "installing", "ready"].includes(etat.kind));
</script>

<div class="mt-auto flex items-center gap-2.5 p-3 rounded-xl bg-(--rg-ver) border border-(--rg-verbd)">
  <div class="flex-1 min-w-0 leading-[1.25]">
    <p class="text-[13px] font-semibold text-(--rg-tx) truncate">RustMusic {__APP_VERSION__} {$t("settings.beta")}</p>
    <p class="text-xs truncate {installer ? 'text-(--rg-gtx)' : 'text-(--rg-mu)'}" title={statut}>{statut}</p>
  </div>
  <button
    type="button"
    class="shrink-0 h-7.5 px-2.5 rounded-lg text-xs font-semibold cursor-pointer transition-colors
           disabled:opacity-50 disabled:cursor-default
           {installer
             ? 'bg-(--rg-g) text-white dark:text-(--rg-on-g) hover:brightness-110'
             : 'bg-(--rg-s2) text-(--rg-tx2) hover:text-(--rg-tx)'}"
    disabled={occupe}
    onclick={() => (installer ? downloadAndInstall() : checkForUpdate(false))}
  >
    {installer ? $t("settings.update_install") : $t("settings.update_check")}
  </button>
</div>
