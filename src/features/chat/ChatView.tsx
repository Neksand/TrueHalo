import React, { useState, useRef, useEffect, useCallback, useMemo } from "react";
import {
  AppSettings,
  ChatMessage,
  COMPRESSION_PRESETS,
  ContentPart,
  ProviderId,
  QuickAction,
  SessionRecord,
  TokenUsage,
} from "../../lib/types";
import { tauriApi } from "../../lib/tauri";
import { formatHotkey } from "../../lib/hotkeys";
import { BUILT_IN_ACTIONS } from "../../lib/quickActions";
import { MarkdownRenderer } from "../../components/markdown/MarkdownRenderer";
import {
  Send,
  Square,
  Sparkles,
  Settings,
  PlusCircle,
  Copy,
  Check,
  AlertCircle,
  Image as ImageIcon,
  Crop,
  Monitor,
  X,
  History,
  Pin,
  PinOff,
  EyeOff,
  ShieldAlert,
  Trash2,
  MessageSquare,
  Search,
  Pencil,
  FileText,
  FileJson,
} from "lucide-react";
import logoImg from "../../assets/logo.png";

interface ChatViewProps {
  activeProvider: ProviderId;
  activeModel: string;
  settings: AppSettings;
  alwaysOnTop: boolean;
  onToggleAlwaysOnTop: (value: boolean) => void;
  onOpenSettings: () => void;
  onRequirePermission: () => void;
  permissionGranted: boolean;
  customActions: QuickAction[];
}

/** Human readable file size. */
const formatBytes = (bytes: number): string => {
  if (!bytes) return "0 KB";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

/**
 * Only keep base64 images on the newest message that carries any. Resending the
 * whole screenshot history on every turn burns huge amounts of tokens.
 */
const stripOlderImages = (messages: ChatMessage[]): ChatMessage[] => {
  let lastImageIndex = -1;
  messages.forEach((message, index) => {
    if (message.content.some((part) => part.type === "image_base64" && part.data)) {
      lastImageIndex = index;
    }
  });

  return messages.map((message, index) => {
    if (index === lastImageIndex) return message;
    if (!message.content.some((part) => part.type === "image_base64")) return message;
    return {
      ...message,
      content: message.content.map((part) =>
        part.type === "image_base64"
          ? { type: "text" as const, text: "[previous screenshot removed to save tokens]" }
          : part
      ),
    };
  });
};

const contentToText = (contentJson: string): string => {
  try {
    const parts = JSON.parse(contentJson) as ContentPart[];
    return parts
      .map((part) => (part.type === "text" ? part.text ?? "" : "[image]"))
      .join("\n");
  } catch {
    return contentJson;
  }
};

const exportTimestamp = (value: number): string => new Date(value).toLocaleString();

export const ChatView: React.FC<ChatViewProps> = ({
  activeProvider,
  activeModel,
  settings,
  alwaysOnTop,
  onToggleAlwaysOnTop,
  onOpenSettings,
  onRequirePermission,
  permissionGranted,
  customActions,
}) => {
  const hotkeys = useMemo(
    () => ({
      toggleCompact: formatHotkey(settings.hotkeyToggle, true),
      toggle: formatHotkey(settings.hotkeyToggle),
      region: formatHotkey(settings.hotkeyRegion),
      fullscreen: formatHotkey(settings.hotkeyFullscreen),
    }),
    [settings.hotkeyToggle, settings.hotkeyRegion, settings.hotkeyFullscreen],
  );

  const quickActions = useMemo(
    () => [...BUILT_IN_ACTIONS, ...customActions],
    [customActions],
  );

  const [currentSessionId, setCurrentSessionId] = useState<string>(() => crypto.randomUUID());
  const [sessions, setSessions] = useState<SessionRecord[]>([]);
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputPrompt, setInputPrompt] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [currentStreamId, setCurrentStreamId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [toast, setToast] = useState<{ text: string; detail?: string } | null>(null);

  const [isCapturing, setIsCapturing] = useState(false);
  const [attachedImage, setAttachedImage] = useState<{
    data: string;
    mediaType: string;
    previewUrl: string;
    bytes: number;
    originalBytes: number;
    width: number;
    height: number;
    compressed: boolean;
  } | null>(null);
  const [extractedText, setExtractedText] = useState<string | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const renameCancelledRef = useRef(false);

  const loadSessions = useCallback(async (query: string) => {
    try {
      const list = query.trim()
        ? await tauriApi.searchSessions(query)
        : await tauriApi.listSessions();
      setSessions(list);
    } catch (e) {
      console.warn("Failed to load sessions from SQLite:", e);
    }
  }, []);

  // Search both titles and message text, debounced so typing stays smooth.
  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadSessions(searchQuery);
    }, 220);
    return () => window.clearTimeout(timer);
  }, [searchQuery, loadSessions]);

  // Auto-dismiss export confirmations.
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 5000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const handleCapture = useCallback(
    async (interactive: boolean) => {
      setIsCapturing(true);
      setErrorMessage(null);
      try {
        const preset = COMPRESSION_PRESETS[settings.compressionPreset];
        const result = await tauriApi.captureScreen({
          interactive,
          show_cursor: false,
          compress: settings.compressScreenshots,
          max_dimension: preset.maxDimension,
          quality: preset.quality,
          hide_window: settings.hideWindowOnCapture,
          ocr: settings.textMode !== "image",
        });

        if (result.status === "success") {
          const ocrText = result.ocr_text?.trim() ?? "";
          const minimum = settings.textMode === "auto" ? 12 : 1;
          const useText =
            settings.textMode !== "image" && ocrText.length >= minimum;

          if (useText) {
            setExtractedText(ocrText);
            setAttachedImage(null);
          } else {
            setExtractedText(null);
            setAttachedImage({
              data: result.base64_data,
              mediaType: result.media_type,
              previewUrl: `data:${result.media_type};base64,${result.base64_data}`,
              bytes: result.file_size_bytes,
              originalBytes: result.original_size_bytes,
              width: result.width,
              height: result.height,
              compressed: result.compressed,
            });
          }
          // Make sure the user actually sees the captured overlay again.
          await tauriApi.showWindow();
        } else if (result.status === "permission_required") {
          onRequirePermission();
        }
      } catch (err: unknown) {
        const message =
          typeof err === "string"
            ? err
            : err instanceof Error
              ? err.message
              : "Could not take a screenshot";
        setErrorMessage(message);
      } finally {
        setIsCapturing(false);
      }
    },
    [
      settings.compressionPreset,
      settings.compressScreenshots,
      settings.hideWindowOnCapture,
      settings.textMode,
      onRequirePermission,
    ]
  );

  // Register hotkey listeners from Tauri backend.
  //
  // Registration is async, so it must not depend on `handleCapture`: the effect
  // would re-run on every render and leak a listener each time (which made one
  // hotkey press start two captures). The handler is kept in a ref instead.
  const captureHotkeyRef = useRef(handleCapture);
  useEffect(() => {
    captureHotkeyRef.current = handleCapture;
  }, [handleCapture]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    const register = async (event: string, interactive: boolean) => {
      const unlisten = await tauriApi.onHotkey(event, () => {
        captureHotkeyRef.current(interactive);
      });
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    };

    void register("hotkey:capture-region", true);
    void register("hotkey:capture-fullscreen", false);

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, isStreaming]);

  // Adjust textarea height dynamically
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(
        textareaRef.current.scrollHeight,
        180
      )}px`;
    }
  }, [inputPrompt]);

  const handleTogglePin = () => {
    onToggleAlwaysOnTop(!alwaysOnTop);
  };

  const sessionUsage = useMemo<TokenUsage>(() => {
    return messages.reduce<TokenUsage>(
      (acc, message) => {
        if (!message.usage) return acc;
        return {
          prompt_tokens: (acc.prompt_tokens ?? 0) + (message.usage.prompt_tokens ?? 0),
          completion_tokens:
            (acc.completion_tokens ?? 0) + (message.usage.completion_tokens ?? 0),
          total_tokens: (acc.total_tokens ?? 0) + (message.usage.total_tokens ?? 0),
        };
      },
      { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }
    );
  }, [messages]);

  const handleQuickHide = async () => {
    try {
      await tauriApi.quickHide();
    } catch (e) {
      console.error(e);
    }
  };

  const handleSelectSession = async (session: SessionRecord) => {
    setCurrentSessionId(session.id);
    setIsSidebarOpen(false);
    try {
      const dbMsgs = await tauriApi.getSessionMessages(session.id);
      const parsed: ChatMessage[] = dbMsgs.map((m) => {
        let content: ContentPart[] = [];
        try {
          content = JSON.parse(m.content_json);
        } catch {
          content = [{ type: "text", text: m.content_json }];
        }
        return {
          id: m.id,
          role: m.role as any,
          content,
          timestamp: m.timestamp,
        };
      });
      setMessages(parsed);
    } catch (err) {
      console.error("Failed to load session messages:", err);
    }
  };

  const handleRenameStart = (e: React.MouseEvent, session: SessionRecord) => {
    e.stopPropagation();
    setRenamingId(session.id);
    setRenameValue(session.title);
  };

  const handleRenameCommit = async (sessionId: string) => {
    const title = renameValue.trim();
    setRenamingId(null);
    if (!title) return;
    try {
      await tauriApi.renameSession(sessionId, title);
      await loadSessions(searchQuery);
    } catch (err) {
      setErrorMessage(
        typeof err === "string" ? err : "Could not rename the conversation",
      );
    }
  };

  const handleExport = async (
    e: React.MouseEvent,
    session: SessionRecord,
    format: "md" | "json",
  ) => {
    e.stopPropagation();
    try {
      const records = await tauriApi.getSessionMessages(session.id);
      const rows = records.map((record) => ({
        role: record.role,
        timestamp: record.timestamp,
        text: contentToText(record.content_json),
      }));

      let content: string;
      let filename: string;

      if (format === "json") {
        content = JSON.stringify(
          {
            title: session.title,
            provider: session.provider,
            model: session.model,
            createdAt: new Date(session.created_at).toISOString(),
            messages: rows.map((row) => ({
              role: row.role,
              timestamp: new Date(row.timestamp).toISOString(),
              text: row.text,
            })),
          },
          null,
          2,
        );
        filename = `${session.title}.json`;
      } else {
        const header = `# ${session.title}\n\n- Provider: ${session.provider}\n- Model: ${session.model}\n- Created: ${exportTimestamp(session.created_at)}\n\n---\n\n`;
        const body = rows
          .map(
            (row) =>
              `## ${row.role === "user" ? "User" : "Assistant"} · ${exportTimestamp(
                row.timestamp,
              )}\n\n${row.text}\n`,
          )
          .join("\n---\n\n");
        content = header + body;
        filename = `${session.title}.md`;
      }

      const path = await tauriApi.saveExport(filename, content);
      setToast({ text: "Export saved", detail: path });
    } catch (err) {
      setErrorMessage(
        typeof err === "string" ? err : "Could not export the conversation",
      );
    }
  };

  const handleDeleteSession = async (e: React.MouseEvent, sessionId: string) => {
    e.stopPropagation();
    try {
      await tauriApi.deleteSession(sessionId);
      if (sessionId === currentSessionId) {
        handleNewChat();
      }
      await loadSessions(searchQuery);
    } catch (err) {
      console.error(err);
    }
  };

  const handleSendMessage = async (
    overridePrompt?: string,
    action?: QuickAction,
  ) => {
    const trimmed = (overridePrompt ?? inputPrompt).trim();
    const ocrText = extractedText?.trim() ?? "";
    if ((!trimmed && !attachedImage && !ocrText) || isStreaming) return;

    setErrorMessage(null);
    const contentParts: ContentPart[] = [];
    const text = [trimmed, ocrText].filter(Boolean).join("\n\n");

    if (text) {
      contentParts.push({ type: "text", text });
    }

    if (attachedImage) {
      contentParts.push({
        type: "image_base64",
        media_type: attachedImage.mediaType,
        data: attachedImage.data,
      });
    }

    const userMessageId = crypto.randomUUID();
    const userMessage: ChatMessage = {
      id: userMessageId,
      role: "user",
      content: contentParts,
      timestamp: Date.now(),
    };

    const streamId = crypto.randomUUID();
    setCurrentStreamId(streamId);

    const assistantPlaceholderId = crypto.randomUUID();
    const assistantMessage: ChatMessage = {
      id: assistantPlaceholderId,
      role: "assistant",
      content: [{ type: "text", text: "" }],
      timestamp: Date.now(),
      isStreaming: true,
    };

    // If starting a fresh session, record it in SQLite
    if (messages.length === 0) {
      const title =
        (action?.label || trimmed || ocrText).slice(0, 32) || "Screen Analysis";
      try {
        await tauriApi.createSession(currentSessionId, title, activeProvider, activeModel);
        loadSessions(searchQuery);
      } catch (e) {
        console.warn("Could not create session in SQLite:", e);
      }
    }

    // Save user message in SQLite
    try {
      await tauriApi.saveMessage(
        userMessageId,
        currentSessionId,
        "user",
        JSON.stringify(contentParts)
      );
    } catch (e) {
      console.warn("Could not save user message:", e);
    }

    const newHistory = [...messages, userMessage];
    setMessages([...newHistory, assistantMessage]);
    if (!overridePrompt) setInputPrompt("");
    setAttachedImage(null);
    setExtractedText(null);
    setIsStreaming(true);

    let accumulatedText = "";

    try {
      const unlisten = await tauriApi.listenStream(
        streamId,
        (delta) => {
          accumulatedText += delta;
          setMessages((prev) =>
            prev.map((msg) => {
              if (msg.id === assistantPlaceholderId) {
                return {
                  ...msg,
                  content: [{ type: "text", text: accumulatedText }],
                };
              }
              return msg;
            })
          );
        },
        async (usage?: TokenUsage) => {
          setIsStreaming(false);
          setCurrentStreamId(null);
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === assistantPlaceholderId
                ? { ...msg, isStreaming: false, usage }
                : msg
            )
          );

          // Save completed assistant message in SQLite
          try {
            await tauriApi.saveMessage(
              assistantPlaceholderId,
              currentSessionId,
              "assistant",
              JSON.stringify([{ type: "text", text: accumulatedText }])
            );
          } catch (e) {
            console.warn("Could not save assistant message:", e);
          }

          unlisten();
        },
        (error) => {
          setIsStreaming(false);
          setCurrentStreamId(null);
          setErrorMessage(error);
          setMessages((prev) =>
            prev.map((msg) =>
              msg.id === assistantPlaceholderId
                ? { ...msg, isStreaming: false }
                : msg
            )
          );
          unlisten();
        }
      );

      // Start stream — only the newest screenshot is sent, older ones are dropped
      // so long conversations stay cheap.
      await tauriApi.startStream({
        stream_id: streamId,
        provider: activeProvider,
        model: activeModel,
        messages: [
          ...(action?.system
            ? [
                {
                  role: "system" as const,
                  content: [{ type: "text" as const, text: action.system }],
                },
              ]
            : []),
          ...stripOlderImages(newHistory).map((m) => ({
            role: m.role,
            content: m.content,
          })),
        ],
      });
    } catch (err: any) {
      setIsStreaming(false);
      setCurrentStreamId(null);
      const errStr = typeof err === "string" ? err : err.message || "Failed to start streaming";
      setErrorMessage(errStr);
    }
  };

  const handleStopGeneration = async () => {
    if (currentStreamId) {
      await tauriApi.cancelStream(currentStreamId);
      setIsStreaming(false);
      setCurrentStreamId(null);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (isStreaming) {
        handleStopGeneration();
      } else {
        handleQuickHide();
      }
    }
  };

  const handleCopyMessage = (id: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleNewChat = () => {
    if (isStreaming) {
      handleStopGeneration();
    }
    setCurrentSessionId(crypto.randomUUID());
    setMessages([]);
    setErrorMessage(null);
    setAttachedImage(null);
    setExtractedText(null);
  };

  const handleImageFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
      setExtractedText(null);
      const result = reader.result as string;
      const base64Data = result.split(",")[1];

      // Read natural dimensions so the preview shows real numbers.
      const probe = new Image();
      probe.onload = () => {
        setAttachedImage({
          data: base64Data,
          mediaType: file.type || "image/png",
          previewUrl: result,
          bytes: file.size,
          originalBytes: file.size,
          width: probe.naturalWidth,
          height: probe.naturalHeight,
          compressed: false,
        });
      };
      probe.onerror = () => {
        setAttachedImage({
          data: base64Data,
          mediaType: file.type || "image/png",
          previewUrl: result,
          bytes: file.size,
          originalBytes: file.size,
          width: 0,
          height: 0,
          compressed: false,
        });
      };
      probe.src = result;
    };
    reader.readAsDataURL(file);
    e.target.value = "";
  };

  return (
    <div className="flex-1 flex h-full overflow-hidden bg-transparent relative">
      {/* Sidebar: Conversation History Drawer */}
      {isSidebarOpen && (
        <aside className="w-64 border-r border-surface-border bg-surface-card/95 backdrop-blur-md flex flex-col z-20 animate-in slide-in-from-left-4 duration-150">
          <div className="p-3 border-b border-surface-border flex items-center justify-between">
            <span className="text-xs font-semibold text-slate-200 flex items-center gap-2">
              <History className="w-3.5 h-3.5 text-halo-accent" />
              History
            </span>
            <button
              onClick={() => setIsSidebarOpen(false)}
              className="text-slate-400 hover:text-slate-200 text-xs px-2 py-0.5 rounded hover:bg-surface-subtle"
            >
              ✕
            </button>
          </div>

          <div className="p-2 space-y-2">
            <button
              onClick={() => {
                handleNewChat();
                setIsSidebarOpen(false);
              }}
              className="w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-surface-subtle border border-surface-border hover:border-halo-accent/40 text-xs text-slate-200 font-medium transition-colors"
            >
              <PlusCircle className="w-3.5 h-3.5 text-halo-accent" />
              <span>New Conversation</span>
            </button>

            <div className="relative">
              <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-1/2 -translate-y-1/2" />
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search history"
                className="w-full pl-8 pr-7 py-1.5 rounded-lg bg-surface-deep border border-surface-border text-[11px] text-slate-200 placeholder:text-slate-600 focus:outline-none focus:border-halo-accent/50"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery("")}
                  className="absolute right-1.5 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 text-[10px] px-1"
                  title="Clear search"
                >
                  ✕
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 overflow-y-auto px-2 space-y-1">
            {sessions.length === 0 ? (
              <div className="p-4 text-center text-xs text-slate-500">
                {searchQuery
                  ? `No matches for “${searchQuery}”.`
                  : "No past conversations yet. Your next capture starts one."}
              </div>
            ) : (
              sessions.map((s) => (
                <div
                  key={s.id}
                  onClick={() => handleSelectSession(s)}
                  className={`group p-2 rounded-lg text-xs cursor-pointer flex items-center justify-between gap-1 transition-colors ${
                    s.id === currentSessionId
                      ? "bg-halo-accent/15 text-slate-100 border border-halo-accent/40"
                      : "text-slate-400 hover:text-slate-200 hover:bg-surface-subtle"
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <MessageSquare className="w-3.5 h-3.5 shrink-0 opacity-70" />
                    {renamingId === s.id ? (
                      <input
                        autoFocus
                        value={renameValue}
                        onChange={(e) => setRenameValue(e.target.value)}
                        onClick={(e) => e.stopPropagation()}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            e.currentTarget.blur();
                          }
                          if (e.key === "Escape") {
                            renameCancelledRef.current = true;
                            setRenamingId(null);
                            e.currentTarget.blur();
                          }
                        }}
                        onBlur={() => {
                          if (renameCancelledRef.current) {
                            renameCancelledRef.current = false;
                            return;
                          }
                          void handleRenameCommit(s.id);
                        }}
                        className="w-full min-w-0 bg-surface-deep border border-halo-accent/40 rounded px-1.5 py-0.5 text-[11px] text-slate-100 focus:outline-none"
                      />
                    ) : (
                      <span className="truncate" title={s.title}>
                        {s.title}
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-0.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      onClick={(e) => handleRenameStart(e, s)}
                      className="p-1 hover:text-halo-accent transition-colors"
                      title="Rename conversation"
                    >
                      <Pencil className="w-3 h-3" />
                    </button>
                    <button
                      onClick={(e) => handleExport(e, s, "md")}
                      className="p-1 hover:text-sky-300 transition-colors"
                      title="Export as Markdown"
                    >
                      <FileText className="w-3 h-3" />
                    </button>
                    <button
                      onClick={(e) => handleExport(e, s, "json")}
                      className="p-1 hover:text-sky-300 transition-colors"
                      title="Export as JSON"
                    >
                      <FileJson className="w-3 h-3" />
                    </button>
                    <button
                      onClick={(e) => handleDeleteSession(e, s.id)}
                      className="p-1 hover:text-rose-400 transition-colors"
                      title="Delete conversation"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>

          {toast && (
            <div className="m-2 p-2 rounded-lg border border-emerald-800/40 bg-emerald-950/30">
              <div className="flex items-center gap-1.5 text-[11px] font-medium text-emerald-200">
                <Check className="w-3 h-3" />
                {toast.text}
              </div>
              {toast.detail && (
                <div
                  className="text-[10px] text-emerald-200/60 break-all mt-1"
                  title={toast.detail}
                >
                  {toast.detail}
                </div>
              )}
            </div>
          )}
        </aside>
      )}

      {/* Main Chat Container */}
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        {/* Top Header */}
        <header
          data-tauri-drag-region="deep"
          className="h-12 border-b border-surface-border bg-surface-card/60 backdrop-blur-md flex items-center justify-between px-4 select-none shrink-0 cursor-default"
        >
          <div className="flex items-center gap-3 pl-16">
            <button
              onClick={() => setIsSidebarOpen(!isSidebarOpen)}
              className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              title="Toggle Conversation History"
            >
              <History className="w-4 h-4" />
            </button>

            <img
              src={logoImg}
              alt="TrueHalo Logo"
              className="w-6 h-6 rounded-full object-cover border border-halo-glow/50 shadow-halo-sm"
            />
            <div className="flex items-center gap-2">
              <span className="text-xs font-semibold text-slate-100 tracking-wide">
                TrueHalo
              </span>
              <span className="text-[10px] text-slate-500 font-mono">Overlay</span>
            </div>
          </div>

          {/* Model Badge & Controls */}
          <div className="flex items-center gap-2">
            {sessionUsage.total_tokens ? (
              <span
                className="hidden sm:inline-flex items-center gap-1 px-2 py-1 rounded-lg bg-surface-subtle/70 border border-surface-border text-[10px] text-slate-400 font-mono"
                title={`Prompt: ${sessionUsage.prompt_tokens ?? 0} · Completion: ${sessionUsage.completion_tokens ?? 0}`}
              >
                Σ {sessionUsage.total_tokens.toLocaleString("en-US")} tok
              </span>
            ) : null}

            <button
              onClick={onOpenSettings}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-surface-subtle border border-surface-border hover:border-halo-accent/50 text-xs text-slate-300 hover:text-slate-100 transition-colors"
              title="Configure model and provider"
            >
              <Sparkles className="w-3.5 h-3.5 text-halo-accent" />
              <span className="font-medium capitalize">{activeProvider}:</span>
              <span className="text-slate-400 font-mono text-[11px] truncate max-w-[130px]">
                {activeModel}
              </span>
            </button>

            <button
              onClick={handleTogglePin}
              className={`p-1.5 rounded-lg transition-colors ${
                alwaysOnTop
                  ? "text-halo-accent bg-halo-accent/15"
                  : "text-slate-400 hover:text-slate-200 hover:bg-surface-subtle"
              }`}
              title={alwaysOnTop ? "Always On Top (Enabled)" : "Pin on top"}
            >
              {alwaysOnTop ? <Pin className="w-4 h-4 fill-current" /> : <PinOff className="w-4 h-4" />}
            </button>

            <button
              onClick={handleQuickHide}
              className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              title={`Quick Hide (${hotkeys.toggleCompact} or Esc)`}
            >
              <EyeOff className="w-4 h-4" />
            </button>

            <button
              onClick={handleNewChat}
              className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              title="New Conversation"
            >
              <PlusCircle className="w-4 h-4" />
            </button>

            <button
              onClick={onOpenSettings}
              className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              title="Settings"
            >
              <Settings className="w-4 h-4" />
            </button>
          </div>
        </header>

        {/* Message Area */}
        <div className="flex-1 overflow-y-auto px-4 py-6 space-y-6">
          {messages.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-center px-4 select-none max-w-md mx-auto">
              <div className="relative mb-5 group">
                <div className="absolute -inset-1 rounded-full bg-gradient-to-r from-halo-accent to-halo-glow opacity-30 blur-lg group-hover:opacity-50 transition-opacity"></div>
                <img
                  src={logoImg}
                  alt="Halo Angel"
                  className="relative w-20 h-20 rounded-2xl object-cover border border-halo-accent/40 shadow-halo"
                />
              </div>
              <h2 className="text-lg font-semibold text-slate-100 mb-1">
                TrueHalo AI Overlay
              </h2>
              <p className="text-xs text-slate-400 leading-relaxed mb-6">
                Fast AI overlay companion. Press <kbd className="px-1 py-0.5 rounded bg-surface-subtle border border-surface-border text-slate-300 font-mono text-[10px]">{hotkeys.toggle}</kbd> to toggle, or capture screens with <kbd className="px-1 py-0.5 rounded bg-surface-subtle border border-surface-border text-slate-300 font-mono text-[10px]">{hotkeys.region}</kbd>.
              </p>

              <button
                onClick={() => handleCapture(true)}
                className="w-full mb-3 flex items-center justify-center gap-2 px-4 py-3 rounded-xl bg-halo-accent hover:bg-halo-accent/90 text-white text-xs font-semibold shadow-halo transition-colors"
              >
                <Crop className="w-4 h-4" />
                Start with a screenshot
                <span className="ml-1 px-1.5 py-0.5 rounded bg-white/15 font-mono text-[10px] font-normal">
                  {hotkeys.region}
                </span>
              </button>

              <div className="grid grid-cols-1 gap-2 w-full text-left">
                <button
                  onClick={() => handleCapture(false)}
                  className="flex items-center gap-2 p-2.5 rounded-lg border border-surface-border bg-surface-card/60 hover:bg-surface-card hover:border-halo-accent/40 text-xs text-slate-200 transition-all text-left"
                >
                  <Monitor className="w-4 h-4 text-sky-400 shrink-0" />
                  <span>Capture entire screen ({hotkeys.fullscreen})</span>
                </button>
                <button
                  onClick={() => setInputPrompt("Explain how this code works in Rust")}
                  className="p-2.5 rounded-lg border border-surface-border bg-surface-card/60 hover:bg-surface-card hover:border-halo-accent/40 text-xs text-slate-300 transition-all text-left"
                >
                  "Explain how this code works in Rust"
                </button>
              </div>
            </div>
          ) : (
            messages.map((message) => {
              const isUser = message.role === "user";
              const textContent =
                message.content.find((c) => c.type === "text")?.text || "";

              return (
                <div
                  key={message.id}
                  className={`flex gap-3 max-w-3xl mx-auto ${
                    isUser ? "justify-end" : "justify-start"
                  }`}
                >
                  {!isUser && (
                    <div className="w-7 h-7 rounded-lg overflow-hidden shrink-0 border border-halo-accent/30 shadow-halo-sm mt-0.5">
                      <img
                        src={logoImg}
                        alt="TrueHalo"
                        className="w-full h-full object-cover"
                      />
                    </div>
                  )}

                  <div
                    className={`selectable group relative max-w-[85%] rounded-2xl px-4 py-3 text-xs leading-relaxed ${
                      isUser
                        ? "bg-halo-accent text-white font-medium rounded-tr-sm shadow-halo-sm"
                        : "bg-surface-card border border-surface-border text-slate-200 rounded-tl-sm shadow-halo-card"
                    }`}
                  >
                    {/* Multimodal image preview if attached */}
                    {message.content.map((part, index) => {
                      if (part.type === "image_base64" && part.data) {
                        return (
                          <div key={index} className="mb-2 rounded-lg overflow-hidden border border-white/20 max-w-sm">
                            <img
                              src={`data:${part.media_type};base64,${part.data}`}
                              alt="Screenshot Attachment"
                              className="max-h-60 w-auto object-contain rounded-lg"
                            />
                          </div>
                        );
                      }
                      return null;
                    })}

                    {/* Message Text */}
                    {isUser ? (
                      <div className="whitespace-pre-wrap">{textContent}</div>
                    ) : (
                      <>
                        <MarkdownRenderer content={textContent} />
                        {message.isStreaming && (
                          <span className="inline-block w-1.5 h-3.5 ml-1 bg-halo-accent animate-pulse align-middle" />
                        )}
                      </>
                    )}

                    {/* Actions for assistant messages */}
                    {!isUser && textContent && !message.isStreaming && (
                      <div className="flex items-center gap-2 mt-2 pt-2 border-t border-surface-border/50 text-[11px] text-slate-400">
                        <button
                          onClick={() => handleCopyMessage(message.id, textContent)}
                          className="flex items-center gap-1 hover:text-slate-200 transition-colors"
                        >
                          {copiedId === message.id ? (
                            <>
                              <Check className="w-3 h-3 text-emerald-400" />
                              <span className="text-emerald-400">Copied</span>
                            </>
                          ) : (
                            <>
                              <Copy className="w-3 h-3" />
                              <span>Copy</span>
                            </>
                          )}
                        </button>
                        {message.usage?.total_tokens ? (
                          <span className="text-slate-500 font-mono">
                            · {message.usage.total_tokens.toLocaleString("en-US")} tok
                          </span>
                        ) : null}
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Screen Recording permission banner */}
        {!permissionGranted && (
          <div className="mx-4 mb-2 p-3 rounded-lg bg-amber-950/50 border border-amber-800/70 text-amber-100 text-xs flex items-center justify-between shadow-lg">
            <div className="flex items-center gap-2">
              <ShieldAlert className="w-4 h-4 shrink-0 text-amber-400" />
              <span>
                Screen recording access is not granted — screenshots are unavailable.
              </span>
            </div>
            <button
              onClick={onRequirePermission}
              className="text-[11px] font-medium px-2.5 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-600/50 transition-colors ml-3 shrink-0"
            >
              Allow
            </button>
          </div>
        )}

        {/* Error Banner */}
        {errorMessage && (
          <div className="mx-4 mb-2 p-3 rounded-lg bg-rose-950/60 border border-rose-800/80 text-rose-200 text-xs flex items-center justify-between shadow-lg">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4 shrink-0 text-rose-400" />
              <span>{errorMessage}</span>
            </div>
            <button
              onClick={onOpenSettings}
              className="text-[11px] underline underline-offset-2 hover:text-white font-medium ml-3 shrink-0"
            >
              Settings
            </button>
          </div>
        )}

        {/* Local OCR preview */}
        {extractedText !== null && (
          <div className="mx-4 mb-2 p-2.5 rounded-lg bg-surface-card border border-sky-700/50 shadow-halo-sm">
            <div className="flex items-center justify-between mb-1.5">
              <span className="text-[11px] font-medium text-slate-100 flex items-center gap-1.5">
                <FileText className="w-3.5 h-3.5 text-sky-400" />
                Text from screenshot · edit before sending
              </span>
              <button
                onClick={() => setExtractedText(null)}
                className="p-1 rounded-md text-slate-400 hover:text-rose-400 hover:bg-rose-950/30 transition-colors"
                title="Remove text"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
            <textarea
              value={extractedText}
              onChange={(e) => setExtractedText(e.target.value)}
              rows={4}
              className="w-full resize-y bg-black/30 border border-surface-border rounded-lg px-2.5 py-2 text-[11px] font-mono text-slate-200 focus:outline-none focus:border-sky-600/60"
            />
            <p className="text-[10px] text-slate-500 mt-1">
              Recognized on-device. Only this text is sent — the screenshot is discarded.
            </p>
          </div>
        )}

        {/* Screen Capture / File Preview */}
        {attachedImage && (
          <div className="mx-4 mb-2 p-2 rounded-lg bg-surface-card border border-halo-accent/50 flex items-center gap-3 shadow-halo-sm">
            <img
              src={attachedImage.previewUrl}
              alt="Screenshot Preview"
              className="w-14 h-14 object-contain bg-black/40 rounded border border-surface-border"
            />
            <div className="flex-1 text-xs min-w-0">
              <span className="font-medium text-slate-100 flex items-center gap-1.5">
                <Crop className="w-3.5 h-3.5 text-halo-accent" />
                Screenshot attached
              </span>
              <p className="text-[11px] text-slate-400">
                {attachedImage.width}×{attachedImage.height} px ·{" "}
                {attachedImage.compressed ? (
                  <>
                    compressed {formatBytes(attachedImage.originalBytes)} →{" "}
                    <span className="text-emerald-300">{formatBytes(attachedImage.bytes)}</span>
                  </>
                ) : (
                  formatBytes(attachedImage.bytes)
                )}
              </p>
            </div>
            <div className="flex items-center gap-1">
              <button
                onClick={() => handleCapture(true)}
                className="px-2 py-1 text-xs text-slate-300 hover:text-white bg-surface-subtle hover:bg-surface-border rounded transition-colors"
                title="Retake screenshot"
              >
                Retake
              </button>
              <button
                onClick={() => setAttachedImage(null)}
                className="p-1.5 rounded-md text-slate-400 hover:text-rose-400 hover:bg-rose-950/30 transition-colors"
                title="Remove image"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}

        {/* Quick actions */}
        <div className="px-4 pt-2">
          <div className="max-w-3xl mx-auto flex items-center gap-1.5 overflow-x-auto pb-1">
            <span className="text-[10px] uppercase tracking-wide text-slate-500 mr-1 shrink-0">
              Quick actions
            </span>
            {quickActions.map((action) => {
              const hasContext = Boolean(
                attachedImage || extractedText?.trim() || inputPrompt.trim(),
              );
              const disabled = !hasContext || isStreaming || isCapturing;
              return (
                <button
                  key={action.id}
                  onClick={() => handleSendMessage(action.prompt, action)}
                  disabled={disabled}
                  title={
                    hasContext
                      ? action.prompt
                      : "Capture a screenshot or type a question first"
                  }
                  className="shrink-0 px-2.5 py-1 rounded-full border border-surface-border bg-surface-card/70 text-[10px] text-slate-300 hover:text-slate-100 hover:border-halo-accent/50 hover:bg-surface-subtle disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                >
                  {action.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Input Area */}
        <div className="p-4 border-t border-surface-border bg-surface-card/40 backdrop-blur-md">
          <div className="max-w-3xl mx-auto flex items-end gap-2 bg-[#0b0c10] border border-surface-border focus-within:border-halo-accent rounded-xl p-2 transition-colors shadow-halo-card">
            <input
              type="file"
              ref={fileInputRef}
              onChange={handleImageFileSelect}
              accept="image/*"
              className="hidden"
            />

            {/* Region Capture button */}
            <button
              type="button"
              onClick={() => handleCapture(true)}
              disabled={isCapturing}
              className={`p-2 rounded-lg transition-colors ${
                isCapturing
                  ? "text-halo-accent animate-pulse"
                  : "text-slate-400 hover:text-halo-accent hover:bg-surface-subtle"
              }`}
              title={`Capture region (${hotkeys.region})`}
            >
              <Crop className="w-4 h-4" />
            </button>

            {/* Fullscreen Capture button */}
            <button
              type="button"
              onClick={() => handleCapture(false)}
              disabled={isCapturing}
              className="p-2 rounded-lg text-slate-400 hover:text-sky-400 hover:bg-surface-subtle transition-colors"
              title={`Capture fullscreen (${hotkeys.fullscreen})`}
            >
              <Monitor className="w-4 h-4" />
            </button>

            {/* Attach local file button */}
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="p-2 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-surface-subtle transition-colors"
              title="Attach image from disk"
            >
              <ImageIcon className="w-4 h-4" />
            </button>

            <textarea
              ref={textareaRef}
              value={inputPrompt}
              onChange={(e) => setInputPrompt(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={
                attachedImage
                  ? "Ask about this screenshot..."
                  : "Ask anything or press Shift+Enter for new line..."
              }
              rows={1}
              className="flex-1 bg-transparent text-xs text-slate-100 placeholder-slate-500 outline-none resize-none py-1.5 px-1 max-h-44"
            />

            {isStreaming ? (
              <button
                onClick={handleStopGeneration}
                className="p-2 rounded-lg bg-rose-600 hover:bg-rose-500 text-white shadow-halo-sm transition-all"
                title="Stop generation (Esc)"
              >
                <Square className="w-4 h-4 fill-current" />
              </button>
            ) : (
              <button
                onClick={() => handleSendMessage()}
                disabled={!inputPrompt.trim() && !attachedImage && !extractedText?.trim()}
                className="p-2 rounded-lg bg-halo-accent hover:bg-halo-accent/90 disabled:opacity-30 disabled:cursor-not-allowed text-white shadow-halo-sm transition-all"
                title="Send message (Enter)"
              >
                <Send className="w-4 h-4" />
              </button>
            )}
          </div>
          <div className="max-w-3xl mx-auto mt-1 px-1 flex justify-between items-center text-[10px] text-slate-500">
            <span>Enter to send · Shift+Enter for newline · Esc to Quick Hide</span>
            <span>{hotkeys.toggleCompact}: Toggle · {hotkeys.region}: Region · {hotkeys.fullscreen}: Fullscreen</span>
          </div>
        </div>
      </div>
    </div>
  );
};
