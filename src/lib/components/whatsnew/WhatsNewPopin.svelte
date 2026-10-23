<script lang="ts">
  // Nouveautés de la version : diapos qui défilent, « Ne plus afficher » au lancement.
  import Icon from "@iconify/svelte";
  import { t } from "#lib/i18n";
  import { popinStore } from "#lib/stores/ui/popin.store";
  import { whatsNew } from "#lib/stores/ui/whatsNew.store";
  import { slidesFor, type WhatsNewSlide } from "#lib/config/whatsNew";
  import WhatsNewArt from "./WhatsNewArt.svelte";

  const slides = slidesFor(whatsNew.version);
  let index = $state(0);
  const last = $derived(index === slides.length - 1);

  const goTo = (i: number) => (index = Math.max(0, Math.min(slides.length - 1, i)));
  const next = () => (last ? popinStore.close() : goTo(index + 1));

  function tryIt(s: WhatsNewSlide) {
    popinStore.close();
    s.action?.();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowRight") goTo(index + 1);
    else if (e.key === "ArrowLeft") goTo(index - 1);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="relative flex flex-col" role="region" aria-roledescription="carousel" aria-label={$t("whats_new.title")}>
  <span class="absolute left-4 top-4 z-10 h-7 px-3 inline-flex items-center gap-1.5 rounded-full text-[12px] font-bold text-white bg-black/25 backdrop-blur-sm">
    <Icon icon="material-symbols:campaign-rounded" width="16" />{$t("whats_new.kicker").replace("{version}", whatsNew.version)}
  </span>
  <button type="button" onclick={() => popinStore.close()} aria-label={$t("common.close")} title={$t("common.close")}
          class="absolute right-3 top-3 z-10 w-8.5 h-8.5 rounded-full flex items-center justify-center cursor-pointer transition-colors
                 bg-black/25 text-white hover:bg-black/45 backdrop-blur-sm">
    <Icon icon="material-symbols:close-rounded" width="20" />
  </button>

  <!-- Toutes les diapos sur une piste qui glisse : illustration et texte avancent ensemble. -->
  <div class="overflow-hidden">
    <div class="flex transition-transform duration-500 ease-[cubic-bezier(.2,.8,.2,1)] motion-reduce:transition-none"
         style="transform: translateX(-{index * 100}%)">
      {#each slides as s, i (s.id)}
        <section class="w-full shrink-0 flex flex-col" aria-hidden={i !== index} inert={i !== index}
                 aria-label="{i + 1} / {slides.length}">
          <div class="h-64 max-sm:h-48"><WhatsNewArt slide={s} /></div>
          <div class="px-7 pt-6 pb-2 max-sm:px-5">
            <p class="text-[12px] font-bold uppercase tracking-widest" style="color: {s.colors[1]}">{i + 1} / {slides.length}</p>
            <h2 class="mt-1.5 text-[23px] font-extrabold leading-tight tracking-[-0.01em] text-(--rg-tx)">{$t(`whats_new.slides.${s.id}.title`)}</h2>
            <p class="mt-2 text-[14.5px] leading-relaxed text-(--rg-tx2)">{$t(`whats_new.slides.${s.id}.text`)}</p>
            {#if s.action}
              <button type="button" onclick={() => tryIt(s)}
                      class="mt-4 h-9 pl-3.5 pr-3 inline-flex items-center gap-1.5 rounded-full border cursor-pointer text-[13px] font-bold transition-colors
                             border-(--rg-bd2) text-(--rg-tx) hover:bg-(--rg-s2)">
                {$t("whats_new.try")}<Icon icon="material-symbols:arrow-forward-rounded" width="17" />
              </button>
            {/if}
          </div>
        </section>
      {/each}
    </div>
  </div>

  <div class="flex justify-center gap-1.5 pt-3 pb-1">
    {#each slides as s, i (s.id)}
      <button type="button" onclick={() => goTo(i)} aria-label="{i + 1} / {slides.length}" aria-current={i === index}
              class="h-2 rounded-full cursor-pointer transition-all duration-300 {i === index ? 'w-6' : 'w-2 bg-(--rg-bd2) hover:bg-(--rg-mu)'}"
              style={i === index ? `background: ${s.colors[0]}` : ""}></button>
    {/each}
  </div>

  <div class="flex flex-wrap items-center gap-3 px-7 max-sm:px-5 py-4 mt-2 border-t border-(--rg-line)">
    <label class="flex items-center gap-2 cursor-pointer text-[13px] text-(--rg-tx2) select-none">
      <input type="checkbox" class="checkbox-app" checked={$whatsNew.hidden} onchange={(e) => whatsNew.setHidden(e.currentTarget.checked)} />
      {$t("whats_new.hide")}
    </label>
    <span class="flex-1"></span>
    {#if index > 0}
      <button type="button" onclick={() => goTo(index - 1)} aria-label={$t("common.previous")} title={$t("common.previous")}
              class="w-10 h-10 rounded-full border flex items-center justify-center cursor-pointer transition-colors border-(--rg-bd2) text-(--rg-tx2) hover:text-(--rg-tx) hover:bg-(--rg-s2)">
        <Icon icon="material-symbols:chevron-left-rounded" width="22" />
      </button>
    {/if}
    <button type="button" onclick={next}
            class="h-10 px-5 inline-flex items-center gap-1.5 rounded-full cursor-pointer text-sm font-bold text-white transition hover:brightness-110"
            style="background: linear-gradient(135deg, {slides[index]?.colors[0]}, {slides[index]?.colors[1]})">
      {last ? $t("whats_new.done") : $t("common.next")}
      {#if !last}<Icon icon="material-symbols:chevron-right-rounded" width="20" />{/if}
    </button>
  </div>
</div>
