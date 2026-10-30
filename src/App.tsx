import React, { useState, useEffect, useCallback } from "react";
import { ChatView } from "./features/chat/ChatView";
import { ProviderSettingsModal } from "./features/providers/ProviderSettingsModal";
import { OnboardingWizard } from "./features/onboarding/OnboardingWizard";
import { PermissionModal } from "./features/capture/PermissionModal";
import {
  AppSettings,
  AssistantProfile,
  DEFAULT_SETTINGS,
  ProviderId,
  QuickAction,
  ScreenCaptureStatus,
} from "./lib/types";
import { tauriApi } from "./lib/tauri";
import { IS_WINDOWS } from "./lib/hotkeys";
import { CUSTOM_ACTIONS_PREF, parseCustomActions } from "./lib/quickActions";
import {
  ACTIVE_PROFILE_PREF,
  PROFILES_PREF,
  parseProfiles,
} from "./lib/profiles";

const PREF_KEYS: Record<keyof AppSettings, string> = {
  windowOpacity: "window_opacity",
  vibrancy: "vibrancy",
  hideFromDock: "hide_from_dock",
  hideFromTaskbar: "hide_from_taskbar",
  clickThrough: "click_through",
  excludeScreenRecord: "exclude_screen_recording",
  alwaysOnTop: "always_on_top",
  compressScreenshots: "compress_screenshots",
  compressionPreset: "compression_preset",
  hideWindowOnCapture: "hide_window_on_capture",
  keepImagesInHistory: "keep_images_in_history",
  ephemeralHistory: "ephemeral_history",
  localEndpoint: "local_endpoint",
  hotkeyToggle: "hotkey_toggle",
  hotkeyRegion: "hotkey_region",
  hotkeyFullscreen: "hotkey_fullscreen",
  textMode: "text_mode",
};

export const App: React.FC = () => {
  const [activeProvider, setActiveProvider] = useState<ProviderId>("openrouter");
  const [activeModel, setActiveModel] = useState<string>("minimax/minimax-m3.1-flash:free");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isOnboardingOpen, setIsOnboardingOpen] = useState(false);
  const [isPermissionOpen, setIsPermissionOpen] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [customActions, setCustomActions] = useState<QuickAction[]>([]);
  const [profiles, setProfiles] = useState<AssistantProfile[]>([]);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [permissionStatus, setPermissionStatus] = useState<ScreenCaptureStatus | null>(null);

  // ---- Persist one settings field in SQLite and apply it natively -------------
  const persistSetting = useCallback(async (key: keyof AppSettings, value: string) => {
    try {
      await tauriApi.setPreference(PREF_KEYS[key], value);
    } catch (e) {
      console.warn(`Could not save preference ${PREF_KEYS[key]}:`, e);
    }
  }, []);

  const updateSettings = useCallback(
    (patch: Partial<AppSettings>) => {
      setSettings((prev) => ({ ...prev, ...patch }));
      (Object.keys(patch) as (keyof AppSettings)[]).forEach((key) => {
        const value = patch[key];
        if (value !== undefined) persistSetting(key, String(value));
      });
    },
    [persistSetting]
  );

  // ---- Screen Recording permission -------------------------------------------
  const refreshPermission = useCallback(async () => {
    try {
      setPermissionStatus(await tauriApi.screenCaptureStatus());
    } catch (e) {
      console.warn("Could not read screen capture status:", e);
    }
  }, []);

  // ---- Bootstrap --------------------------------------------------------------
  useEffect(() => {
    (async () => {
      let storedProvider: string | null = null;
      try {
        const [
          providerPref,
          modelPref,
          opacityPref,
          protection,
          vibrancy,
          hideFromDock,
          alwaysOnTop,
          compress,
          preset,
          hideOnCapture,
          hotkeyTogglePref,
          hotkeyRegionPref,
          hotkeyFullscreenPref,
          textModePref,
          customActionsPref,
          profilesPref,
          activeProfilePref,
          hideTaskbarPref,
          clickThroughPref,
          keepImagesPref,
          ephemeralPref,
          localEndpointPref,
        ] = await Promise.all([
          tauriApi.getPreference("active_provider"),
          tauriApi.getPreference("active_model"),
          tauriApi.getPreference(PREF_KEYS.windowOpacity),
          tauriApi.getPreference(PREF_KEYS.excludeScreenRecord),
          tauriApi.getPreference(PREF_KEYS.vibrancy),
          tauriApi.getPreference(PREF_KEYS.hideFromDock),
          tauriApi.getPreference(PREF_KEYS.alwaysOnTop),
          tauriApi.getPreference(PREF_KEYS.compressScreenshots),
          tauriApi.getPreference(PREF_KEYS.compressionPreset),
          tauriApi.getPreference(PREF_KEYS.hideWindowOnCapture),
          tauriApi.getPreference(PREF_KEYS.hotkeyToggle),
          tauriApi.getPreference(PREF_KEYS.hotkeyRegion),
          tauriApi.getPreference(PREF_KEYS.hotkeyFullscreen),
          tauriApi.getPreference(PREF_KEYS.textMode),
          tauriApi.getPreference(CUSTOM_ACTIONS_PREF),
          tauriApi.getPreference(PROFILES_PREF),
          tauriApi.getPreference(ACTIVE_PROFILE_PREF),
          tauriApi.getPreference(PREF_KEYS.hideFromTaskbar),
          tauriApi.getPreference(PREF_KEYS.clickThrough),
          tauriApi.getPreference(PREF_KEYS.keepImagesInHistory),
          tauriApi.getPreference(PREF_KEYS.ephemeralHistory),
          tauriApi.getPreference(PREF_KEYS.localEndpoint),
        ]);

        setCustomActions(parseCustomActions(customActionsPref));
        storedProvider = providerPref;
        const loadedProfiles = parseProfiles(profilesPref);
        setProfiles(loadedProfiles);
        if (activeProfilePref && loadedProfiles.some((p) => p.id === activeProfilePref)) {
          setActiveProfileId(activeProfilePref);
        }

        // Ephemeral mode: leave no history behind from the previous run.
        if (ephemeralPref === "true") {
          try {
            await tauriApi.clearAllHistory();
          } catch (e) {
            console.warn("Could not clear ephemeral history:", e);
          }
        }

        if (providerPref) setActiveProvider(providerPref as ProviderId);
        if (modelPref) {
          setActiveModel(modelPref);
        } else if (!providerPref || providerPref === "openrouter") {
          setActiveModel("minimax/minimax-m3.1-flash:free");
        }

        const parsedOpacity = opacityPref ? parseFloat(opacityPref) : NaN;
        const shouldProtect = protection !== null ? protection === "true" : true;
        const presetValue =
          preset === "quality" || preset === "speed"
            ? (preset as AppSettings["compressionPreset"])
            : "balanced";

        const loaded: AppSettings = {
          windowOpacity: !isNaN(parsedOpacity) ? parsedOpacity : DEFAULT_SETTINGS.windowOpacity,
          vibrancy: vibrancy !== null ? vibrancy === "true" : DEFAULT_SETTINGS.vibrancy,
          hideFromDock:
            hideFromDock !== null ? hideFromDock === "true" : DEFAULT_SETTINGS.hideFromDock,
          hideFromTaskbar:
            hideTaskbarPref !== null
              ? hideTaskbarPref === "true"
              : DEFAULT_SETTINGS.hideFromTaskbar,
          clickThrough:
            clickThroughPref !== null
              ? clickThroughPref === "true"
              : DEFAULT_SETTINGS.clickThrough,
          excludeScreenRecord: shouldProtect,
          alwaysOnTop:
            alwaysOnTop !== null ? alwaysOnTop === "true" : DEFAULT_SETTINGS.alwaysOnTop,
          compressScreenshots:
            compress !== null ? compress === "true" : DEFAULT_SETTINGS.compressScreenshots,
          compressionPreset: presetValue,
          hideWindowOnCapture:
            hideOnCapture !== null
              ? hideOnCapture === "true"
              : DEFAULT_SETTINGS.hideWindowOnCapture,
          keepImagesInHistory:
            keepImagesPref !== null
              ? keepImagesPref === "true"
              : DEFAULT_SETTINGS.keepImagesInHistory,
          ephemeralHistory:
            ephemeralPref !== null
              ? ephemeralPref === "true"
              : DEFAULT_SETTINGS.ephemeralHistory,
          localEndpoint:
            localEndpointPref?.trim() || DEFAULT_SETTINGS.localEndpoint,
          hotkeyToggle: hotkeyTogglePref || DEFAULT_SETTINGS.hotkeyToggle,
          hotkeyRegion: hotkeyRegionPref || DEFAULT_SETTINGS.hotkeyRegion,
          hotkeyFullscreen: hotkeyFullscreenPref || DEFAULT_SETTINGS.hotkeyFullscreen,
          textMode:
            textModePref === "image" || textModePref === "text"
              ? textModePref
              : "auto",
        };
        setSettings(loaded);

        // Re-apply natively so backend and UI never drift apart.
        const protectionApplied = await tauriApi.setScreenRecordingProtection(
          loaded.excludeScreenRecord,
        );
        if (loaded.excludeScreenRecord && !protectionApplied) {
          // Unsupported OS/build (Windows < 10 2004): keep the UI honest.
          setSettings((prev) => ({ ...prev, excludeScreenRecord: false }));
        }
        const dockApplied = await tauriApi.setHideFromDock(loaded.hideFromDock);
        if (loaded.hideFromDock && !dockApplied) {
          // Accessory mode is macOS-only.
          setSettings((prev) => ({ ...prev, hideFromDock: false }));
        }
        await tauriApi.setAlwaysOnTop(loaded.alwaysOnTop);
        await tauriApi.setClickThrough(loaded.clickThrough);
        if (IS_WINDOWS) {
          await tauriApi.setHideFromTaskbar(loaded.hideFromTaskbar);
        }
      } catch (e) {
        console.warn("Failed to load stored preferences:", e);
      }

      await refreshPermission();

      // Check credentials (local servers need no key)
      const hasOpenRouter = await tauriApi.hasApiKey("openrouter");
      const hasGemini = await tauriApi.hasApiKey("gemini");
      const hasOpenAI = await tauriApi.hasApiKey("openai");

      if (!hasOpenRouter && !hasGemini && !hasOpenAI && storedProvider !== "local") {
        setIsOnboardingOpen(true);
      }
    })();
  }, [refreshPermission]);

  // Windows cannot hide a translucent window from capture without leaving a
  // black rectangle where the window used to be: the excluded region inherits
  // the window's alpha. While protection is on, the overlay therefore stays
  // fully opaque; macOS has no such limitation.
  const protectionForcesOpaque = IS_WINDOWS && settings.excludeScreenRecord;
  const effectiveOpacity = protectionForcesOpaque ? 1 : settings.windowOpacity;
  const effectiveVibrancy = settings.vibrancy && !protectionForcesOpaque;

  // Keep the transparent window opacity in a CSS variable (no re-render cost).
  useEffect(() => {
    document.documentElement.style.setProperty("--window-opacity", String(effectiveOpacity));
  }, [effectiveOpacity]);

  // Apply the native backdrop material whenever the effective state changes
  // (user toggle, capture protection, or restored preferences).
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const applied = await tauriApi.setVibrancy(effectiveVibrancy);
      if (!cancelled && effectiveVibrancy && !applied) {
        // Transparency effects are disabled in Windows settings.
        setSettings((prev) => ({ ...prev, vibrancy: false }));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [effectiveVibrancy]);

  // Re-check permission whenever the window regains focus (user came back
  // from System Settings).
  useEffect(() => {
    const onFocus = () => refreshPermission();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refreshPermission]);

  // Tray menu → Settings
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await tauriApi.onHotkey("ui:open-settings", () => setIsSettingsOpen(true));
    })();
    return () => unlisten?.();
  }, []);

  const handleSelectModel = async (provider: ProviderId, model: string) => {
    setActiveProvider(provider);
    setActiveModel(model);
    setIsSettingsOpen(false);

    try {
      await tauriApi.setPreference("active_provider", provider);
      await tauriApi.setPreference("active_model", model);
    } catch (e) {
      console.warn("Could not save provider/model preference:", e);
    }
  };

  const handleChangeOpacity = (val: number) => updateSettings({ windowOpacity: val });

  // The effect above applies the material natively and reverts unsupported
  // requests, so the handler only stores the preference.
  const handleToggleVibrancy = (val: boolean) => {
    updateSettings({ vibrancy: val });
  };

  const handleToggleHideFromDock = async (val: boolean) => {
    updateSettings({ hideFromDock: val });
    const applied = await tauriApi.setHideFromDock(val);
    if (val && !applied) updateSettings({ hideFromDock: false });
  };

  const handleToggleHideFromTaskbar = async (val: boolean) => {
    updateSettings({ hideFromTaskbar: val });
    await tauriApi.setHideFromTaskbar(val);
  };

  const handleToggleClickThrough = async (val: boolean) => {
    updateSettings({ clickThrough: val });
    await tauriApi.setClickThrough(val);
  };

  // Tray menu → click-through toggle (the overlay ignores mouse events then,
  // so the tray is the way back without a mouse).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await tauriApi.onHotkey("ui:toggle-click-through", () => {
        setSettings((prev) => {
          const next = !prev.clickThrough;
          void tauriApi.setClickThrough(next);
          void tauriApi.setPreference(PREF_KEYS.clickThrough, String(next));
          return { ...prev, clickThrough: next };
        });
      });
    })();
    return () => unlisten?.();
  }, []);

  const handleToggleScreenRecord = async (val: boolean) => {
    updateSettings({ excludeScreenRecord: val });
    const applied = await tauriApi.setScreenRecordingProtection(val);
    if (val && !applied) {
      // Older Windows builds cannot hide a window from capture without
      // blacking it out — leave the switch off instead of lying to the user.
      updateSettings({ excludeScreenRecord: false });
    }
  };

  const updateCustomActions = useCallback(async (actions: QuickAction[]) => {
    setCustomActions(actions);
    try {
      await tauriApi.setPreference(CUSTOM_ACTIONS_PREF, JSON.stringify(actions));
    } catch (e) {
      console.warn("Could not save custom quick actions:", e);
    }
  }, []);

  const updateProfiles = useCallback(async (next: AssistantProfile[]) => {
    setProfiles(next);
    try {
      await tauriApi.setPreference(PROFILES_PREF, JSON.stringify(next));
    } catch (e) {
      console.warn("Could not save assistant profiles:", e);
    }
  }, []);

  const updateActiveProfile = useCallback(async (id: string | null) => {
    setActiveProfileId(id);
    try {
      await tauriApi.setPreference(ACTIVE_PROFILE_PREF, id ?? "");
    } catch (e) {
      console.warn("Could not save the active profile:", e);
    }
  }, []);

  const handleOnboardingComplete = async (provider: ProviderId, model: string) => {
    setActiveProvider(provider);
    setActiveModel(model);
    setIsOnboardingOpen(false);
    try {
      await tauriApi.setPreference("active_provider", provider);
      await tauriApi.setPreference("active_model", model);
    } catch (e) {
      console.warn("Could not save the setup choices:", e);
    }
  };

  const handleToggleAlwaysOnTop = async (val: boolean) => {
    updateSettings({ alwaysOnTop: val });
    await tauriApi.setAlwaysOnTop(val);
  };

  return (
    <div className="app-backdrop h-screen w-screen flex flex-col text-slate-100 overflow-hidden font-sans">
      <ChatView
        activeProvider={activeProvider}
        activeModel={activeModel}
        settings={settings}
        alwaysOnTop={settings.alwaysOnTop}
        onToggleAlwaysOnTop={handleToggleAlwaysOnTop}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onRequirePermission={() => {
          setIsPermissionOpen(true);
          void refreshPermission();
        }}
        permissionGranted={permissionStatus ? permissionStatus.granted : true}
        customActions={customActions}
        profiles={profiles}
        activeProfileId={activeProfileId}
        onSelectProfile={updateActiveProfile}
      />

      <ProviderSettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        activeProvider={activeProvider}
        activeModel={activeModel}
        onSelectModel={handleSelectModel}
        settings={settings}
        onChangeOpacity={handleChangeOpacity}
        onToggleVibrancy={handleToggleVibrancy}
        onToggleHideFromDock={handleToggleHideFromDock}
        onToggleHideFromTaskbar={handleToggleHideFromTaskbar}
        onToggleClickThrough={handleToggleClickThrough}
        onToggleScreenRecord={handleToggleScreenRecord}
        onChangeSettings={updateSettings}
        permissionStatus={permissionStatus}
        onRefreshPermission={refreshPermission}
        onOpenPermission={() => setIsPermissionOpen(true)}
        customActions={customActions}
        onChangeCustomActions={updateCustomActions}
        profiles={profiles}
        onChangeProfiles={updateProfiles}
        onRunSetup={() => {
          setIsSettingsOpen(false);
          setIsOnboardingOpen(true);
        }}
      />

      <OnboardingWizard
        isOpen={isOnboardingOpen}
        onComplete={handleOnboardingComplete}
      />

      <PermissionModal
        isOpen={isPermissionOpen}
        status={permissionStatus}
        onClose={() => setIsPermissionOpen(false)}
        onRefresh={refreshPermission}
      />
    </div>
  );
};

export default App;
