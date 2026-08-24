<script lang="ts">
  import Icon from "@iconify/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { popinStore } from "$lib/stores/ui/popin.store";
  import { playlistStore } from "$lib/stores/playlist/playlist.store";
  import { profilSelector } from "$lib/stores/profil/profil.store";
  import { libraryStore } from "$lib/stores/library/library.store";
  import { PLAYLIST_COLORS, PLAYLIST_ICONS } from "../playlistConfig";
  import RuleGroup from "./RuleGroup.svelte";
  import { PRESETS, type Preset } from "./presets";
  import {
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
  }: {
    /** Renseigné pour modifier une playlist existante. */
    playlistId?: number | null;
  } = $props();

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
          tags = trouves.map((t) => ({
            key: `tag:${t.key}`,
            label: t.key.startsWith("custom:") ? t.key.slice(7) : t.key.replace(/_/g, " "),
            kind: "text" as const,
            filled: t.filled,
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
    group = structuredClone(p.rules);
    limit = { ...p.limit };

    if (!name.trim() || name === nomPoseParRecette) {
      name = p.name;
      nomPoseParRecette = p.name;
    }

    const ci = PLAYLIST_COLORS.indexOf(p.color);
    if (ci >= 0) colorIndex = ci;
    const ii = PLAYLIST_ICONS.findIndex((x) => x.id === p.icon);
    if (ii >= 0) iconIndex = ii;
  }

  const champsTriables = $derived([
    { key: "random", label: "au hasard" },
    ...fields.map((f) => ({ key: f.key, label: f.label })),
  ]);

  async function enregistrer() {
    if (!vocabulary) return;
    if (!name.trim()) {
      erreur = "Le nom est obligatoire.";
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
      } else {
        const profilId = $profilSelector.profilSelected?.id;
        if (!profilId) throw new Error("Aucun profil sélectionné.");
        await invoke("create_smart_playlist", {
          profilId,
          name,
          color,
          icon: icon.id,
          rules: regles,
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

<div class="flex flex-col h-full max-h-[80vh]">

  {#if chargement}
    <div class="flex-1 flex items-center justify-center py-16">
      <Icon icon="lucide:loader-2" width="22" class="animate-spin text-neutral-400" />
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto scrollbar-app px-6 py-5 space-y-5">

      <!-- Identité -->
      <div class="flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0"
          style="background: {color}20; border: 1px solid {color}40"
        >
          <Icon icon={icon.id} width="18" style="color: {color}" />
        </div>
        <input
          type="text"
          bind:value={name}
          placeholder="Nom de la playlist"
          class="flex-1 text-sm px-3 py-2 rounded-lg
                 bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                 text-neutral-800 dark:text-neutral-200
                 focus:outline-none focus:border-emerald-400"
        />
      </div>

      <div class="flex flex-wrap gap-1.5">
        {#each PLAYLIST_COLORS as c, i (c)}
          <button
            type="button"
            onclick={() => colorIndex = i}
            class="w-5 h-5 rounded-full cursor-pointer transition-transform
                   {colorIndex === i ? 'scale-110 ring-2 ring-offset-1 ring-neutral-400 dark:ring-offset-neutral-900' : ''}"
            style="background: {c}"
            aria-label="Couleur"
          ></button>
        {/each}
      </div>

      <div class="flex flex-wrap gap-1">
        {#each PLAYLIST_ICONS as ic, i (ic.id)}
          <button
            type="button"
            onclick={() => iconIndex = i}
            class="p-1.5 rounded-md cursor-pointer
                   {iconIndex === i
                     ? 'bg-neutral-200 dark:bg-white/10 text-neutral-800 dark:text-neutral-100'
                     : 'text-neutral-400 hover:bg-neutral-100 dark:hover:bg-white/5'}"
            aria-label={ic.id}
          >
            <Icon icon={ic.id} width="15" />
          </button>
        {/each}
      </div>

      <!-- Recettes toutes faites, à la création comme à la modification.
           Je les avais masquées en édition, de peur d'écraser un travail sans
           le dire. C'était une mauvaise protection : quand une playlist porte
           les mauvaises règles — un nom resté d'une recette, des conditions
           venues d'une autre — repartir d'une recette est précisément le geste
           qui la répare, et il était devenu impossible. L'intitulé dit
           maintenant ce qui va se passer. -->
      <div class="pt-3 border-t border-neutral-200/60 dark:border-white/6">
        <h3 class="text-[10px] uppercase tracking-wider text-neutral-400 mb-2">
          {playlistId == null ? "Partir d'une recette" : "Remplacer par une recette"}
        </h3>
        <div class="flex flex-wrap gap-1.5">
          {#each PRESETS as p (p.key)}
            <button
              type="button"
              onclick={() => appliquerRecette(p)}
              title={p.hint}
              class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg cursor-pointer
                     text-[11px] font-medium
                     bg-neutral-100 dark:bg-white/5
                     border border-neutral-200 dark:border-white/10
                     text-neutral-700 dark:text-neutral-300
                     hover:border-neutral-300 dark:hover:border-white/20"
            >
              <Icon icon={p.icon} width="13" style="color: {p.color}" />
              {p.name}
            </button>
          {/each}
        </div>
      </div>

      <!-- Règles -->
      <div class="pt-2 border-t border-neutral-200/60 dark:border-white/6">
        <RuleGroup bind:group {fields} vocabulary={vocabulary!} />
      </div>

      <!-- Tri et coupe -->
      <div class="pt-3 border-t border-neutral-200/60 dark:border-white/6 space-y-2">
        <div class="flex items-center gap-2 flex-wrap">
          <span class="text-xs text-neutral-500 dark:text-neutral-400">Limiter à</span>
          <input
            type="number"
            min="1"
            value={limit.count ?? ''}
            onchange={(e) => limit = { ...limit, count: e.currentTarget.value ? Number(e.currentTarget.value) : null }}
            placeholder="—"
            class="text-xs px-2 py-1.5 rounded-md w-20
                   bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                   text-neutral-800 dark:text-neutral-200 focus:outline-none focus:border-emerald-400"
          />
          <span class="text-xs text-neutral-500 dark:text-neutral-400">morceaux, triés par</span>
          <select
            value={limit.sort ?? ''}
            onchange={(e) => limit = { ...limit, sort: e.currentTarget.value || null }}
            class="text-xs px-2 py-1.5 rounded-md cursor-pointer max-w-44
                   bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-white/10
                   text-neutral-800 dark:text-neutral-200"
          >
            <option value="">ordre naturel</option>
            {#each champsTriables as c (c.key)}
              <option value={c.key}>{c.label}</option>
            {/each}
          </select>
          {#if limit.sort && limit.sort !== 'random'}
            <button
              type="button"
              onclick={() => limit = { ...limit, desc: !limit.desc }}
              class="text-xs px-2 py-1.5 rounded-md cursor-pointer flex items-center gap-1
                     bg-white dark:bg-white/5 border border-neutral-200 dark:border-white/10
                     text-neutral-600 dark:text-neutral-300"
            >
              <Icon icon={limit.desc ? 'lucide:arrow-down' : 'lucide:arrow-up'} width="12" />
              {limit.desc ? 'décroissant' : 'croissant'}
            </button>
          {/if}
        </div>
        <p class="text-[11px] text-neutral-400 dark:text-neutral-500">
          La coupe ne change pas le nombre de morceaux qui répondent aux règles,
          seulement combien s'affichent.
        </p>
      </div>
    </div>

    <!-- Pied : le compte, et l'enregistrement -->
    <div class="shrink-0 flex items-center justify-between gap-4 px-6 py-3
                border-t border-neutral-200/60 dark:border-white/6">
      <div class="min-w-0 flex-1">
        {#if erreur}
          <p class="text-xs text-red-500 truncate" title={erreur}>{erreur}</p>
        {:else if comptage}
          <p class="text-xs text-neutral-400 flex items-center gap-1.5">
            <Icon icon="lucide:loader-2" width="12" class="animate-spin" />
            calcul…
          </p>
        {:else if compte !== null}
          <p class="text-xs text-neutral-600 dark:text-neutral-300">
            <span class="font-semibold tabular-nums">{compte}</span>
            morceau{compte > 1 ? 'x' : ''} correspond{compte > 1 ? 'ent' : ''}
            {#if limit.count && compte > limit.count}
              <span class="text-neutral-400"> — {limit.count} affichés</span>
            {/if}
          </p>
        {/if}
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <button
          type="button"
          onclick={() => popinStore.close()}
          class="text-xs px-3 py-1.5 rounded-lg cursor-pointer
                 text-neutral-600 dark:text-neutral-300
                 hover:bg-neutral-200/60 dark:hover:bg-white/5"
        >
          Annuler
        </button>
        <button
          type="button"
          onclick={enregistrer}
          disabled={enregistrement}
          class="text-xs px-3 py-1.5 rounded-lg cursor-pointer font-medium text-white
                 bg-emerald-500 hover:bg-emerald-400
                 disabled:opacity-40 disabled:cursor-default"
        >
          {playlistId != null ? 'Enregistrer' : 'Créer'}
        </button>
      </div>
    </div>
  {/if}
</div>
