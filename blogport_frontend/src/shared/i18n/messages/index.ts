import type { Locale } from '../locale';
import { en, type Catalogue } from './en';

/**
 * Every catalogue this build carries.
 *
 * §02 asks that "a third locale is a file rather than a refactor" — so adding
 * one means writing `id.ts` and adding a line here. Nothing else changes, and
 * the switcher appears on its own the moment there is a second language to
 * switch to.
 *
 * Deliberately partial: `Locale` names the languages the product intends to
 * speak, and this names the ones it can.
 */
export const catalogues: Partial<Record<Locale, Catalogue>> = { en };

export type { Catalogue };
export { en };
