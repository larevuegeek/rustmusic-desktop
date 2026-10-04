<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, currentLocale } from "$lib/i18n";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { profilSelector } from "$lib/stores/profil/profil.store";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { oublier } from "$lib/stores/mix/mix.store";
  import { PLAYLIST_COLORS, PLAYLIST_ICONS } from "../playlistConfig";
  import RuleGroup from "./RuleGroup.svelte";
  import { PRESETS, type Preset } from "./presets";
  import {
    libelleChamp,
    pruneGroup,
    type FieldOption,
    type Group,
    type Limit,
    type Vocabulary,
  } from "./types";

  /**
   * Composer une playlist intelligente.
   *
   * Le nombre de morceaux retenus est recalculé pendant qu'on écrit les règles.
   * C'est ce qui les rend compréhensibles : « supérieur à » et « au moins »
   * s'expliquent mal, mais un compteur qui passe de 3 000 à 12 ne laisse aucun
   * doute sur ce qu'on vient de demander.
   */
  let {
    playlistId = null,
    mix: mixDemande = false,
  }: {
    /** Renseigné pour modifier une playlist existante. */
    playlistId?: number | null;
    /** Un mix : les mêmes règles, mais un tirage au hasard renouvelé chaque jour. */
    mix?: boolean;
  } = $props();

  const mix = $derived(mixDemande || !!$playlistStore.playlists.find((p) => p.id === playlistId)?.is_mix);
  // Les tailles proposées pour un mix ; le tirage est toujours au hasard.
  const TAILLES = [30, 50, 100];
  const tirage = (n: number | null | undefined): Limit => ({ count: n ?? 50, sort: "random", desc: false });

  let name = $state("");
  let colorIndex = $state(0);
  let iconIndex = $state(0);
  let group = $state<Group>({ match: "all", rules: [] });
  let limit = $state<Limit>({ count: null, sort: null, desc: true });

  let vocabulary = $state<Vocabulary | null>(null);
  let fields = $state<FieldOption[]>([]);
  let chargement = $state(true);
  let enregistrement = $state(false);
  let erreur = $state<string | null>(null);

  let compte = $state<number | null>(null);
  let comptage = $state(false);

  const color = $derived(PLAYLIST_COLORS[colorIndex]);
  const icon = $derived(PLAYLIST_ICONS[iconIndex]);
  const libraryId = $derived($libraryStore.libraries[0]?.id ?? null);

  $effect(() => {
    charger();
  });

  async function charger() {
    chargement = true;
    try {
      const voc = await invoke<Vocabulary>("get_rule_vocabulary");
      vocabulary = voc;

      // Les tags viennent s'ajouter aux champs bâtis. Ceux du recensement
      // seulement : proposer la liste théorique noierait les utiles.
      let tags: FieldOption[] = [];
      if (libraryId != null) {
        try {
          const trouves = await invoke<{ key: string; filled: number }[]>(
            "get_library_tag_keys",
            { libraryId },
          );
          tags = trouves.map((tg) => ({
            key: `tag:${tg.key}`,
            label: tg.key.startsWith("custom:") ? tg.key.slice(7) : tg.key.replace(/_/g, " "),
            kind: "text" as const,
            filled: tg.filled,
          }));
        } catch (e) {
          console.error("Recensement des tags impossible:", e);
        }
      }

      fields = [...voc.fields, ...tags];

      if (playlistId != null) {
        const enregistrees = await invoke<(Group & { limit?: Limit }) | null>(
          "get_smart_playlist_rules",
          { playlistId },
        );
        if (enregistrees) {
          group = { match: enregistrees.match, rules: enregistrees.rules };
          if (enregistrees.limit) limit = enregistrees.limit;
        }
        // Nom, couleur et icône se rechargent aussi.
        //
        // Ne restaurer que le nom laissait l'aperçu afficher la première
        // couleur et la première icône du catalogue — et l'enregistrement les
        // écrivait telles quelles. Ouvrir « Règles » pour changer une condition
        // repeignait donc la playlist au passage, sans que rien ne le dise.
        const existante = $playlistStore.playlists?.find((p) => p.id === playlistId);
        if (existante) {
          name = existante.name;
          const ci = PLAYLIST_COLORS.indexOf(existante.color);
          if (ci >= 0) colorIndex = ci;
          const ii = PLAYLIST_ICONS.findIndex((x) => x.id === existante.icon);
          if (ii >= 0) iconIndex = ii;
        }
      }

      // Un mix tire toujours au hasard, même si ses règles viennent d'ailleurs.
      if (mix) limit = tirage(limit.count);

      if (group.rules.length === 0) {
        group = { match: "all", rules: [{ field: "rating", op: "gte", value: "4" }] };
      }
    } catch (e) {
      erreur = String(e);
    } finally {
      chargement = false;
    }
  }

  // ─── Compteur vivant ───
  //
  // Recompter à chaque frappe enverrait une requête par caractère. Un délai
  // court laisse finir de taper sans qu'on ait l'impression d'attendre.
  let minuterie: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Dépendances explicites : c'est le contenu des règles qui déclenche, pas
    // l'affichage.
    const _ = JSON.stringify(group) + JSON.stringify(limit);
    if (!vocabulary) return;

    if (minuterie) clearTimeout(minuterie);
    minuterie = setTimeout(recompter, 250);

    return () => {
      if (minuterie) clearTimeout(minuterie);
    };
  });

  async function recompter() {
    if (!vocabulary) return;
    comptage = true;
    erreur = null;
    try {
      const nettoyees = pruneGroup(group, vocabulary, fields);
      compte = await invoke<number>("count_smart_playlist", {
        rules: { ...nettoyees, limit: null },
      });
    } catch (e) {
      compte = null;
      erreur = String(e);
    } finally {
      comptage = false;
    }
  }

  /**
   * Nom posé par la dernière recette appliquée, tant qu'on n'y a pas touché.
   *
   * Sans cette mémoire, essayer deux recettes l'une après l'autre laissait le
   * nom de la première sur les règles de la seconde : une playlist appelée
   * « Les plus écoutés » qui contenait en réalité les ajouts récents. Le défaut
   * ne se voyait qu'à l'usage, la playlist ayant l'air correcte partout.
   */
  let nomPoseParRecette = $state<string | null>(null);
  // La dernière recette appliquée, mise en évidence.
  let recette = $state<string | null>(null);

  /**
   * Charge une recette dans le formulaire.
   *
   * Le nom est remplacé s'il est vide, ou s'il vient lui-même d'une recette :
   * dans les deux cas personne n'y tient. Un nom tapé à la main, en revanche,
   * survit — qui écrit « Mon truc » puis clique une recette en veut les règles,
   * pas l'intitulé.
   *
   * Les copies profondes évitent que modifier une règle ensuite ne salisse la
   * recette pour la fois d'après.
   */
  function appliquerRecette(p: Preset) {
    recette = p.key;
    group = structuredClone(p.rules);
    limit = mix ? tirage(limit.count) : { ...p.limit };

    if (!name.trim() || name === nomPoseParRecette) {
      name = $t(p.name);
      nomPoseParRecette = name;
    }

    const ci = PLAYLIST_COLORS.indexOf(p.color);
    if (ci >= 0) colorIndex = ci;
    const ii = PLAYLIST_ICONS.findIndex((x) => x.id === p.icon);
    if (ii >= 0) iconIndex = ii;
  }

  const champsTriables = $derived([
    { key: "random", label: $t("smart.random") },
    ...fields.map((f) => ({ key: f.key, label: libelleChamp(f, $t) })),
  ]);

  async function enregistrer() {
    if (!vocabulary) return;
    if (!name.trim()) {
      erreur = $t("smart.name_required");
      return;
    }

    enregistrement = true;
    erreur = null;
    try {
      const regles = {
        ...pruneGroup(group, vocabulary, fields),
        limit: limit.count || limit.sort ? limit : null,
      };

      if (playlistId != null) {
        await invoke("update_smart_playlist", {
          playlistId,
          name,
          color,
          icon: icon.id,
          rules: regles,
        });
        // Les règles ont changé : le tirage du jour ne vaut plus.
        if (mix) oublier(`perso:${playlistId}`);
      } else {
        const profilId = $profilSelector.profilSelected?.id;
        if (!profilId) throw new Error($t("smart.no_profile"));
        await invoke("create_smart_playlist", {
          profilId,
          name,
          color,
          icon: icon.id,
          rules: regles,
          isMix: mix,
        });
      }

      await playlistStore.refresh();
      popinStore.close();
    } catch (e) {
      erreur = String(e);
    } finally {
      enregistrement = false;
    }
  }
</script>

<div class="flex flex-col h-full max-h-[82vh]">

  {#if chargement}
    <div class="flex-1 flex items-center justify-center py-16">
      <Icon icon="lucide:loader-2" width="22" class="animate-spin text-(--rg-mu)" />
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto scrollbar-app px-7 pt-6 pb-7 space-y-7">

      <!-- Identité : la vignette telle qu'elle apparaîtra, le nom, puis couleur et icône. -->
      <div class="flex items-start gap-5">
        <div class="relative shrink-0 w-24 h-24 rounded-2xl flex items-center justify-center text-white transition-colors"
             style="background: linear-gradient(140deg, {color}, color-mix(in oklab, {color} 40%, black)); box-shadow: 0 14px 34px -10px {color}">
          <Icon icon={icon.id} width="38" />
          <span class="absolute inset-0 rounded-2xl ring-1 ring-inset ring-white/15"></span>
        </div>
        <div class="flex-1 min-w-0 pt-1">
          <input
            type="text"
            bind:value={name}
            placeholder={mix ? $t("mix_view.name_placeholder") : $t("smart.name_placeholder")}
            class="w-full bg-transparent text-[22px] font-extrabold tracking-[-0.01em] text-(--rg-tx) placeholder:text-(--rg-ph)
                   border-b-2 border-(--rg-bd) focus:border-(--rg-g) outline-none pb-1.5 transition-colors"
          />
          <p class="mt-2.5 flex gap-2 text-[13px] leading-snug text-(--rg-mu) text-pretty">
            <Icon icon={mix ? "material-symbols:shuffle-rounded" : "lucide:sparkles"} width="15" class="shrink-0 mt-px text-(--rg-g)" />
            {mix ? $t("mix_view.explain_mix") : $t("mix_view.explain_smart")}
          </p>
        </div>
      </div>

      <div class="grid grid-cols-[auto_1fr] items-center gap-x-5 gap-y-3.5">
        <span class="text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("smart.color")}</span>
        <div class="flex flex-wrap gap-2">
          {#each PLAYLIST_COLORS as c, i (c)}
            <button type="button" onclick={() => (colorIndex = i)} aria-label={$t("smart.color")} aria-pressed={colorIndex === i}
                    class="w-6 h-6 rounded-full cursor-pointer transition-transform hover:scale-110
                           {colorIndex === i ? 'ring-2 ring-offset-2 ring-(--rg-tx) ring-offset-(--rg-carte) scale-110' : ''}"
                    style="background: {c}"></button>
          {/each}
        </div>
        <span class="text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("smart.icon")}</span>
        <div class="flex flex-wrap gap-1">
          {#each PLAYLIST_ICONS as ic, i (ic.id)}
            <button type="button" onclick={() => (iconIndex = i)} aria-label={$t(ic.cle)} title={$t(ic.cle)} aria-pressed={iconIndex === i}
                    class="w-9 h-9 flex items-center justify-center rounded-xl cursor-pointer transition-colors
                           {iconIndex === i ? 'text-white' : 'text-(--rg-mu) hover:bg-(--rg-hover) hover:text-(--rg-tx)'}"
                    style={iconIndex === i ? `background: ${color}` : ""}>
              <Icon icon={ic.id} width="18" />
            </button>
          {/each}
        </div>
      </div>

      <!-- Recettes : un point de départ en un clic, modifiable ensuite. -->
      <section>
        <h3 class="mb-3 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">
          {playlistId == null ? $t("smart.from_preset") : $t("smart.replace_with_preset")}
        </h3>
        <div class="grid grid-cols-4 max-[760px]:grid-cols-2 gap-2">
          {#each PRESETS as p (p.key)}
            <button type="button" onclick={() => appliquerRecette(p)} title={$t(p.hint)}
                    class="group min-w-0 flex flex-col gap-2 p-3 rounded-xl border text-left cursor-pointer transition-colors
                           {recette === p.key ? 'bg-(--rg-creux-on)' : 'border-(--rg-bd) bg-(--rg-creux) hover:border-(--rg-bd2)'}"
                    style={recette === p.key ? `border-color: ${p.color}` : ""}>
              <span class="w-8 h-8 rounded-lg flex items-center justify-center" style="background: color-mix(in oklab, {p.color} 16%, transparent); color: {p.color}">
                <Icon icon={p.icon} width="17" />
              </span>
              <span class="text-[13px] font-semibold leading-tight text-(--rg-tx)">{$t(p.name)}</span>
              <span class="text-[11.5px] leading-snug text-(--rg-mu) line-clamp-2">{$t(p.hint)}</span>
            </button>
          {/each}
        </div>
      </section>

      <!-- Règles -->
      <section>
        <h3 class="mb-3 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("smart.rules")}</h3>
        <div class="p-4 rounded-2xl border border-(--rg-bd) bg-(--rg-creux)">
          <RuleGroup bind:group {fields} vocabulary={vocabulary!} />
        </div>
      </section>

      {#if mix}
        <!-- Taille du tirage : le hasard est imposé, seul le nombre se choisit. -->
        <section>
          <h3 class="mb-3 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("mix_view.size")}</h3>
          <div class="grid grid-cols-3 gap-2">
            {#each TAILLES as n (n)}
              <button type="button" onclick={() => (limit = tirage(n))} aria-pressed={limit.count === n}
                      class="relative h-18 flex flex-col items-center justify-center rounded-xl border cursor-pointer transition-colors
                             {limit.count === n ? 'border-(--rg-g) bg-(--rg-creux-on)' : 'border-(--rg-bd) bg-(--rg-creux) hover:border-(--rg-bd2)'}">
                <span class="text-[22px] font-extrabold leading-none tabular-nums {limit.count === n ? 'text-(--rg-gtx)' : 'text-(--rg-tx)'}">{n}</span>
                <span class="mt-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-(--rg-mu)">{$t("library_head.tracks_n")}</span>
                {#if limit.count === n}
                  <Icon icon="material-symbols:check-circle-rounded" width="18" class="absolute top-2 right-2 text-(--rg-g)" />
                {/if}
              </button>
            {/each}
          </div>
        </section>
      {:else}
        <!-- Tri et coupe -->
        <section>
          <h3 class="mb-3 text-[11px] font-bold uppercase tracking-[0.1em] text-(--rg-mu)">{$t("smart.limit_to")}</h3>
          <div class="flex items-center gap-2 flex-wrap">
            <input
              type="number"
              min="1"
              value={limit.count ?? ''}
              onchange={(e) => limit = { ...limit, count: e.currentTarget.value ? Number(e.currentTarget.value) : null }}
              placeholder="—"
              class="h-9 w-24 px-3 rounded-lg text-sm bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx) outline-none focus:border-(--rg-g)"
            />
            <span class="text-sm text-(--rg-mu)">{$t("smart.tracks_sorted_by")}</span>
            <select
              value={limit.sort ?? ''}
              onchange={(e) => limit = { ...limit, sort: e.currentTarget.value || null }}
              class="h-9 px-3 rounded-lg text-sm cursor-pointer max-w-52 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx)"
            >
              <option value="">{$t("smart.natural_order")}</option>
              {#each champsTriables as c (c.key)}
                <option value={c.key}>{c.label}</option>
              {/each}
            </select>
            {#if limit.sort && limit.sort !== 'random'}
              <button
                type="button"
                onclick={() => limit = { ...limit, desc: !limit.desc }}
                class="h-9 px-3 rounded-lg text-sm cursor-pointer flex items-center gap-1.5 bg-(--rg-champ) border border-(--rg-bd) text-(--rg-tx2) hover:border-(--rg-bd2)"
              >
                <Icon icon={limit.desc ? 'lucide:arrow-down' : 'lucide:arrow-up'} width="14" />
                {limit.desc ? $t("smart.descending") : $t("smart.ascending")}
              </button>
            {/if}
          </div>
          <p class="mt-2 text-xs text-(--rg-mu)">{$t("smart.limit_hint")}</p>
        </section>
      {/if}
    </div>

    <!-- Pied : le compte vivant, et l'enregistrement -->
    <div class="shrink-0 flex items-center justify-between gap-4 px-7 py-4 border-t border-(--rg-bd) bg-(--rg-creux)">
      <div class="min-w-0 flex-1">
        {#if erreur}
          <p class="text-sm text-red-500 truncate" title={erreur}>{erreur}</p>
        {:else if comptage}
          <p class="text-sm text-(--rg-mu) flex items-center gap-2">
            <Icon icon="lucide:loader-2" width="14" class="animate-spin" />
            {$t("smart.counting")}
          </p>
        {:else if compte !== null}
          <p class="flex items-baseline gap-2 text-(--rg-tx)">
            <span class="text-[22px] font-extrabold tabular-nums leading-none">{compte.toLocaleString($currentLocale)}</span>
            <span class="text-sm text-(--rg-tx2)">{$t(new Intl.PluralRules($currentLocale).select(compte) === "one" ? "smart.matching_one" : "smart.matching_n")}</span>
          </p>
          {#if mix && compte === 0}
            <p class="mt-1 text-xs text-(--rg-am)">{$t("mix_view.empty_pool")}</p>
          {:else if mix && limit.count}
            <p class="mt-1 text-xs text-(--rg-mu)">{$t("mix_view.drawn").replace("{n}", Math.min(limit.count, compte).toLocaleString($currentLocale))}</p>
          {:else if limit.count && compte > limit.count}
            <p class="mt-1 text-xs text-(--rg-mu)">{$t("smart.shown").replace("{n}", limit.count.toLocaleString($currentLocale))}</p>
          {/if}
        {/if}
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <button type="button" onclick={() => popinStore.close()}
                class="h-10 px-4 rounded-xl text-sm font-semibold cursor-pointer text-(--rg-tx2) hover:bg-(--rg-hover) hover:text-(--rg-tx) transition-colors">
          {$t("common.cancel")}
        </button>
        <button type="button" onclick={enregistrer} disabled={enregistrement}
                class="h-10 pl-4 pr-5 rounded-xl text-sm font-bold cursor-pointer flex items-center gap-2 bg-(--rg-g) text-(--rg-on-g)
                       hover:brightness-110 transition disabled:opacity-40 disabled:cursor-default">
          <Icon icon={playlistId != null ? "material-symbols:check-rounded" : mix ? "material-symbols:shuffle-rounded" : "lucide:sparkles"} width="18" />
          {playlistId != null ? $t("common.save") : mix ? $t("mix_view.create_cta") : $t("smart.create_cta")}
        </button>
      </div>
    </div>
  {/if}
</div>
