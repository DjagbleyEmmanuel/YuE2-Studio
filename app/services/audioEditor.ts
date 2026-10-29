/**
 * AudioMass, the studio's audio editor.
 *
 * The editor is a page of static files the local service already serves, and
 * it reaches the studio's media through the same address. It opens in a panel
 * over the studio window rather than in a window of its own: the player and
 * everything else the studio was showing stay where they were, and the page
 * keeps its work when the panel is put away.
 */

export interface AudioEditorState {
  /** Whether the panel is in front. Closing it leaves the page loaded behind. */
  open: boolean;
  /** The editor page with the file asked for, or null before the first open. */
  src: string | null;
  /** The file's name, for the panel's title. */
  name: string | null;
}

const CLOSED: AudioEditorState = { open: false, src: null, name: null };

let state: AudioEditorState = CLOSED;
const listeners = new Set<() => void>();

function publish(next: AudioEditorState): void {
  state = next;
  listeners.forEach((listener) => listener());
}

/** The editor page for one audio file, given as an absolute URL. */
export function editorPage(audioUrl: string): string {
  // The page is part of the studio's own files (public/editor is built into the
  // window's assets), so it loads from the studio's origin: the panel can be
  // captured and read like the rest of the interface, and a file the service
  // serves is reached over the loopback address the caller passed.
  return `/editor/index.html?audioUrl=${encodeURIComponent(audioUrl)}`;
}

/** Opens the editor on one audio file, given as an absolute URL. */
export function openAudioEditor(audioUrl: string, name?: string | null): void {
  publish({ open: true, src: editorPage(audioUrl), name: name ?? null });
}

/** Puts the panel away; what was edited on the page stays for the next open. */
export function closeAudioEditor(): void {
  if (!state.open) return;
  publish({ ...state, open: false });
}

/** What the panel shows. A store for useSyncExternalStore, so it is React-free. */
export function audioEditor(): AudioEditorState {
  return state;
}

export function subscribeAudioEditor(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
