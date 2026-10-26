import { describe, expect, it } from 'vitest';
import { STYLE_GLOSSES, STYLE_SUGGESTIONS, styleSuggestions } from './styleSuggestions';

describe('style suggestions', () => {
  it('are style phrases, not tempos, weights or sentence fragments', () => {
    expect(STYLE_SUGGESTIONS.length).toBeGreaterThan(400);
    expect(new Set(STYLE_SUGGESTIONS.map(phrase => phrase.toLowerCase())).size).toBe(STYLE_SUGGESTIONS.length);
    for (const phrase of STYLE_SUGGESTIONS) {
      expect(phrase).not.toMatch(/\bbpm\b|[():]/i);
      expect(phrase).not.toMatch(/^(a|an|and|with) /i);
      expect(phrase.split(' ').length).toBeLessThanOrEqual(5);
    }
  });

  it('glosses every phrase of the list in each language that has a file, and nothing else', () => {
    expect(STYLE_GLOSSES.zh).toBeDefined();
    for (const [language, glosses] of Object.entries(STYLE_GLOSSES)) {
      expect([...glosses!.keys()].sort(), language).toEqual([...STYLE_SUGGESTIONS].sort());
      for (const [phrase, gloss] of glosses!) expect(gloss, `${language}: ${phrase}`).not.toBe('');
    }
  });

  it('shows the gloss beside the English phrase the model reads', () => {
    const zh = styleSuggestions('zh');
    expect(zh.find(item => item.value === 'male vocals')).toEqual({ value: 'male vocals', label: '男声演唱' });
    expect(styleSuggestions('en').every(item => item.label === undefined)).toBe(true);
  });
});
