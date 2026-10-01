<script lang="ts">
  // Bouton qui affiche le choix courant et ouvre la liste des options (tri…).
  import Icon from "@iconify/svelte";
  import type { Snippet } from "svelte";
  import Menu from "./Menu.svelte";
  import MenuItem from "./MenuItem.svelte";

  let { value, options, onchange, labelClass = "", menuClass = "w-52.5", label, icon, indicator = "material-symbols:expand-more-rounded", fin }: {
    value: string;
    options: { value: string; label: string; icon?: string }[];
    onchange: (value: string) => void;
    /** Ex. `@max-xl:hidden` : seule l'icône reste quand la place manque. */
    labelClass?: string;
    menuClass?: string;
    /** Libellé et icône affichés quand la valeur n'est pas dans la liste (tri par tag…). */
    label?: string;
    icon?: string;
    /** Icône de fin du bouton (chevron, ou sens du tri). */
    indicator?: string;
    /** Entrées ajoutées après un séparateur (« Inverser l'ordre »…). */
    fin?: Snippet<[() => void]>;
  } = $props();

  let open = $state(false);
  const courant = $derived(options.find((o) => o.value === value) ?? null);
  const libelle = $derived(courant?.label ?? label ?? "");
  const icone = $derived(courant?.icon ?? icon);
  const fermer = () => (open = false);
</script>

<div class="relative">
  <button type="button" aria-expanded={open} title={libelle}
          class="h-8.5 pl-3 pr-2.5 flex items-center gap-1.5 rounded-[10px] border text-[13px] font-semibold whitespace-nowrap cursor-pointer transition-colors
                 bg-(--rg-carte) border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2) hover:text-(--rg-tx)"
          onclick={() => (open = !open)}>
    {#if icone}<Icon icon={icone} width="18" />{/if}
    <span class={labelClass}>{libelle}</span>
    <Icon icon={indicator} width="17" />
  </button>
  <Menu bind:open class={menuClass}>
    {#each options as o (o.value)}
      <MenuItem icon={o.icon ?? null} actif={o.value === value} onclick={() => { onchange(o.value); fermer(); }}>{o.label}</MenuItem>
    {/each}
    {#if fin}
      <div class="h-px my-1 bg-(--rg-bd) dark:bg-[#2a312d]"></div>
      {@render fin(fermer)}
    {/if}
  </Menu>
</div>
