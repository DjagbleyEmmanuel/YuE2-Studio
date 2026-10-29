import React, { useCallback, useEffect, useState } from 'react';
import { Check, FolderCog, Loader2, RotateCcw, X } from 'lucide-react';
import { useI18n } from '../context/I18nContext';

/**
 * Where the LoRAs live, and every other model the studio downloads.
 *
 * The LoRAs used to be one folder under the studio's data root, which on Linux
 * is in the home partition: someone with a small disk had no way to answer that
 * from the window, and importing a LoRA from another disk made a second copy of
 * it because a link cannot cross filesystems. The folder is theirs to choose
 * now, and what is already downloaded comes along.
 *
 * Each capability reads its folder once, at start, so a new folder is read at
 * the next start; the window says so and offers the restart instead of letting
 * a download land where it was not asked to.
 */

type FolderUse = { name: string; bytes: number };

type DownloadsFolder = {
  directory: string;
  chosen: string | null;
  default: string;
  free_bytes: number;
  total_bytes: number;
  folders: FolderUse[];
  restart_required: boolean;
};

const CARD = 'rounded-2xl border border-zinc-200 bg-zinc-50 p-4 dark:border-white/10 dark:bg-white/3';
const OUTLINE =
  'inline-flex items-center gap-1.5 rounded-lg border border-zinc-200 px-3 py-1.5 text-xs font-semibold text-zinc-700 hover:border-pink-400 hover:text-pink-600 disabled:cursor-not-allowed disabled:opacity-50 dark:border-white/10 dark:text-zinc-200';
const PRIMARY =
  'inline-flex items-center justify-center gap-2 rounded-lg bg-linear-to-r from-orange-500 to-pink-600 px-4 py-2 text-sm font-bold text-white disabled:cursor-not-allowed disabled:opacity-50';

const bytes = (value: number) => (value < 1024 ** 3 ? `${Math.round(value / 1024 ** 2)} MB` : `${(value / 1024 ** 3).toFixed(1)} GB`);

async function errorMessage(response: Response): Promise<string> {
  const body = (await response.json().catch(() => null)) as { error?: string } | null;
  return body?.error || `HTTP ${response.status}`;
}

/** The studio's own window can reopen itself; a browser tab only reloads. */
function restartStudio() {
  const bridge = (window as unknown as { __TAURI__?: { core?: { invoke?: (command: string) => Promise<unknown> } } }).__TAURI__;
  if (bridge?.core?.invoke) {
    void bridge.core.invoke('restart_studio');
    return;
  }
  window.location.reload();
}

export const DownloadsFolderCard: React.FC<{ compact?: boolean }> = ({ compact = false }) => {
  const { t } = useI18n();
  const tt = t as unknown as (key: string) => string;
  const [folder, setFolder] = useState<DownloadsFolder | null>(null);
  const [picked, setPicked] = useState<{ path: string; free_bytes: number; total_bytes: number } | null>(null);
  const [moveExisting, setMoveExisting] = useState(true);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const response = await fetch('/v1/downloads/directory');
    if (!response.ok) return;
    setFolder((await response.json()) as DownloadsFolder);
  }, []);

  useEffect(() => {
    void load().catch(() => undefined);
  }, [load]);

  const choose = async () => {
    if (busy) return;
    setNote(null);
    const response = await fetch('/v1/downloads/pick-directory', { method: 'POST' });
    if (!response.ok) {
      setError(await errorMessage(response));
      return;
    }
    const body = (await response.json()) as { picked: boolean; path: string | null; free_bytes: number; total_bytes: number };
    if (!body.picked || !body.path || body.path === folder?.directory) return;
    setPicked({ path: body.path, free_bytes: body.free_bytes, total_bytes: body.total_bytes });
    setMoveExisting(true);
  };

  const used = folder ? folder.folders.reduce((sum, entry) => sum + entry.bytes, 0) : 0;

  const apply = async (directory: string | null) => {
    if (busy) return;
    setBusy(true);
    setNote(null);
    setError(null);
    try {
      const response = await fetch('/v1/downloads/directory', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ directory, move_existing: moveExisting }),
      });
      if (!response.ok) {
        setError(await errorMessage(response));
        return;
      }
      const body = (await response.json()) as {
        report: { directory: string; moved_bytes: number; moved: FolderUse[]; failed: { file: string; error: string }[] } | null;
        status: DownloadsFolder;
        restart_required: boolean;
      };
      setPicked(null);
      setFolder(body.status);
      setNote(
        [
          body.report && body.report.moved_bytes > 0 ? tt('downloadsFolderMoved').replace('{size}', bytes(body.report.moved_bytes)) : '',
          body.report && body.report.failed.length > 0
            ? tt('downloadsFolderRefused').replace('{files}', body.report.failed.map((file) => file.file).join(', '))
            : '',
          body.restart_required ? tt('downloadsFolderRestart') : '',
        ]
          .filter(Boolean)
          .join(' '),
      );
    } finally {
      setBusy(false);
    }
  };

  if (!folder) return null;

  return (
    <section className={CARD}>
      <div className="flex items-center gap-2 text-sm font-semibold text-zinc-900 dark:text-white">
        <FolderCog size={17} className="text-pink-500" />
        {tt('downloadsFolder')}
      </div>
      {!compact && <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-300">{tt('downloadsFolderIntro')}</p>}
      <p className="mt-2 break-all font-mono text-[11px] text-zinc-600 dark:text-zinc-300">{folder.directory}</p>
      <p className="mt-1 text-[11px] text-zinc-500 dark:text-zinc-400">
        {tt('downloadsFolderSpace').replace('{free}', bytes(folder.free_bytes)).replace('{total}', bytes(folder.total_bytes))}
      </p>
      {used > 0 && (
        <p className="mt-1 text-[11px] leading-4 text-zinc-500 dark:text-zinc-400">
          {tt('downloadsFolderContents').replace(
            '{list}',
            folder.folders
              .filter((entry) => entry.bytes > 0)
              .map((entry) => `${tt(`downloadsPart_${entry.name}`)} ${bytes(entry.bytes)}`)
              .join(' · '),
          )}
        </p>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <button type="button" onClick={() => void choose()} disabled={busy} className={OUTLINE}>
          <FolderCog size={13} />
          {tt('downloadsFolderChange')}
        </button>
        {folder.chosen && (
          <button type="button" onClick={() => void apply(null)} disabled={busy} className={OUTLINE}>
            {tt('downloadsFolderDefault')}
          </button>
        )}
        {folder.restart_required && (
          <button type="button" onClick={restartStudio} className={OUTLINE}>
            <RotateCcw size={13} />
            {tt('downloadsFolderRestartNow')}
          </button>
        )}
      </div>

      {picked && (
        <div className="mt-3 rounded-lg border border-pink-400/40 bg-pink-500/5 p-3">
          <p className="text-[11px] font-medium text-zinc-600 dark:text-zinc-300">{tt('downloadsFolderPicked')}</p>
          <p className="mt-1 break-all font-mono text-[11px] text-zinc-700 dark:text-zinc-200">{picked.path}</p>
          <p className="mt-1 text-[11px] text-zinc-500 dark:text-zinc-400">
            {tt('downloadsFolderSpace').replace('{free}', bytes(picked.free_bytes)).replace('{total}', bytes(picked.total_bytes))}
          </p>
          {used > picked.free_bytes && (
            <p className="mt-1 text-[11px] leading-4 text-amber-600 dark:text-amber-300">
              {tt('downloadsFolderTight')
                .replace('{need}', bytes(used))
                .replace('{free}', bytes(picked.free_bytes))}
            </p>
          )}
          {used > 0 && (
            <label className="mt-2 flex items-start gap-2 text-[11px] leading-4 text-zinc-600 dark:text-zinc-300">
              <input type="checkbox" checked={moveExisting} onChange={(event) => setMoveExisting(event.target.checked)} className="mt-0.5" />
              <span>{tt('downloadsFolderMove').replace('{size}', bytes(used))}</span>
            </label>
          )}
          <div className="mt-3 flex items-center gap-2">
            <button type="button" onClick={() => void apply(picked.path)} disabled={busy} className={PRIMARY}>
              {busy ? <Loader2 size={14} className="animate-spin" /> : <Check size={14} />}
              {busy ? tt('downloadsFolderApplying') : tt('downloadsFolderUse')}
            </button>
            <button type="button" onClick={() => setPicked(null)} disabled={busy} className={`${OUTLINE} border-transparent`}>
              <X size={13} />
              {tt('cancel')}
            </button>
          </div>
        </div>
      )}

      {folder.restart_required && !picked && (
        <p className="mt-2 text-[11px] leading-4 text-amber-600 dark:text-amber-300">{tt('downloadsFolderRestart')}</p>
      )}
      {note && <p className="mt-2 text-[11px] leading-4 text-emerald-600 dark:text-emerald-300">{note}</p>}
      {error && (
        <p role="alert" className="mt-2 text-[11px] leading-4 text-rose-600 dark:text-rose-300">
          {error}
        </p>
      )}
    </section>
  );
};
