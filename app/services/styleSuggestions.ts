/**
 * What the style fields suggest while a style is written. The phrases stay in
 * English, the language YuE2 reads its style in; a window in another language
 * shows each one's gloss beside it, from `data/style_suggestions.<language>.txt`
 * (one `phrase<TAB>gloss` a line). A language without a file shows the phrases alone.
 */

import type { Language } from '../i18n/translations';
import baseText from '../data/style_suggestions.txt?raw';

const glossTexts = import.meta.glob('../data/style_suggestions.*.txt', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

const lines = (text: string) => text.split('\n').map(line => line.replace(/\r$/, '')).filter(line => line.trim().length > 0);

export const STYLE_SUGGESTIONS: string[] = lines(baseText).map(line => line.trim());

/** Each language's glosses by phrase, from the files beside the list. */
export const STYLE_GLOSSES: Partial<Record<Language, Map<string, string>>> = Object.fromEntries(
  Object.entries(glossTexts).map(([path, text]) => {
    const language = path.match(/style_suggestions\.([a-z]+)\.txt$/)?.[1] ?? path;
    const glosses = new Map(lines(text).map(line => {
      const [phrase, gloss = ''] = line.split('\t');
      return [phrase.trim(), gloss.trim()] as [string, string];
    }));
    return [language, glosses];
  }),
);

export interface StyleSuggestion {
  value: string;
  label?: string;
}

export function styleSuggestions(language: Language): StyleSuggestion[] {
  const glosses = STYLE_GLOSSES[language];
  return STYLE_SUGGESTIONS.map(value => {
    const label = glosses?.get(value);
    return label ? { value, label } : { value };
  });
}
