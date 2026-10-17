/** Avant 1900, une année est absente ou fantaisiste : l'album passe pour non daté. */
export const estDatee = (y: number | null | undefined): y is number => !!y && y >= 1900;

export const decennieDe = (y: number) => Math.floor(y / 10) * 10;

/** « 80 » avant 2000, « 2010 » après. */
export const courtDecennie = (d: number) => (d >= 1920 && d < 2000 ? String(d % 100) : String(d));

export const teinteDecennie = (d: number) => (((d - 1950) / 10) * 47 + 360) % 360;
