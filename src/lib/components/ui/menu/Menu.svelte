<script lang="ts">
  // Menu ancré sous son bouton (parent en `relative`) : se ferme au clic extérieur et à Échap.
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    align = "right",
    class: classes = "",
    children,
  }: {
    open?: boolean;
    align?: "left" | "right";
    class?: string;
    children: Snippet;
  } = $props();

  let el = $state<HTMLElement | null>(null);

  // Le parent contient le bouton : un clic dessus le laisse basculer le menu lui-même.
  $effect(() => {
    if (!open) return;
    const dehors = (e: MouseEvent) => {
      if (el && !el.parentElement?.contains(e.target as Node)) open = false;
    };
    const echap = (e: KeyboardEvent) => {
      if (e.key === "Escape") open = false;
    };
    document.addEventListener("mousedown", dehors);
    document.addEventListener("keydown", echap);
    return () => {
      document.removeEventListener("mousedown", dehors);
      document.removeEventListener("keydown", echap);
    };
  });
</script>

{#if open}
  <div
    bind:this={el}
    role="menu"
    class="absolute top-full mt-2 z-30 p-1.5 rounded-xl border text-(--rg-tx)
           bg-(--rg-carte) border-(--rg-bd) shadow-[0_18px_40px_rgba(0,0,0,0.18)]
           dark:bg-(--rg-s2) dark:border-[#2a312d] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)]
           {align === 'right' ? 'right-0' : 'left-0'} {classes}"
  >
    {@render children()}
  </div>
{/if}
