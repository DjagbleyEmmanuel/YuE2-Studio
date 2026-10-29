import { afterEach, describe, expect, it, vi } from 'vitest';
import { audioEditor, closeAudioEditor, editorPage, openAudioEditor, subscribeAudioEditor } from './audioEditor';

afterEach(() => {
  closeAudioEditor();
});

describe('the audio editor panel', () => {
  const audio = 'http://127.0.0.1:8791/v1/library/media/take-1';

  it('opens on the file asked for, with its name for the title', () => {
    openAudioEditor(audio, 'Take one');

    expect(audioEditor()).toEqual({ open: true, src: editorPage(audio), name: 'Take one' });
    expect(audioEditor().src).toContain(`audioUrl=${encodeURIComponent(audio)}`);
    // the page belongs to the studio's own files, so it loads in its origin
    expect(audioEditor().src?.startsWith('/editor/index.html?')).toBe(true);
  });

  it('keeps the page loaded when the panel is put away', () => {
    openAudioEditor(audio, 'Take one');
    closeAudioEditor();

    expect(audioEditor().open).toBe(false);
    // reopening the same file carries on with the page, rather than reloading it
    expect(audioEditor().src).toBe(editorPage(audio));
  });

  it('replaces the page when another file is asked for', () => {
    const other = 'http://127.0.0.1:8791/v1/library/media/take-2';
    openAudioEditor(audio);
    openAudioEditor(other);

    expect(audioEditor().src).toBe(editorPage(other));
    expect(audioEditor().name).toBeNull();
  });

  it('tells the panel when to show and hide', () => {
    const heard = vi.fn();
    const stop = subscribeAudioEditor(heard);

    openAudioEditor(audio);
    closeAudioEditor();
    stop();
    openAudioEditor(audio);

    expect(heard).toHaveBeenCalledTimes(2);
  });
});
