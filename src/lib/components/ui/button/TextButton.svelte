<script lang="ts">
  // Bouton discret sans cadre (« Effacer les filtres ») : icône, libellé masquable quand la place manque.
  import Icon from "@iconify/svelte";

  let { icon, label, labelClass = "", onclick }: {
    icon: string;
    label: string;
    /** Ex. `@max-3xl:hidden`. */
    labelClass?: string;
    onclick: () => void;
  } = $props();
</script>

<button type="button" title={label} {onclick}
        class="h-8.5 px-2 flex items-center gap-1 text-[13px] font-semibold cursor-pointer text-(--rg-mu) hover:text-(--rg-tx)">
  <Icon {icon} width="17" />
  <span class={labelClass}>{label}</span>
</button>
