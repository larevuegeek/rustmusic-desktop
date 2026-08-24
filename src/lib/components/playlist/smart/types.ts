/**
 * Le vocabulaire des règles, tel que le backend le décrit.
 *
 * Ni les champs ni les opérateurs ne sont recopiés ici : les énumérer côté
 * interface ferait qu'un champ ajouté en Rust resterait invisible, ou qu'un
 * opérateur proposé n'existerait pas. Une commande les rend, et cette
 * description ne fait que les typer.
 */

export type FieldKind = "text" | "number" | "date" | "bool";

export type FieldInfo = {
  key: string;
  label: string;
  kind: FieldKind;
};

export type OpInfo = {
  key: string;
  label: string;
  /** Nombre de valeurs à saisir : 0 pour « est vide », 2 pour « entre ». */
  arity: number;
};

export type Vocabulary = {
  fields: FieldInfo[];
  operators: Record<FieldKind, OpInfo[]>;
};

/** Un champ proposable : ceux de la bibliothèque, plus les tags recensés. */
export type FieldOption = FieldInfo & {
  /** Nombre de pistes qui renseignent ce tag. Absent pour les champs bâtis. */
  filled?: number;
};

export type RuleValue = string | number | [number, number] | null;

export type Rule = {
  field: string;
  op: string;
  value: RuleValue;
};

export type Group = {
  match: "all" | "any";
  rules: Node[];
};

export type Node = Rule | Group;

export type Limit = {
  count: number | null;
  sort: string | null;
  desc: boolean;
};

export type SmartRules = Group & { limit?: Limit | null };

/** Opérateurs valides pour le champ d'une règle. */
export function operatorsFor(
  vocabulary: Vocabulary,
  fields: FieldOption[],
  fieldKey: string,
): OpInfo[] {
  // Un tag est toujours du texte : sa valeur est lue dans du JSON, sans type
  // déclaré. Retomber sur « texte » quand le champ est inconnu évite aussi une
  // liste vide si les deux sources se désynchronisent.
  const kind = fields.find((f) => f.key === fieldKey)?.kind ?? "text";
  return vocabulary.operators[kind] ?? vocabulary.operators.text ?? [];
}

export function arityOf(ops: OpInfo[], opKey: string): number {
  return ops.find((o) => o.key === opKey)?.arity ?? 1;
}

/** Une règle neuve, posée sur le premier champ proposé. */
export function defaultRule(fields: FieldOption[]): Rule {
  const f = fields[0];
  return { field: f?.key ?? "title", op: "contains", value: "" };
}

/**
 * Résume des règles en une phrase lisible.
 *
 * Une playlist intelligente affiche son nom et son contenu, mais jamais ce
 * qu'elle demande. Quand les deux divergent — un nom resté d'une recette, des
 * règles venues d'une autre — rien ne le signale : la playlist a l'air correcte
 * partout. Ce résumé est ce qui rend l'écart visible d'un coup d'œil.
 */
export function resumerRegles(
  group: Group,
  vocabulary: Vocabulary,
  fields: FieldOption[],
): string {
  const liant = group.match === "all" ? " et " : " ou ";

  const morceaux = group.rules.map((n) => {
    if ("rules" in n) return `(${resumerRegles(n, vocabulary, fields)})`;

    const champ = fields.find((f) => f.key === n.field);
    const ops = operatorsFor(vocabulary, fields, n.field);
    const op = ops.find((o) => o.key === n.op);

    const nomChamp = champ?.label ?? n.field;
    const nomOp = op?.label ?? n.op;

    if (op?.arity === 0) return `${nomChamp} ${nomOp}`;
    if (Array.isArray(n.value)) return `${nomChamp} entre ${n.value[0]} et ${n.value[1]}`;
    return `${nomChamp} ${nomOp} ${n.value}`;
  });

  return morceaux.join(liant);
}

/**
 * Nettoie les règles avant de les envoyer.
 *
 * Une condition dont la valeur est vide serait refusée par le moteur, et
 * l'utilisateur en a rarement conscience — il vient d'ajouter une ligne qu'il
 * n'a pas remplie. On la retire plutôt que de faire échouer l'enregistrement
 * de tout le reste.
 */
export function pruneGroup(group: Group, vocabulary: Vocabulary, fields: FieldOption[]): Group {
  const gardees: Node[] = [];

  for (const n of group.rules) {
    if ("rules" in n) {
      const sous = pruneGroup(n, vocabulary, fields);
      // Un groupe vidé de ses conditions ne filtrerait rien : le garder
      // reviendrait à ajouter un « et vrai » invisible.
      if (sous.rules.length > 0) gardees.push(sous);
      continue;
    }

    const arite = arityOf(operatorsFor(vocabulary, fields, n.field), n.op);
    if (arite === 0) {
      gardees.push({ ...n, value: null });
    } else if (arite === 2) {
      if (Array.isArray(n.value) && n.value.length === 2) gardees.push(n);
    } else if (typeof n.value === "string" ? n.value.trim() !== "" : n.value != null) {
      gardees.push(n);
    }
  }

  return { match: group.match, rules: gardees };
}
