import React, { useState, useEffect } from "react";
import {
  AppSettings,
  COMPRESSION_PRESETS,
  CompressionPreset,
  ModelInfo,
  ProviderId,
  QuickAction,
  ScreenCaptureStatus,
} from "../../lib/types";
import { tauriApi } from "../../lib/tauri";
import { formatHotkey, IS_WINDOWS } from "../../lib/hotkeys";
import { BUILT_IN_ACTIONS } from "../../lib/quickActions";
import {
  Key,
  Shield,
  RefreshCw,
  Trash2,
  Eye,
  EyeOff,
  CheckCircle2,
  AlertCircle,
  Sparkles,
  ExternalLink,
  Sliders,
  MonitorOff,
  CircleGauge,
  Keyboard,
  Zap,
} from "lucide-react";

interface ProviderSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  activeProvider: ProviderId;
  activeModel: string;
  onSelectModel: (provider: ProviderId, model: string) => void;
  settings: AppSettings;
  onChangeOpacity: (val: number) => void;
  onToggleVibrancy: (val: boolean) => void;
  onToggleHideFromDock: (val: boolean) => void;
  onToggleScreenRecord: (val: boolean) => void;
  onChangeSettings: (patch: Partial<AppSettings>) => void;
  permissionStatus: ScreenCaptureStatus | null;
  onRefreshPermission: () => Promise<void> | void;
  onOpenPermission: () => void;
  onRunSetup: () => void;
  customActions: QuickAction[];
  onChangeCustomActions: (actions: QuickAction[]) => void;
}

const PROVIDERS: {
  id: ProviderId;
  name: string;
  defaultModel: string;
  docsUrl: string;
  loginUrl?: string;
}[] = [
  {
    id: "openrouter",
    name: "OpenRouter",
    defaultModel: "minimax/minimax-m3.1-flash:free",
    docsUrl: "https://openrouter.ai/keys",
  },
  {
    id: "gemini",
    name: "Google Gemini",
    defaultModel: "gemini-1.5-flash",
    docsUrl: "https://aistudio.google.com/app/apikey",
  },
  {
    id: "openai",
    name: "OpenAI (ChatGPT)",
    defaultModel: "gpt-4o",
    docsUrl: "https://platform.openai.com/api-keys",
    loginUrl: "https://platform.openai.com/account/api-keys",
  },
];

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta"]);

const HotkeyRecorder: React.FC<{
  value: string;
  defaultValue: string;
  onChange: (value: string) => void;
}> = ({ value, defaultValue, onChange }) => {
  const [recording, setRecording] = useState(false);
  const [hint, setHint] = useState<string | null>(null);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Tab") {
      setRecording(false);
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      setRecording(false);
      setHint(null);
      event.currentTarget.blur();
      return;
    }

    if (event.key === "Backspace" || event.key === "Delete") {
      onChange(defaultValue);
      setRecording(false);
      setHint(null);
      event.currentTarget.blur();
      return;
    }

    if (MODIFIER_KEYS.has(event.key)) return;
    if (!event.code) return;

    const parts: string[] = [];
    if (event.metaKey) parts.push("Super");
    if (event.ctrlKey) parts.push("Control");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");

    const isFunctionKey = /^F\d{1,2}$/.test(event.code);
    if (parts.length === 0 && !isFunctionKey) {
      setHint("Hold Ctrl, Alt or ⌘ with the key");
      return;
    }

    parts.push(event.code);
    onChange(parts.join("+"));
    setRecording(false);
    setHint(null);
    event.currentTarget.blur();
  };

  return (
    <div className="flex flex-col items-end gap-1 shrink-0">
      <input
        readOnly
        aria-label="Record a shortcut"
        title="Click, then press a key combination"
        value={recording ? "Press a combination…" : formatHotkey(value)}
        onFocus={() => {
          setRecording(true);
          setHint(null);
        }}
        onBlur={() => setRecording(false)}
        onKeyDown={handleKeyDown}
        className={`w-44 px-2.5 py-1.5 rounded-lg bg-surface-deep border text-center font-mono text-[11px] cursor-pointer focus:outline-none transition-colors ${
          recording
            ? "border-halo-accent/70 text-halo-accent"
            : "border-surface-border text-slate-200 hover:border-slate-500"
        }`}
      />
      {hint && <span className="text-[10px] text-amber-300/80">{hint}</span>}
    </div>
  );
};

const HotkeyRow: React.FC<{
  label: string;
  description: string;
  value: string;
  defaultValue: string;
  onChange: (value: string) => void;
}> = ({ label, description, value, defaultValue, onChange }) => (
  <div className="flex items-center justify-between gap-4 py-3.5">
    <div className="min-w-0">
      <div className="text-xs font-medium text-slate-200">{label}</div>
      <div className="text-[11px] text-slate-500 mt-0.5">{description}</div>
    </div>
    <HotkeyRecorder value={value} defaultValue={defaultValue} onChange={onChange} />
  </div>
);

export const ProviderSettingsModal: React.FC<ProviderSettingsModalProps> = ({
  isOpen,
  onClose,
  activeProvider,
  activeModel,
  onSelectModel,
  settings,
  onChangeOpacity,
  onToggleVibrancy,
  onToggleHideFromDock,
  onToggleScreenRecord,
  onChangeSettings,
  permissionStatus,
  onRefreshPermission,
  onOpenPermission,
  onRunSetup,
  customActions,
  onChangeCustomActions,
}) => {
  const [activeTab, setActiveTab] = useState<"providers" | "appearance" | "hotkeys" | "actions" | "privacy">("providers");
  const [selectedProvider, setSelectedProvider] = useState<ProviderId>(activeProvider);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [hasKey, setHasKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [searchModel, setSearchModel] = useState("");
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" } | null>(null);
  const [actionLabel, setActionLabel] = useState("");
  const [actionPrompt, setActionPrompt] = useState("");
  const [keychainNotice, setKeychainNotice] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      checkKeyStatus(selectedProvider);
    }
  }, [isOpen, selectedProvider]);

  const checkKeyStatus = async (provider: ProviderId) => {
    setMessage(null);
    const keyExists = await tauriApi.hasApiKey(provider);
    setHasKey(keyExists);
    if (keyExists) {
      fetchModels(provider);
    } else {
      setModels([]);
    }
  };

  const fetchModels = async (provider: ProviderId) => {
    setLoadingModels(true);
    try {
      const list = await tauriApi.listModels(provider);
      setModels(list);
    } catch (err: any) {
      setMessage({
        type: "error",
        text: typeof err === "string" ? err : err.message || "Failed to fetch models",
      });
    } finally {
      setLoadingModels(false);
    }
  };

  const handleSaveKey = async () => {
    if (!apiKeyInput.trim()) return;
    setSaving(true);
    setMessage(null);
    try {
      await tauriApi.saveApiKey(selectedProvider, apiKeyInput.trim());
      setApiKeyInput("");
      setHasKey(true);
      setMessage({ type: "success", text: "Key saved to the system keychain." });
      await fetchModels(selectedProvider);
    } catch (err: any) {
      setMessage({ type: "error", text: err.message || "Could not save the key to the keychain" });
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteKey = async () => {
    if (!confirm(`Delete the ${selectedProvider} key from the keychain?`)) {
      return;
    }
    try {
      await tauriApi.deleteApiKey(selectedProvider);
      setHasKey(false);
      setModels([]);
      setMessage({ type: "success", text: "Key removed from the keychain" });
    } catch (err: any) {
      setMessage({ type: "error", text: "Could not delete the key" });
    }
  };

  const handleOpenExternal = async (url: string) => {
    try {
      await tauriApi.openExternal(url);
    } catch {
      setMessage({ type: "error", text: "Could not open the link in your browser" });
    }
  };

  const applyHotkeys = async (patch: Partial<AppSettings>) => {
    const next = { ...settings, ...patch };
    try {
      await tauriApi.setHotkeys(
        next.hotkeyToggle,
        next.hotkeyRegion,
        next.hotkeyFullscreen,
      );
      onChangeSettings(patch);
      setMessage({ type: "success", text: "Shortcuts updated" });
    } catch (e) {
      const text =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Could not update the shortcuts";
      setMessage({ type: "error", text });
    }
  };

  if (!isOpen) return null;

  const currentProviderInfo = PROVIDERS.find((p) => p.id === selectedProvider)!;
  const filteredModels = models.filter(
    (m) =>
      m.id.toLowerCase().includes(searchModel.toLowerCase()) ||
      m.name.toLowerCase().includes(searchModel.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-surface-card border border-surface-border rounded-xl w-full max-w-3xl max-h-[88vh] flex flex-col shadow-halo overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div
          data-tauri-drag-region="deep"
          className="flex items-center justify-between px-6 py-4 border-b border-surface-border bg-surface-subtle/50"
        >
          <div className="flex items-center gap-2.5">
            <div className="p-1.5 rounded-lg bg-halo-accent/20 text-halo-accent border border-halo-accent/30">
              <Key className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-semibold text-slate-100">TrueHalo Settings</h2>
              <p className="text-xs text-slate-400">AI providers, transparency and privacy</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-200 text-sm px-2.5 py-1 rounded-md hover:bg-surface-subtle transition-colors"
          >
            ✕
          </button>
        </div>

        {/* Primary Settings Navigation */}
        <div className="flex border-b border-surface-border px-6 pt-2 gap-4 bg-surface-deep/80 text-xs overflow-x-auto">
          <button
            onClick={() => setActiveTab("providers")}
            className={`pb-2 shrink-0 font-medium border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "providers"
                ? "border-halo-accent text-slate-100"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            <Key className="w-3.5 h-3.5" />
            <span>AI Providers &amp; Models</span>
          </button>

          <button
            onClick={() => setActiveTab("appearance")}
            className={`pb-2 shrink-0 font-medium border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "appearance"
                ? "border-halo-accent text-slate-100"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            <Sliders className="w-3.5 h-3.5" />
            <span>Transparency &amp; Appearance</span>
          </button>

          <button
            onClick={() => setActiveTab("hotkeys")}
            className={`pb-2 shrink-0 font-medium border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "hotkeys"
                ? "border-halo-accent text-slate-100"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            <Keyboard className="w-3.5 h-3.5" />
            <span>Shortcuts</span>
          </button>

          <button
            onClick={() => setActiveTab("actions")}
            className={`pb-2 shrink-0 font-medium border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "actions"
                ? "border-halo-accent text-slate-100"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            <Zap className="w-3.5 h-3.5" />
            <span>Quick Actions</span>
          </button>

          <button
            onClick={() => setActiveTab("privacy")}
            className={`pb-2 shrink-0 font-medium border-b-2 transition-all flex items-center gap-1.5 ${
              activeTab === "privacy"
                ? "border-halo-accent text-slate-100"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            <MonitorOff className="w-3.5 h-3.5" />
            <span>Screen Recording &amp; Privacy</span>
          </button>
        </div>

        {/* Tab 1: AI Providers & Models */}
        {activeTab === "providers" && (
          <div className="flex-1 flex flex-col overflow-hidden">
            {/* Sub-tabs for Providers */}
            <div className="flex border-b border-surface-border px-6 pt-3 gap-2 bg-surface-deep/40 shrink-0">
              {PROVIDERS.map((provider) => {
                const isSelected = selectedProvider === provider.id;
                return (
                  <button
                    key={provider.id}
                    onClick={() => setSelectedProvider(provider.id)}
                    className={`px-3 py-1.5 text-xs font-medium rounded-t-lg transition-all border-b-2 flex items-center gap-2 ${
                      isSelected
                        ? "border-halo-accent text-slate-100 bg-surface-card"
                        : "border-transparent text-slate-400 hover:text-slate-200"
                    }`}
                  >
                    <span>{provider.name}</span>
                    {provider.id === activeProvider && (
                      <span className="w-1.5 h-1.5 rounded-full bg-halo-accent shadow-halo-sm"></span>
                    )}
                  </button>
                );
              })}
              <button
                onClick={onRunSetup}
                className="ml-auto mb-1 px-2.5 py-1 text-[10px] rounded-md border border-surface-border text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              >
                Setup wizard
              </button>
            </div>

            {/* Provider Content */}
            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              {/* OpenAI Account Login Info Banner */}
              {selectedProvider === "openai" && (
                <div className="p-3.5 rounded-xl border border-sky-800/40 bg-sky-950/20 text-xs space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-sky-200 flex items-center gap-1.5">
                      <Sparkles className="w-4 h-4 text-sky-400" />
                      OpenAI / ChatGPT account sign-in
                    </span>
                    <button
                      onClick={() => handleOpenExternal(currentProviderInfo.loginUrl || currentProviderInfo.docsUrl)}
                      className="flex items-center gap-1 text-[11px] text-sky-300 hover:text-sky-100 bg-sky-900/40 hover:bg-sky-800/60 px-2.5 py-1 rounded-lg border border-sky-700/50 transition-colors"
                    >
                      <span>Open OpenAI API keys</span>
                      <ExternalLink className="w-3 h-3" />
                    </button>
                  </div>
                  <p className="text-slate-400 text-[11px] leading-relaxed">
                    The button opens the OpenAI sign-in and API keys page in your browser. OpenAI does not
                    offer account OAuth to third-party apps, so sign in there, click <b>Create new secret
                    key</b> and paste it below. The key is stored in the system credential store.
                  </p>
                </div>
              )}

              {/* Status banner */}
              <div className="flex items-center justify-between p-3.5 rounded-lg border border-surface-border bg-surface-subtle/40">
                <div className="flex items-center gap-2.5">
                  {hasKey ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-amber-400 shrink-0" />
                  )}
                  <div className="text-xs">
                    <span className="font-medium text-slate-200">
                      {hasKey ? "Key connected in the system keychain" : "API key not configured"}
                    </span>
                    <p className="text-slate-400 text-[11px] mt-0.5">
                      Get a key in the{" "}
                      <button
                        onClick={() => handleOpenExternal(currentProviderInfo.docsUrl)}
                        className="text-halo-400 hover:underline"
                      >
                        {currentProviderInfo.name}
                      </button>
                    </p>
                  </div>
                </div>

                {hasKey && (
                  <button
                    onClick={handleDeleteKey}
                    className="flex items-center gap-1 text-xs text-rose-400 hover:text-rose-300 px-2 py-1 rounded hover:bg-rose-950/30 transition-colors"
                    title="Delete key"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Delete</span>
                  </button>
                )}
              </div>

              {/* Key Input Section */}
              <div className="space-y-2">
                <label className="text-xs font-medium text-slate-300 flex items-center justify-between">
                  <span>{hasKey ? "Update API key" : "Enter API key"}</span>
                  <span className="flex items-center gap-1 text-[11px] text-slate-400">
                    <Shield className="w-3 h-3 text-emerald-400" />
                    System credential store
                  </span>
                </label>
                <div className="flex gap-2">
                  <div className="relative flex-1">
                    <input
                      type={showKey ? "text" : "password"}
                      placeholder={hasKey ? "••••••••••••••••••••••••" : "Paste your key (sk-...)"}
                      value={apiKeyInput}
                      onChange={(e) => setApiKeyInput(e.target.value)}
                      className="w-full bg-[#0b0c10] border border-surface-border focus:border-halo-accent rounded-lg px-3 py-2 text-xs text-slate-100 placeholder-slate-500 outline-none pr-8 transition-colors"
                    />
                    <button
                      type="button"
                      onClick={() => setShowKey(!showKey)}
                      className="absolute right-2.5 top-2.5 text-slate-400 hover:text-slate-200"
                    >
                      {showKey ? <EyeOff className="w-3.5 h-3.5" /> : <Eye className="w-3.5 h-3.5" />}
                    </button>
                  </div>
                  <button
                    onClick={handleSaveKey}
                    disabled={saving || !apiKeyInput.trim()}
                    className="px-4 py-2 bg-halo-accent hover:bg-halo-accent/90 disabled:opacity-40 disabled:cursor-not-allowed text-white text-xs font-medium rounded-lg shadow-halo-sm transition-all shrink-0"
                  >
                    {saving ? "Saving..." : "Save"}
                  </button>
                </div>
              </div>

              {/* Feedback messages */}
              {message && (
                <div
                  className={`p-3 rounded-lg text-xs flex items-center gap-2 ${
                    message.type === "success"
                      ? "bg-emerald-950/40 border border-emerald-800/60 text-emerald-200"
                      : "bg-rose-950/40 border border-rose-800/60 text-rose-200"
                  }`}
                >
                  {message.type === "success" ? (
                    <CheckCircle2 className="w-4 h-4 shrink-0" />
                  ) : (
                    <AlertCircle className="w-4 h-4 shrink-0" />
                  )}
                  <span>{message.text}</span>
                </div>
              )}

              {/* Model Selection section */}
              {hasKey && (
                <div className="space-y-3 pt-2 border-t border-surface-border">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-xs font-semibold text-slate-200">Available models</h3>
                      <p className="text-[11px] text-slate-400">
                        {selectedProvider === "openrouter" && "Default: MiniMax M3.1 Flash (free)"}
                        {selectedProvider !== "openrouter" && "Choose a model to work with"}
                      </p>
                    </div>
                    <button
                      onClick={() => fetchModels(selectedProvider)}
                      disabled={loadingModels}
                      className="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-200 px-2.5 py-1 rounded bg-surface-subtle hover:bg-surface-border transition-colors disabled:opacity-50"
                    >
                      <RefreshCw className={`w-3 h-3 ${loadingModels ? "animate-spin" : ""}`} />
                      <span>Refresh</span>
                    </button>
                  </div>

                  {models.length > 5 && (
                    <input
                      type="text"
                      placeholder="Search models (minimax, claude, gpt, flash)..."
                      value={searchModel}
                      onChange={(e) => setSearchModel(e.target.value)}
                      className="w-full bg-[#0b0c10] border border-surface-border focus:border-halo-accent rounded-lg px-3 py-1.5 text-xs text-slate-100 placeholder-slate-500 outline-none transition-colors"
                    />
                  )}

                  {loadingModels ? (
                    <div className="py-8 text-center text-xs text-slate-400 flex flex-col items-center gap-2">
                      <RefreshCw className="w-5 h-5 animate-spin text-halo-accent" />
                      <span>Loading the {currentProviderInfo.name} model catalog...</span>
                    </div>
                  ) : (
                    <div className="max-h-52 overflow-y-auto space-y-1.5 pr-1">
                      {filteredModels.length === 0 ? (
                        <div className="p-4 text-center text-xs text-slate-400">
                          No models found.
                        </div>
                      ) : (
                        filteredModels.map((m) => {
                          const isCurrent = selectedProvider === activeProvider && activeModel === m.id;
                          return (
                            <div
                              key={m.id}
                              onClick={() => onSelectModel(selectedProvider, m.id)}
                              className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-all flex items-center justify-between ${
                                isCurrent
                                  ? "bg-halo-accent/15 border-halo-accent/60 shadow-halo-sm text-slate-100"
                                  : "bg-[#0b0c10] border-surface-border hover:border-surface-border/80 hover:bg-surface-card text-slate-300"
                              }`}
                            >
                              <div className="min-w-0 pr-3">
                                <div className="flex items-center gap-2">
                                  <span className="font-medium truncate">{m.name}</span>
                                  {m.supports_vision && (
                                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-sky-950/60 border border-sky-800/60 text-sky-300 font-mono">
                                      vision
                                    </span>
                                  )}
                                  {m.id.includes("free") && (
                                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-950/60 border border-emerald-800/60 text-emerald-300 font-mono">
                                      free
                                    </span>
                                  )}
                                </div>
                                <div className="text-[11px] text-slate-500 truncate font-mono mt-0.5">
                                  {m.id}
                                </div>
                              </div>

                              {isCurrent && (
                                <span className="flex items-center gap-1 text-[11px] text-halo-accent font-medium shrink-0">
                                  <Sparkles className="w-3.5 h-3.5" />
                                  Active
                                </span>
                              )}
                            </div>
                          );
                        })
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Tab 2: Appearance & Opacity */}
        {activeTab === "appearance" && (
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Window transparency</h3>
              <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                Adjust how transparent the TrueHalo overlay is. The value is saved automatically and restored on restart.
              </p>
            </div>

            {IS_WINDOWS && settings.excludeScreenRecord && (
              <div className="p-3 rounded-lg bg-amber-950/20 border border-amber-900/40 text-[11px] text-amber-200/80 flex items-start gap-2">
                <AlertCircle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
                <span>
                  Screen-recording protection is on. Windows needs an opaque window to
                  avoid leaving a black box in recordings, so opacity and frosted glass
                  apply again once you turn protection off.
                </span>
              </div>
            )}

            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/30 space-y-4">
              <div className="flex items-center justify-between text-xs">
                <span className="font-medium text-slate-200">Interface opacity:</span>
                <span className="font-mono text-halo-accent font-semibold text-sm">
                  {Math.round(settings.windowOpacity * 100)}%
                </span>
              </div>

              <input
                type="range"
                min="0.05"
                max="1.0"
                step="0.01"
                value={settings.windowOpacity}
                onChange={(e) => onChangeOpacity(parseFloat(e.target.value))}
                className="w-full h-2 bg-[#0b0c10] rounded-lg appearance-none cursor-pointer accent-halo-accent"
              />

              <div className="flex justify-between text-[11px] text-slate-500">
                <span>5% (Almost invisible)</span>
                <span>50% (Glass)</span>
                <span>100% (Solid)</span>
              </div>
            </div>

            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/30 space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <span className="text-xs font-semibold text-slate-100">
                    Frosted glass (Vibrancy)
                  </span>
                  <p className="text-[11px] text-slate-400 mt-1 max-w-md leading-relaxed">
                    Native backdrop blur. Keeps text readable even when the
                    window is highly transparent.
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
                  <input
                    type="checkbox"
                    checked={settings.vibrancy}
                    onChange={(e) => onToggleVibrancy(e.target.checked)}
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-surface-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-halo-accent"></div>
                </label>
              </div>
            </div>
          </div>
        )}

        {/* Tab 3: Global shortcuts */}
        {activeTab === "hotkeys" && (
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Global shortcuts</h3>
              <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                Click a shortcut and press the new key combination. Use at least one
                modifier (Ctrl, Alt or ⌘). Esc cancels, Backspace resets one shortcut.
              </p>
            </div>

            {message && (
              <div
                className={`p-3 rounded-lg border text-xs ${
                  message.type === "success"
                    ? "bg-emerald-950/30 border-emerald-800/40 text-emerald-200"
                    : "bg-rose-950/30 border-rose-800/40 text-rose-200"
                }`}
              >
                {message.text}
              </div>
            )}

            <div className="rounded-xl border border-surface-border bg-surface-subtle/30 divide-y divide-surface-border px-4">
              <HotkeyRow
                label="Show / hide overlay"
                description="Works even when TrueHalo is hidden in the menu bar."
                value={settings.hotkeyToggle}
                defaultValue="Alt+Space"
                onChange={(value) => applyHotkeys({ hotkeyToggle: value })}
              />
              <HotkeyRow
                label="Capture region"
                description="Drag a rectangle, then ask about it."
                value={settings.hotkeyRegion}
                defaultValue="Alt+Shift+Digit2"
                onChange={(value) => applyHotkeys({ hotkeyRegion: value })}
              />
              <HotkeyRow
                label="Capture full screen"
                description="Grab every display and analyze it."
                value={settings.hotkeyFullscreen}
                defaultValue="Alt+Shift+Digit3"
                onChange={(value) => applyHotkeys({ hotkeyFullscreen: value })}
              />
            </div>

            <button
              onClick={() =>
                applyHotkeys({
                  hotkeyToggle: "Alt+Space",
                  hotkeyRegion: "Alt+Shift+Digit2",
                  hotkeyFullscreen: "Alt+Shift+Digit3",
                })
              }
              className="px-3 py-1.5 text-[11px] rounded-lg border border-surface-border text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
            >
              Reset to defaults
            </button>
          </div>
        )}

        {/* Tab 4: Quick Actions */}
        {activeTab === "actions" && (
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Quick Actions</h3>
              <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                One-click prompts that run against the current screenshot or
                recognized text. Custom actions are stored locally.
              </p>
            </div>

            {message && (
              <div
                className={`p-3 rounded-lg border text-xs ${
                  message.type === "success"
                    ? "bg-emerald-950/30 border-emerald-800/40 text-emerald-200"
                    : "bg-rose-950/30 border-rose-800/40 text-rose-200"
                }`}
              >
                {message.text}
              </div>
            )}

            <div className="rounded-xl border border-surface-border bg-surface-subtle/30 divide-y divide-surface-border px-4">
              {BUILT_IN_ACTIONS.map((action) => (
                <div key={action.id} className="py-3">
                  <div className="text-xs font-medium text-slate-200">{action.label}</div>
                  <div className="text-[11px] text-slate-500 mt-0.5 line-clamp-2">
                    {action.prompt}
                  </div>
                </div>
              ))}
            </div>

            <div>
              <h3 className="text-sm font-semibold text-slate-100">Your actions</h3>
              <p className="text-xs text-slate-400 mt-1">
                Add a name and the prompt to run. Use them from the chip row above
                the message box.
              </p>
            </div>

            {customActions.length > 0 && (
              <div className="rounded-xl border border-surface-border bg-surface-subtle/30 divide-y divide-surface-border px-4">
                {customActions.map((action) => (
                  <div
                    key={action.id}
                    className="py-3 flex items-start justify-between gap-3"
                  >
                    <div className="min-w-0">
                      <div className="text-xs font-medium text-slate-200">
                        {action.label}
                      </div>
                      <div className="text-[11px] text-slate-500 mt-0.5 line-clamp-2">
                        {action.prompt}
                      </div>
                    </div>
                    <button
                      onClick={() =>
                        onChangeCustomActions(
                          customActions.filter((item) => item.id !== action.id),
                        )
                      }
                      className="p-1.5 rounded-md text-slate-500 hover:text-rose-400 hover:bg-rose-950/30 transition-colors shrink-0"
                      title="Delete action"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            )}

            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/30 space-y-3">
              <div>
                <label className="text-[11px] font-medium text-slate-300 block mb-1">
                  Action name
                </label>
                <input
                  value={actionLabel}
                  onChange={(e) => setActionLabel(e.target.value)}
                  placeholder="e.g. Write tests"
                  maxLength={32}
                  className="w-full px-3 py-2 rounded-lg bg-surface-deep border border-surface-border text-xs text-slate-100 placeholder:text-slate-600 focus:outline-none focus:border-halo-accent/60"
                />
              </div>
              <div>
                <label className="text-[11px] font-medium text-slate-300 block mb-1">
                  Prompt
                </label>
                <textarea
                  value={actionPrompt}
                  onChange={(e) => setActionPrompt(e.target.value)}
                  placeholder="What should TrueHalo do with the screenshot or text?"
                  rows={3}
                  className="w-full px-3 py-2 rounded-lg bg-surface-deep border border-surface-border text-xs text-slate-100 placeholder:text-slate-600 focus:outline-none focus:border-halo-accent/60 resize-y"
                />
              </div>
              <button
                onClick={() => {
                  const label = actionLabel.trim();
                  const prompt = actionPrompt.trim();
                  if (!label || !prompt) {
                    setMessage({
                      type: "error",
                      text: "Give the action a name and a prompt",
                    });
                    return;
                  }
                  onChangeCustomActions([
                    ...customActions,
                    {
                      id: crypto.randomUUID(),
                      label: label.slice(0, 32),
                      prompt,
                      custom: true,
                    },
                  ]);
                  setActionLabel("");
                  setActionPrompt("");
                  setMessage({ type: "success", text: "Custom action added" });
                }}
                className="px-3 py-2 rounded-lg bg-halo-accent hover:bg-halo-accent/90 text-white text-xs font-medium transition-colors"
              >
                Add action
              </button>
            </div>
          </div>
        )}

        {/* Tab 5: Privacy, Screen Recording & Capture */}
        {activeTab === "privacy" && (
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
            {/* Screen Recording permission */}
            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/40 space-y-3">
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-start gap-2.5">
                  {permissionStatus?.granted ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
                  )}
                  <div>
                    <span className="text-xs font-semibold text-slate-100">
                      Screen Recording access
                    </span>
                    <p className="text-[11px] text-slate-400 mt-1 max-w-md leading-relaxed">
                      {permissionStatus?.granted
                        ? "Permission granted. Screenshots work."
                        : "Permission not granted — screenshots are unavailable until the system allows screen recording."}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={() => onRefreshPermission()}
                    className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
                    title="Refresh status"
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                  </button>
                  {!permissionStatus?.granted && (
                    <button
                      onClick={onOpenPermission}
                      className="px-3 py-1.5 bg-halo-accent hover:bg-halo-accent/90 text-white text-[11px] font-medium rounded-lg transition-colors"
                    >
                      Configure
                    </button>
                  )}
                </div>
              </div>
            </div>

            {/* Keychain access repair */}
            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/40 space-y-3">
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-start gap-2.5">
                  <Key className="w-4 h-4 text-halo-accent shrink-0 mt-0.5" />
                  <div>
                    <span className="text-xs font-semibold text-slate-100">
                      Password prompt when accessing keys
                    </span>
                    <p className="text-[11px] text-slate-400 mt-1 max-w-md leading-relaxed">
                      If the system asks for a password when you open Settings or send
                      a request, the keys were saved by an old (unsigned) build.
                      Click the button once: the keys are re-bound to the current
                      signature and the prompts stop for good.
                    </p>
                  </div>
                </div>

                <button
                  onClick={async () => {
                    setKeychainNotice("Re-binding keys…");
                    try {
                      const result = await tauriApi.repairKeychainAccess();
                      setKeychainNotice(
                        result.has_any_key
                          ? `Done: ${result.checked} key(s) checked, ${result.repaired} re-bound.`
                          : "No saved keys — no password will be needed."
                      );
                    } catch (e) {
                      setKeychainNotice("Could not re-bind the keys. Please try again.");
                    }
                  }}
                  className="shrink-0 px-3 py-1.5 bg-surface-subtle hover:bg-surface-border border border-surface-border text-slate-200 text-[11px] font-medium rounded-lg transition-colors"
                >
                  Fix
                </button>
              </div>

              {keychainNotice && (
                <div className="p-2.5 rounded-lg bg-surface-deep/60 border border-surface-border text-[11px] text-slate-300">
                  {keychainNotice}
                </div>
              )}

              <p className="text-[10px] text-slate-500 leading-relaxed">
                Tip: if the system dialog still appears, choose{" "}
                <b className="text-slate-400">“Always Allow”</b> rather than just
                “Allow” — that way the system remembers the access.
              </p>
            </div>

            {/* Screenshot compression */}
            <div>
              <h3 className="text-sm font-semibold text-slate-100 flex items-center gap-2">
                <CircleGauge className="w-4 h-4 text-halo-accent" />
                Sending screenshots
              </h3>
              <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                Compression greatly reduces the token count and speeds up replies.
                A raw Retina screenshot is 10–20 MB, usually under 1 MB after
                compression.
              </p>
            </div>

            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/30 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-slate-100">
                  Compress screenshots before sending
                </span>
                <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
                  <input
                    type="checkbox"
                    checked={settings.compressScreenshots}
                    onChange={(e) =>
                      onChangeSettings({ compressScreenshots: e.target.checked })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-surface-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-halo-accent"></div>
                </label>
              </div>

              {settings.compressScreenshots && (
                <div className="grid grid-cols-3 gap-2">
                  {(Object.keys(COMPRESSION_PRESETS) as CompressionPreset[]).map((key) => {
                    const preset = COMPRESSION_PRESETS[key];
                    const isActive = settings.compressionPreset === key;
                    return (
                      <button
                        key={key}
                        onClick={() => onChangeSettings({ compressionPreset: key })}
                        className={`p-2.5 rounded-lg border text-left transition-all ${
                          isActive
                            ? "bg-halo-accent/15 border-halo-accent/60 text-slate-100"
                            : "bg-[#0b0c10] border-surface-border hover:border-surface-border/80 text-slate-300"
                        }`}
                      >
                        <span className="text-xs font-medium block">{preset.label}</span>
                        <span className="text-[10px] text-slate-500 block mt-0.5">
                          {preset.maxDimension}px · q{preset.quality}
                        </span>
                      </button>
                    );
                  })}
                </div>
              )}

              <div className="flex items-center justify-between border-t border-surface-border pt-3">
                <div>
                  <span className="text-xs font-medium text-slate-200">
                    Hide the window while capturing
                  </span>
                  <p className="text-[11px] text-slate-400 mt-0.5">
                    Ensures the overlay never ends up in its own screenshot.
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
                  <input
                    type="checkbox"
                    checked={settings.hideWindowOnCapture}
                    onChange={(e) =>
                      onChangeSettings({ hideWindowOnCapture: e.target.checked })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-surface-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-halo-accent"></div>
                </label>
              </div>
            </div>

            {/* Screenshot input mode */}
            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/30 space-y-3">
              <div>
                <span className="text-xs font-semibold text-slate-100">
                  Screenshot input
                </span>
                <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">
                  Text mode recognizes text on-device with the system OCR engine
                  and sends only that text. It is faster, cheaper, and easier to
                  correct before asking.
                </p>
              </div>

              <div className="grid grid-cols-3 gap-2">
                {(["auto", "text", "image"] as const).map((mode) => {
                  const isActive = settings.textMode === mode;
                  const label =
                    mode === "auto" ? "Auto" : mode === "text" ? "Text only" : "Image only";
                  const hint =
                    mode === "auto"
                      ? "OCR when useful"
                      : mode === "text"
                        ? "Fallback to image"
                        : "Always send image";
                  return (
                    <button
                      key={mode}
                      onClick={() => onChangeSettings({ textMode: mode })}
                      className={`p-2.5 rounded-lg border text-left transition-all ${
                        isActive
                          ? "bg-halo-accent/15 border-halo-accent/60 text-slate-100"
                          : "bg-[#0b0c10] border-surface-border hover:border-surface-border/80 text-slate-300"
                      }`}
                    >
                      <span className="text-xs font-medium block">{label}</span>
                      <span className="text-[10px] text-slate-500 block mt-0.5">
                        {hint}
                      </span>
                    </button>
                  );
                })}
              </div>

              <p className="text-[10px] text-slate-500">
                OCR never leaves your device. If it finds no text, TrueHalo sends
                the screenshot instead so the question still works.
              </p>
            </div>

            {/* Stealth */}
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Stealth mode</h3>
              <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                TrueHalo stays available through the menu bar / tray icon.
              </p>
            </div>

            <div className="p-4 rounded-xl border border-surface-border bg-surface-subtle/40 space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <span className="text-xs font-semibold text-slate-100">
                    Hide from the Dock / taskbar and app switcher
                  </span>
                  <p className="text-[11px] text-slate-400 mt-1 max-w-md leading-relaxed">
                    The app is not shown in the Dock / taskbar or the app switcher,
                    but keeps running. Use the menu bar / tray icon to control it.
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
                  <input
                    type="checkbox"
                    checked={settings.hideFromDock}
                    onChange={(e) => onToggleHideFromDock(e.target.checked)}
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-surface-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-halo-accent"></div>
                </label>
              </div>

              <div className="flex items-center justify-between border-t border-surface-border pt-3">
                <div>
                  <span className="text-xs font-semibold text-slate-100 flex items-center gap-2">
                    <MonitorOff className="w-4 h-4 text-halo-accent" />
                    Hide the window from other screen recordings
                  </span>
                  <p className="text-[11px] text-slate-400 mt-1 max-w-md leading-relaxed">
                    The TrueHalo window is excluded from captures by other apps
                    (OBS, Zoom, QuickTime).
                  </p>
                  <p className="text-[10px] text-slate-500 mt-1 max-w-md leading-relaxed">
                    macOS 13+ or Windows 10 2004+. On older Windows builds the
                    window stays visible in recordings.
                  </p>
                  {IS_WINDOWS && settings.excludeScreenRecord && (
                    <p className="text-[10px] text-amber-300/80 mt-1 max-w-md leading-relaxed">
                      Windows: the overlay stays fully opaque while this is on.
                      Transparency and frosted glass come back when you turn it off.
                    </p>
                  )}
                </div>

                <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
                  <input
                    type="checkbox"
                    checked={settings.excludeScreenRecord}
                    onChange={(e) => onToggleScreenRecord(e.target.checked)}
                    className="sr-only peer"
                  />
                  <div className="w-11 h-6 bg-surface-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-halo-accent"></div>
                </label>
              </div>

              {settings.excludeScreenRecord && (
                <div className="p-2.5 rounded-lg bg-emerald-950/40 border border-emerald-800/50 text-emerald-300 text-[11px] flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 shrink-0" />
                  <span>Protection active: the TrueHalo window is hidden from external screen captures.</span>
                </div>
              )}
            </div>

            <div className="p-4 rounded-xl border border-amber-900/40 bg-amber-950/20 text-xs text-amber-200/80 space-y-1.5">
              <span className="font-semibold text-amber-300 flex items-center gap-1.5">
                <AlertCircle className="w-4 h-4 text-amber-400" />
                Platform limitations:
              </span>
              <p className="text-[11px] leading-relaxed">
                Screen Recording permission is bound to the app's code signature.
                TrueHalo is signed with a stable certificate, so access survives
                updates. If you replaced the app manually, the system may ask
                again — that is normal behaviour.
              </p>
            </div>

            {/* History / token hygiene */}
            <div className="p-4 rounded-xl border border-rose-900/40 bg-rose-950/20 space-y-3">
              <div>
                <span className="text-xs font-semibold text-rose-200">
                  History and tokens
                </span>
                <p className="text-[11px] text-slate-400 mt-1 leading-relaxed max-w-md">
                  Only the latest screenshot is sent with each request — older
                  images are replaced with a placeholder to save tokens. You can
                  delete the entire local chat history here.
                </p>
              </div>
              <button
                onClick={async () => {
                  if (!confirm("Delete the entire local chat history?")) return;
                  await tauriApi.clearAllHistory();
                  setMessage({ type: "success", text: "Chat history deleted" });
                }}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-900/50 hover:bg-rose-800/60 border border-rose-700/50 text-rose-100 text-[11px] font-medium rounded-lg transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
                Clear all history
              </button>
            </div>
          </div>
        )}

        {/* Footer */}
        <div className="px-6 py-3 border-t border-surface-border bg-surface-subtle/50 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-1.5 bg-surface-subtle hover:bg-surface-border text-slate-200 text-xs font-medium rounded-lg transition-colors"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
