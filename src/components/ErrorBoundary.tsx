import React from "react";
import { tauriApi } from "../lib/tauri";

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Catches render errors so a broken screen can never leave the transparent
 * window blank. The error is written to the native log for diagnosis.
 */
export class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    const detail = `${error.message}\n${error.stack ?? ""}\n${info.componentStack ?? ""}`;
    console.error("TrueHalo UI error:", detail);
    void tauriApi.logFrontendError(detail).catch(() => {});
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <div className="h-screen w-screen flex items-center justify-center bg-[#0d0e12] text-slate-200 p-6">
        <div className="max-w-md space-y-3 text-center">
          <h1 className="text-sm font-semibold">Something went wrong</h1>
          <p className="text-xs text-slate-400">
            The interface hit an unexpected error. Details were written to the
            TrueHalo log.
          </p>
          <p className="text-[11px] text-rose-300 font-mono break-words">
            {this.state.error.message}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="px-3 py-1.5 rounded-lg bg-halo-accent text-white text-xs font-medium"
          >
            Reload
          </button>
        </div>
      </div>
    );
  }
}
