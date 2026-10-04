<script lang="ts">
  import Icon from "@iconify/svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n";
  import { portal } from "$lib/helper/portal";
  import { toasts } from "$lib/stores/ui/toast.store";
  import { settingsStore } from "$lib/stores/settings/settings.store";
  import { pinsStore, isPinned, type PinKind } from "$lib/stores/library/pins.store";

  /** Les options d'une section d'épinglés, sous son « … ». */
  let { kind, libraryId, enCours, x, y, onclose }: {
    kind: PinKind;
    libraryId: number;
    /** L'album ou l'artiste du morceau en cours, s'il est dans cette bibliothèque. */
    enCours: { id: string; titre: string } | null;
    x: number;
    y: number;
    onclose: () => void;
  } = $props();

  const dejaEpingle = $derived(isPinned($pinsStore, kind, enCours?.id));
  const album = $derived(kind === 'album');

  // Aligné à droite sur le bouton. Vers le haut, ancré par le bas juste au-dessus
  // du « … » (y = bas du bouton + 4) : sa hauteur réelle importe peu.
  let menuStyle = $derived.by(() => {
    const largeur = 248;
    const hauteur = 210;
    const posX = Math.max(8, Math.min(x - largeur, window.innerWidth - largeur - 8));
    return y + hauteur > window.innerHeight
      ? `left: ${posX}px; bottom: ${window.innerHeight - y + 38}px;`
      : `left: ${posX}px; top: ${y}px;`;
  });

  // Lu avant `onclose()` : les props viennent d'un état que la fermeture remet à `null`.
  function figer() {
    return { kind, libraryId, enCours, album };
  }

  async function epinglerEnCours() {
    const m = figer();
    onclose();
    if (!m.enCours) return;
    try {
      await pinsStore.setPinned(m.kind, m.libraryId, m.enCours.id, true);
      toasts.push({ type: 'success', title: m.enCours.titre, message: $t('sidebar.pinned') });
    } catch (e) {
      toasts.push({ type: 'error', title: $t('notify.error'), message: String(e) });
    }
  }

  const ligne = "w-full flex items-center gap-2.5 px-3.5 py-2 text-sm text-left cursor-pointer transition-colors disabled:opacity-40 disabled:cursor-default";
</script>

<div use:portal>
<button
  type="button"
  class="fixed inset-0 z-9998 cursor-default"
  onclick={onclose}
  oncontextmenu={(e) => { e.preventDefault(); onclose(); }}
  aria-label={$t("menu.close_menu")}
></button>

<div
  role="menu"
  tabindex="-1"
  oncontextmenu={(e) => e.preventDefault()}
  class="fixed z-[9999] w-62 py-1.5
         bg-(--rg-carte) dark:bg-(--rg-s2) 
         border border-(--rg-bd) dark:border-[#2a312d]
         rounded-xl shadow-[0_18px_40px_rgba(0,0,0,0.18)] dark:shadow-[0_18px_40px_rgba(0,0,0,0.6)] 
         overflow-hidden"
  style={menuStyle}
>
  <button
    class="{ligne} text-(--rg-tx) enabled:hover:bg-green-500/15 enabled:hover:text-green-400"
    onclick={epinglerEnCours}
    disabled={!enCours || dejaEpingle}
    title={enCours?.titre ?? ''}
  >
    <Icon icon="material-symbols:keep-outline-rounded" width="15" class="opacity-60 shrink-0" />
    <span class="truncate">{$t(album ? 'sidebar.pin_playing_album' : 'sidebar.pin_playing_artist')}</span>
  </button>

  <button
    class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]"
    onclick={() => { const m = figer(); onclose(); goto(`/library/${m.libraryId}/${m.album ? 'albums' : 'artists'}`); }}
  >
    <Icon icon={album ? 'material-symbols:album-rounded' : 'material-symbols:mic-external-on-rounded'} width="15" class="opacity-60 shrink-0" />
    {$t(album ? 'sidebar.browse_albums' : 'sidebar.browse_artists')}
  </button>

  <div class="h-px mx-2 my-1 bg-(--rg-bd) dark:bg-white/8"></div>

  <button
    class="{ligne} text-(--rg-tx) hover:bg-(--rg-s2) dark:hover:bg-[#252b28]"
    onclick={() => { const m = figer(); onclose(); settingsStore.set(m.album ? 'show_pinned_albums' : 'show_pinned_artists', 'false'); }}
  >
    <Icon icon="lucide:eye-off" width="14" class="opacity-60 shrink-0" />
    {$t('sidebar.hide_section')}
  </button>
  <!-- La section emporte ce menu : on dit où la retrouver. -->
  <p class="px-3.5 pb-1.5 text-[11px] leading-snug text-(--rg-mu)">{$t('sidebar.hide_section_note')}</p>
</div>
</div>
