import React, { useState } from "react";
import { tauriApi } from "../../lib/tauri";
import { ScreenCaptureStatus } from "../../lib/types";
import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  RefreshCw,
  ShieldAlert,
  RotateCw,
  Info,
} from "lucide-react";

interface PermissionModalProps {
  isOpen: boolean;
  status: ScreenCaptureStatus | null;
  onClose: () => void;
  onRefresh: () => Promise<void> | void;
}

/**
 * Dedicated Screen Recording permission flow.
 *
 * macOS binds the Screen Recording grant to the exact signed binary, so a new
 * build invalidates an old grant. This dialog explains how to fix it once and
 * for all instead of leaving the user in the "granted, but asked again" loop.
 */
export const PermissionModal: React.FC<PermissionModalProps> = ({
  isOpen,
  status,
  onClose,
  onRefresh,
}) => {
  const [requesting, setRequesting] = useState(false);

  if (!isOpen) return null;

  const granted = status?.granted ?? false;
  const promptShown = status?.prompt_shown ?? false;

  const handleRequest = async () => {
    setRequesting(true);
    try {
      await tauriApi.requestScreenCapturePermission();
      await onRefresh();
    } finally {
      setRequesting(false);
    }
  };

  const handleOpenSettings = async () => {
    await tauriApi.openScreenCaptureSettings();
  };

  const handleRestart = async () => {
    await tauriApi.restartApp();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-surface-card border border-surface-border rounded-xl w-full max-w-lg shadow-halo overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        <div
          data-tauri-drag-region="deep"
          className="flex items-center gap-3 px-6 py-4 border-b border-surface-border bg-surface-subtle/50"
        >
          <div
            className={`p-1.5 rounded-lg border ${
              granted
                ? "bg-emerald-950/40 border-emerald-700/50 text-emerald-400"
                : "bg-amber-950/40 border-amber-800/50 text-amber-400"
            }`}
          >
            {granted ? (
              <CheckCircle2 className="w-5 h-5" />
            ) : (
              <ShieldAlert className="w-5 h-5" />
            )}
          </div>
          <div>
            <h2 className="text-base font-semibold text-slate-100">
              Screen Recording access
            </h2>
            <p className="text-xs text-slate-400">
              System permission for TrueHalo screenshots
            </p>
          </div>
        </div>

        <div className="p-6 space-y-4 text-xs text-slate-300">
          {granted ? (
            <div className="p-3 rounded-lg bg-emerald-950/40 border border-emerald-800/60 text-emerald-200 flex items-start gap-2">
              <CheckCircle2 className="w-4 h-4 shrink-0 mt-0.5" />
              <span>
                Permission granted. If you rebuilt or reinstalled the app after
                granting it, restart TrueHalo — the system only applies the
                access after a restart.
              </span>
            </div>
          ) : (
            <div className="p-3 rounded-lg bg-amber-950/40 border border-amber-800/60 text-amber-200 flex items-start gap-2">
              <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
              <span>
                The system has not allowed TrueHalo to record the screen yet.
                Screenshots are unavailable without it.
              </span>
            </div>
          )}

          <ol className="space-y-2 list-decimal list-inside leading-relaxed">
            <li>
              Click <b>“Open System Settings”</b> below.
            </li>
            <li>
              Open the system privacy settings and select the screen
              recording pane.
            </li>
            <li>
              Turn on the switch next to <b>TrueHalo</b>.
            </li>
            <li>
              The system will ask you to quit and reopen the app — or click{" "}
              <b>“Restart TrueHalo”</b> in this window.
            </li>
          </ol>

          <div className="p-3 rounded-lg bg-surface-subtle/40 border border-surface-border text-[11px] text-slate-400 flex items-start gap-2">
            <Info className="w-4 h-4 shrink-0 mt-0.5 text-sky-400" />
            <span>
              <b className="text-slate-300">Important:</b> if the TrueHalo switch
              is already on but screenshots still ask for permission, turn it off
              and on again. That recreates the access record for the current
              build.
            </span>
          </div>

          {promptShown && !granted && (
            <div className="text-[11px] text-slate-500">
              The system dialog has already been shown once for this app run —
              it will not be shown again, so access must be granted manually in
              system settings.
            </div>
          )}
        </div>

        <div className="px-6 py-3 border-t border-surface-border bg-surface-subtle/50 flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <button
              onClick={handleOpenSettings}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-halo-accent hover:bg-halo-accent/90 text-white text-xs font-medium rounded-lg shadow-halo-sm transition-colors"
            >
              <ExternalLink className="w-3.5 h-3.5" />
              Open System Settings
            </button>

            {!granted && (
              <button
                onClick={handleRequest}
                disabled={requesting}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-surface-subtle hover:bg-surface-border text-slate-200 text-xs rounded-lg transition-colors disabled:opacity-50"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${requesting ? "animate-spin" : ""}`} />
                Request access
              </button>
            )}
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleRestart}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-surface-subtle hover:bg-surface-border text-slate-200 text-xs rounded-lg transition-colors"
              title="Restart the app to apply the permission"
            >
              <RotateCw className="w-3.5 h-3.5" />
              Restart
            </button>
            <button
              onClick={onClose}
              className="px-3 py-1.5 text-slate-400 hover:text-slate-200 text-xs rounded-lg hover:bg-surface-subtle transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
