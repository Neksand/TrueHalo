import React, { useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  Check,
  ExternalLink,
  KeyRound,
  Loader2,
  Monitor,
  Search,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { tauriApi } from "../../lib/tauri";
import { ModelInfo, ProviderId } from "../../lib/types";
import { PROVIDERS, ProviderInfo } from "../../lib/providers";

interface OnboardingWizardProps {
  isOpen: boolean;
  onComplete: (provider: ProviderId, model: string) => void;
}

const STEPS = ["Provider", "API key", "Model", "Ready"];

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({ isOpen, onComplete }) => {
  const [step, setStep] = useState(0);
  const [provider, setProvider] = useState<ProviderInfo | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState("");
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Start from a clean slate every time the wizard is opened.
  useEffect(() => {
    if (!isOpen) return;
    setStep(0);
    setProvider(null);
    setApiKey("");
    setModels([]);
    setSelectedModel("");
    setSearch("");
    setBusy(false);
    setError(null);
  }, [isOpen]);

  const filteredModels = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = q
      ? models.filter(
          (m) => m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q),
        )
      : models;
    return list.slice(0, 80);
  }, [models, search]);

  if (!isOpen) return null;

  const chooseProvider = (next: ProviderInfo) => {
    setProvider(next);
    setSelectedModel(next.defaultModel);
    setError(null);
    setStep(1);
  };

  const openKeyPage = async () => {
    if (!provider) return;
    try {
      await tauriApi.openExternal(provider.keyUrl);
    } catch {
      setError("Could not open the browser. Copy the link from the provider's website.");
    }
  };

  const saveAndTest = async () => {
    if (!provider || !apiKey.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await tauriApi.saveApiKey(provider.id, apiKey.trim());
      const catalog = await tauriApi.listModels(provider.id);
      if (!catalog.length) {
        throw new Error("The provider returned an empty model list.");
      }
      setModels(catalog);
      const preferred = catalog.find((m) => m.id === provider.defaultModel);
      setSelectedModel(preferred?.id ?? catalog[0].id);
      setStep(2);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setError(
        message.includes("401") || /unauthori[sz]ed|invalid.*key/i.test(message)
          ? "That key was rejected. Check it and try again."
          : `Could not verify the key: ${message}`,
      );
    } finally {
      setBusy(false);
    }
  };

  const finish = () => {
    if (!provider || !selectedModel) return;
    setStep(3);
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/80 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-surface-card border border-surface-border rounded-2xl w-full max-w-xl shadow-halo overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-6 pt-5 pb-4 border-b border-surface-border bg-surface-subtle/40">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-halo-accent/20 text-halo-accent border border-halo-accent/30">
              <Sparkles className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-semibold text-slate-100">Welcome to TrueHalo</h2>
              <p className="text-xs text-slate-400">Three steps and you are ready to capture.</p>
            </div>
          </div>

          <div className="flex items-center gap-1.5 mt-4">
            {STEPS.map((label, index) => (
              <React.Fragment key={label}>
                <div
                  className={`flex items-center gap-1.5 text-[10px] font-medium ${
                    index === step
                      ? "text-halo-accent"
                      : index < step
                        ? "text-emerald-400"
                        : "text-slate-600"
                  }`}
                >
                  <span
                    className={`w-4 h-4 rounded-full flex items-center justify-center border ${
                      index < step
                        ? "bg-emerald-500/20 border-emerald-500/40"
                        : index === step
                          ? "bg-halo-accent/20 border-halo-accent/50"
                          : "border-surface-border"
                    }`}
                  >
                    {index < step ? <Check className="w-2.5 h-2.5" /> : index + 1}
                  </span>
                  {label}
                </div>
                {index < STEPS.length - 1 && (
                  <div className="flex-1 h-px bg-surface-border" />
                )}
              </React.Fragment>
            ))}
          </div>
        </div>

        <div className="p-6 space-y-4">
          {error && (
            <div className="p-3 rounded-lg bg-rose-950/40 border border-rose-800/50 text-rose-200 text-xs">
              {error}
            </div>
          )}

          {/* Step 1: provider */}
          {step === 0 && (
            <div className="space-y-3">
              <p className="text-xs text-slate-400">
                Pick where your AI requests go. You bring your own key, so there is no
                subscription and no middleman.
              </p>
              {PROVIDERS.map((p) => (
                <button
                  key={p.id}
                  onClick={() => chooseProvider(p)}
                  className="w-full flex items-center justify-between gap-4 p-4 rounded-xl border border-surface-border bg-surface-subtle/30 hover:border-halo-accent/50 hover:bg-surface-subtle/60 transition-all text-left"
                >
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-semibold text-slate-100">{p.name}</span>
                      {p.recommended && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-halo-accent/15 text-halo-accent border border-halo-accent/30">
                          Recommended
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-slate-400 mt-1">{p.blurb}</p>
                  </div>
                  <ArrowRight className="w-4 h-4 text-slate-500 shrink-0" />
                </button>
              ))}
            </div>
          )}

          {/* Step 2: key */}
          {step === 1 && provider && (
            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">
                  Paste your {provider.name} API key
                </h3>
                <p className="text-[11px] text-slate-400 mt-1">
                  The key is stored in the system credential store and never reaches the UI again.
                </p>
              </div>

              <div className="flex items-center gap-2">
                <div className="relative flex-1">
                  <KeyRound className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") saveAndTest();
                    }}
                    placeholder="sk-..."
                    autoFocus
                    className="w-full pl-9 pr-3 py-2.5 rounded-lg bg-surface-deep border border-surface-border text-sm text-slate-100 placeholder:text-slate-600 focus:outline-none focus:border-halo-accent/60"
                  />
                </div>
                <button
                  onClick={saveAndTest}
                  disabled={busy || !apiKey.trim()}
                  className="px-4 py-2.5 rounded-lg bg-halo-accent hover:bg-halo-accent/90 disabled:opacity-40 disabled:cursor-not-allowed text-white text-xs font-medium transition-colors flex items-center gap-2"
                >
                  {busy && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
                  Save &amp; test
                </button>
              </div>

              <button
                onClick={openKeyPage}
                className="text-[11px] text-halo-400 hover:underline flex items-center gap-1"
              >
                Get a {provider.name} key
                <ExternalLink className="w-3 h-3" />
              </button>
            </div>
          )}

          {/* Step 3: model */}
          {step === 2 && provider && (
            <div className="space-y-3">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">Pick a model</h3>
                <p className="text-[11px] text-slate-400 mt-1">
                  Vision models can read screenshots. You can change this later in Settings.
                </p>
              </div>

              <div className="relative">
                <Search className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search models"
                  className="w-full pl-9 pr-3 py-2 rounded-lg bg-surface-deep border border-surface-border text-xs text-slate-100 placeholder:text-slate-600 focus:outline-none focus:border-halo-accent/60"
                />
              </div>

              <div className="max-h-64 overflow-y-auto rounded-lg border border-surface-border divide-y divide-surface-border">
                {filteredModels.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => setSelectedModel(m.id)}
                    className={`w-full flex items-center justify-between gap-3 px-3 py-2 text-left transition-colors ${
                      selectedModel === m.id
                        ? "bg-halo-accent/10"
                        : "hover:bg-surface-subtle/50"
                    }`}
                  >
                    <div className="min-w-0">
                      <div className="text-xs text-slate-200 truncate">{m.name}</div>
                      <div className="text-[10px] text-slate-500 truncate">{m.id}</div>
                    </div>
                    {m.supports_vision && (
                      <span className="text-[9px] px-1.5 py-0.5 rounded bg-sky-500/10 text-sky-300 border border-sky-500/30 shrink-0">
                        vision
                      </span>
                    )}
                  </button>
                ))}
                {filteredModels.length === 0 && (
                  <div className="p-4 text-xs text-slate-500 text-center">No models match.</div>
                )}
              </div>

              <button
                onClick={finish}
                disabled={!selectedModel}
                className="w-full py-2.5 rounded-lg bg-halo-accent hover:bg-halo-accent/90 disabled:opacity-40 disabled:cursor-not-allowed text-white text-xs font-medium transition-colors flex items-center justify-center gap-2"
              >
                Continue
                <ArrowRight className="w-3.5 h-3.5" />
              </button>
            </div>
          )}

          {/* Step 4: ready */}
          {step === 3 && provider && (
            <div className="space-y-4">
              <div className="p-4 rounded-xl bg-emerald-950/30 border border-emerald-800/40 flex items-start gap-3">
                <ShieldCheck className="w-5 h-5 text-emerald-400 shrink-0 mt-0.5" />
                <div>
                  <div className="text-sm font-semibold text-emerald-200">You are ready</div>
                  <p className="text-[11px] text-emerald-200/70 mt-1">
                    {provider.name} · {selectedModel}
                  </p>
                </div>
              </div>

              <div className="space-y-2 text-xs text-slate-300">
                <div className="flex items-center gap-2">
                  <Monitor className="w-4 h-4 text-slate-500" />
                  <span>Capture a region with the hotkey, then ask your question.</span>
                </div>
                <div className="flex items-center gap-2">
                  <KeyRound className="w-4 h-4 text-slate-500" />
                  <span>Your key stays in the system credential store.</span>
                </div>
                <div className="flex items-center gap-2">
                  <ShieldCheck className="w-4 h-4 text-slate-500" />
                  <span>The overlay is hidden from screen recordings.</span>
                </div>
              </div>

              <button
                onClick={() => onComplete(provider.id, selectedModel)}
                className="w-full py-2.5 rounded-lg bg-halo-accent hover:bg-halo-accent/90 text-white text-xs font-medium transition-colors"
              >
                Start using TrueHalo
              </button>
            </div>
          )}
        </div>

        {step > 0 && step < 3 && (
          <div className="px-6 pb-5">
            <button
              onClick={() => {
                setError(null);
                setStep(step - 1);
              }}
              className="text-[11px] text-slate-500 hover:text-slate-300 transition-colors"
            >
              Back
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
