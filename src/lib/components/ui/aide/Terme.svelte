<script module lang="ts">
  import { writable } from "svelte/store";
  // Une seule bulle ouverte à la fois.
  const ouverte = writable<symbol | null>(null);
</script>

<script lang="ts">
  // Terme technique souligné en pointillés ; un clic ouvre sa définition courte.
  // Bulles coupées dans les réglages : le texte reste, sans soulignement.
  import type { Snippet } from "svelte";
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { rechercheReglages } from "$lib/stores/ui/settingsSearch.store";
  import { t } from "$lib/i18n";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { portal } from "$lib/helper/portal";
  import { lienTerme, type TermeId } from "$lib/config/glossaire";

  let { id, children }: { id: TermeId; children?: Snippet } = $props();

  const moi = Symbol();
  const LARGEUR = 300;
  const actif = $derived($settingsStore.show_help_bubbles !== "false");
  const visible = $derived(actif && $ouverte === moi);

  let declencheur = $state<HTMLElement>();
  let bulle = $state<HTMLElement>();
  let place = $state("");

  function ouvrir() {
    if (!declencheur) return;
    const r = declencheur.getBoundingClientRect();
    const x = Math.round(Math.min(Math.max(8, r.left + r.width / 2 - LARGEUR / 2), innerWidth - LARGEUR - 8));
    // Au-dessus quand la place manque en dessous.
    place = r.bottom + 200 > innerHeight
      ? `left:${x}px;bottom:${Math.round(innerHeight - r.top + 6)}px`
      : `left:${x}px;top:${Math.round(r.bottom + 6)}px`;
    ouverte.set(moi);
  }

  function fermer() {
    if ($ouverte === moi) ouverte.set(null);
  }

  function basculer(e: Event) {
    // La ligne ou la tuile qui contient le terme ne doit pas réagir.
    e.stopPropagation();
    e.preventDefault();
    if (visible) fermer();
    else ouvrir();
  }

  function clavier(e: KeyboardEvent) {
    if (e.key !== "Enter" && e.key !== " ") return;
    basculer(e);
    if (visible) tick().then(() => bulle?.querySelector("button")?.focus());
  }

  function enSavoirPlus() {
    fermer();
    rechercheReglages.set("");
    // Déjà sur le guide : l'adresse ne changerait peut-être pas, on fait défiler directement.
    if (location.pathname === "/settings/guide") window.dispatchEvent(new CustomEvent("aide-terme", { detail: `terme-${id}` }));
    else goto(lienTerme(id));
  }

  $effect(() => {
    if (!visible) return;
    const dehors = (e: PointerEvent) => {
      const cible = e.target as Node;
      if (!bulle?.contains(cible) && !declencheur?.contains(cible)) fermer();
    };
    const echap = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      fermer();
      declencheur?.focus();
    };
    // Un défilement décolle la bulle de son mot : on la ferme.
    const defile = (e: Event) => {
      if (!bulle?.contains(e.target as Node)) fermer();
    };
    window.addEventListener("pointerdown", dehors, true);
    window.addEventListener("keydown", echap);
    window.addEventListener("scroll", defile, true);
    window.addEventListener("resize", fermer);
    return () => {
      window.removeEventListener("pointerdown", dehors, true);
      window.removeEventListener("keydown", echap);
      window.removeEventListener("scroll", defile, true);
      window.removeEventListener("resize", fermer);
    };
  });
</script>

{#if actif}<span
    bind:this={declencheur}
    role="button"
    tabindex="0"
    aria-expanded={visible}
    aria-haspopup="dialog"
    class="cursor-help underline decoration-dotted decoration-[1.5px] underline-offset-[3px] decoration-current/45
           hover:decoration-(--rg-g) focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-(--rg-g) rounded-xs
           {visible ? 'decoration-(--rg-g)' : ''}"
    onclick={basculer}
    onkeydown={clavier}
  >{#if children}{@render children()}{:else}{$t(`glossary.${id}.name`)}{/if}</span
>{:else if children}{@render children()}{:else}{$t(`glossary.${id}.name`)}{/if}{#if visible}<div use:portal>
    <div
      bind:this={bulle}
      role="dialog"
      aria-label={$t(`glossary.${id}.name`)}
      class="terme-bulle fixed z-1000 flex flex-col gap-1.5 px-4 pt-3.5 pb-3 rounded-xl border text-left
             bg-(--rg-carte) border-(--rg-bd2) shadow-[0_12px_32px_rgb(0_0_0/0.18)]"
      style="width:{LARGEUR}px;{place}"
    >
      <p class="text-sm font-bold leading-tight text-(--rg-tx)">{$t(`glossary.${id}.name`)}</p>
      <p class="text-[13px] leading-[1.45] font-normal text-(--rg-tx2) text-pretty">{$t(`glossary.${id}.short`)}</p>
      <button
        type="button"
        class="self-start mt-0.5 flex items-center gap-1 text-[13px] font-semibold text-(--rg-gtx) hover:underline cursor-pointer"
        onclick={enSavoirPlus}
      >
        {$t("glossary.more")}<span aria-hidden="true">→</span>
      </button>
    </div>
  </div>{/if}

<style>
  .terme-bulle {
    font-family: 'Figtree Variable', 'Inter Variable', system-ui, sans-serif;
  }
</style>
