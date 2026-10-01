<script lang="ts">
  import { fly } from "svelte/transition";
  import { alphabetNavVisible } from "$lib/stores/ui/alphabetNav.store";

  let {
    availableLetters = new Set<string>(),
    onletter,
    toujours = false,
  }: {
    availableLetters?: Set<string>;
    onletter: (letter: string) => void;
    /** Affiché d'office (la page décide), sans passer par la préférence d'affichage. */
    toujours?: boolean;
  } = $props();

  const LETTERS = ['#', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
                   'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

  let activeLetter = $state<string | null>(null);

  function handleClick(letter: string) {
    if (!availableLetters.has(letter)) return;
    activeLetter = letter;
    onletter(letter);
    setTimeout(() => { activeLetter = null; }, 600);
  }
</script>

{#if toujours || $alphabetNavVisible}
  <!-- Toute la hauteur de la zone : les lettres se tassent plutôt que de déborder sur ce qui est au-dessus. -->
  <div class="absolute right-2.5 inset-y-2 z-20 flex items-center pointer-events-none" transition:fly={{ x: 10, duration: 200 }}>
  <div
    class="pointer-events-auto max-h-full overflow-hidden flex flex-col items-center py-1.5 px-0.75 rounded-xl
           bg-(--rg-carte)/70 backdrop-blur-md shadow-[0_4px_16px_rgba(0,0,0,0.08)] dark:shadow-none"
  >
    {#each LETTERS as letter}
      {@const available = availableLetters.has(letter)}
      {@const active = activeLetter === letter}
      <button
        type="button"
        class="w-5.5 h-4.75 min-h-0 shrink flex items-center justify-center text-[clamp(8px,1.5vh,11px)] leading-none font-bold rounded-[5px] transition-all duration-150
               {active
                 ? 'bg-(--rg-g) text-(--rg-on-g) scale-110'
                 : available
                   ? 'text-(--rg-tx2) hover:bg-(--rg-g) hover:text-(--rg-on-g) cursor-pointer'
                   : 'text-(--rg-bd2) cursor-default'}"
        disabled={!available}
        onclick={() => handleClick(letter)}
      >
        {letter}
      </button>
    {/each}
  </div>
  </div>
{/if}
