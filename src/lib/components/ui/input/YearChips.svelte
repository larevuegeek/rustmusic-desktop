<script lang="ts">
  // Puces « Tout » + les dix années d'une décennie ; une année sans album est grisée.
  import { t, currentLocale } from "$lib/i18n";

  let {
    decennie,
    annee,
    total,
    compte,
    label,
    onchoisir,
  }: {
    decennie: number;
    annee: number | null;
    total: number;
    compte: (y: number) => number;
    label: string;
    onchoisir: (y: number | null) => void;
  } = $props();

  const nombre = (n: number) => n.toLocaleString($currentLocale);
  const puce = "h-8 px-3 inline-flex items-center gap-1.5 rounded-full border text-[13px] font-semibold transition-colors";
  const actif = "bg-(--rg-gbg) border-(--rg-gbd) text-(--rg-gtx)";
</script>

<div class="flex flex-wrap gap-1.5" role="radiogroup" aria-label={label}>
  <button type="button" role="radio" aria-checked={annee == null} onclick={() => onchoisir(null)}
          class="{puce} cursor-pointer {annee == null ? actif : 'border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2)'}">
    {$t("years_view.all")}<span class="text-[11px] font-medium opacity-70">{nombre(total)}</span>
  </button>
  {#each Array.from({ length: 10 }, (_, i) => decennie + i) as y (y)}
    {@const n = compte(y)}
    <button type="button" role="radio" aria-checked={annee === y} disabled={n === 0} onclick={() => onchoisir(y)}
            title={n === 0 ? $t("years_view.year_empty") : n === 1 ? $t("years_view.year_album") : $t("years_view.year_albums").replace("{n}", nombre(n))}
            class="{puce} font-mono tabular-nums {annee === y ? actif : 'border-(--rg-bd) text-(--rg-tx2)'}
                   {n === 0 ? 'opacity-35 cursor-default' : 'cursor-pointer hover:border-(--rg-bd2)'}">
      {y}
    </button>
  {/each}
</div>
