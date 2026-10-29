import React, { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { SlidersHorizontal, X } from 'lucide-react';
import { useI18n } from '../context/I18nContext';
import { named } from '../services/accessibleName';
import { audioEditor, closeAudioEditor, subscribeAudioEditor } from '../services/audioEditor';

/**
 * The audio editor in the studio window.
 *
 * A panel over the content area, so the editor gets the room it needs while
 * the song, the player and the studio's own navigation stay in the same
 * window around it; nothing is torn down or moved out. The page is kept after
 * the first open, hidden, so putting the panel away to look at the studio and
 * opening the same file again carries on with the edit rather than starting
 * over.
 */
export const AudioEditorPanel: React.FC = () => {
  const { t } = useI18n();
  const editor = useSyncExternalStore(subscribeAudioEditor, audioEditor);
  const [everOpened, setEverOpened] = useState(false);
  const page = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    if (editor.open) setEverOpened(true);
  }, [editor.open]);

  useEffect(() => {
    // the keys belong to the editor while it is in front; the page exists only
    // after the first open, so this waits for it
    if (editor.open && everOpened) page.current?.focus();
  }, [editor.open, everOpened]);

  if (!everOpened || !editor.src) return null;

  return (
    <div
      role="region"
      aria-label={t('audioEditor')}
      aria-hidden={!editor.open}
      className={editor.open ? 'absolute inset-0 z-30 flex min-h-0 flex-col bg-white dark:bg-suno' : 'hidden'}
    >
      <div className="flex h-11 shrink-0 items-center gap-2 border-b border-zinc-200 px-3 dark:border-white/10">
        <SlidersHorizontal size={16} className="shrink-0 text-pink-500" />
        <span className="shrink-0 text-sm font-semibold text-zinc-900 dark:text-white">{t('audioEditor')}</span>
        {editor.name && <span className="truncate text-xs text-zinc-500 dark:text-zinc-400">— {editor.name}</span>}
        <button
          type="button"
          onClick={closeAudioEditor}
          {...named(t('close'))}
          className="ml-auto shrink-0 rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-white/10 dark:hover:text-white"
        >
          <X size={18} />
        </button>
      </div>
      <iframe
        ref={page}
        src={editor.src}
        title={editor.name ?? t('audioEditor')}
        allow="microphone; autoplay; clipboard-write"
        className="min-h-0 w-full flex-1 border-0 bg-[#141414]"
      />
    </div>
  );
};
