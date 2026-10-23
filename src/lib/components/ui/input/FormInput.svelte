<script lang="ts">
  // Saisie de popin (ligne ou zone de texte), habillée comme les réglages.
  let {
    value = $bindable(""),
    id,
    placeholder = "",
    multiline = false,
    rows = 2,
    disabled = false,
    invalid = false,
    password = false,
    mono = false,
    oninput,
    onenter,
  }: {
    value?: string;
    id?: string;
    placeholder?: string;
    multiline?: boolean;
    rows?: number;
    disabled?: boolean;
    invalid?: boolean;
    /** Saisie masquée (mot de passe). */
    password?: boolean;
    /** Adresse, clé : chasse fixe, sans correction. */
    mono?: boolean;
    oninput?: () => void;
    /** Entrée dans un champ d'une ligne. */
    onenter?: () => void;
  } = $props();

  const classes = $derived(
    `w-full px-4 rounded-xl text-sm outline-none transition-colors disabled:opacity-60
     bg-(--rg-champ) text-(--rg-tx) placeholder:text-(--rg-ph) border
     ${invalid ? "border-red-500/60 focus:border-red-500" : "border-(--rg-bd) hover:border-(--rg-bd2) focus:border-(--rg-g)"}`,
  );
</script>

{#if multiline}
  <textarea {id} {placeholder} {rows} {disabled} bind:value {oninput} class="{classes} py-3 resize-none"></textarea>
{:else}
  <input
    {id}
    type={password ? "password" : "text"}
    autocomplete={password ? "current-password" : "off"}
    spellcheck={!(password || mono)}
    {placeholder}
    {disabled}
    bind:value
    {oninput}
    onkeydown={(e) => { if (e.key === "Enter") onenter?.(); }}
    class="{classes} h-11 {mono ? 'font-mono text-[13px]' : ''}"
  />
{/if}
