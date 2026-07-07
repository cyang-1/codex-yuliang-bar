import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useFormattedResetTime } from "../hooks/useFormattedResetTime";
import { useProviders } from "../hooks/useProviders";
import { getSettingsSnapshot, refreshProvidersIfStale } from "../lib/tauri";
import type {
  BootstrapState,
  ProviderUsageSnapshot,
  RateWindowSnapshot,
  SettingsSnapshot,
} from "../types/bridge";
import { FLOAT_BAR_CONFIG_CHANGED_EVENT, resizeFloatBar } from "./api";
import "./FloatBar.css";

const CODEX_PROVIDER_ID = "codex";

function GaugeIcon() {
  return (
    <svg className="floatbar__brand-mark" viewBox="0 0 44 44" fill="none" aria-hidden="true">
      <circle cx="22" cy="22" r="18" stroke="currentColor" strokeWidth="4" />
      <path
        d="M14 20a11 11 0 0 1 16 0M17 16l-2-3M22 14v-4M27 16l2-3"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
      />
      <path d="M22 19c4 6 6 10 6 13a6 6 0 1 1-12 0c0-3 2-7 6-13Z" fill="currentColor" />
    </svg>
  );
}

function ClockIcon() {
  return (
    <svg className="floatbar__metric-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z" stroke="currentColor" strokeWidth="2.2" />
      <path d="M12 7v5l3 2" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
    </svg>
  );
}

function CalendarIcon() {
  return (
    <svg className="floatbar__metric-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M7 3v4M17 3v4M4.5 9h15M6 5h12a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z"
        stroke="currentColor"
        strokeWidth="2.2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function formatPercent(value: number): string {
  return `${Math.round(Math.max(0, Math.min(100, value)))}%`;
}

function quotaTone(window: RateWindowSnapshot, highRemaining: number, critRemaining: number) {
  const remaining = Math.max(0, Math.min(100, window.remainingPercent));
  if (window.isExhausted || remaining <= critRemaining) return "crit";
  if (remaining <= highRemaining) return "warn";
  return "ok";
}

function QuotaBlock({
  icon,
  label,
  window,
  reset,
  tone,
}: {
  icon: "clock" | "calendar";
  label: string;
  window: RateWindowSnapshot | null;
  reset: string | null;
  tone: "ok" | "warn" | "crit";
}) {
  const remaining = window ? Math.max(0, Math.min(100, window.remainingPercent)) : 0;
  return (
    <section className={`floatbar__quota floatbar__quota--${tone}`} data-tauri-drag-region>
      <div className="floatbar__quota-label" data-tauri-drag-region>
        {icon === "clock" ? <ClockIcon /> : <CalendarIcon />}
        <span data-tauri-drag-region>{label}</span>
      </div>
      <div className="floatbar__quota-percent" data-tauri-drag-region>
        {window ? formatPercent(remaining) : "--"}
      </div>
      <div className="floatbar__meter" data-tauri-drag-region>
        <span style={{ width: `${remaining}%` }} data-tauri-drag-region />
      </div>
      <div className="floatbar__reset-text" data-tauri-drag-region>
        {reset ?? "等待刷新"}
      </div>
    </section>
  );
}

function CodexCard({
  provider,
  highRemaining,
  critRemaining,
  resetRelative,
}: {
  provider: ProviderUsageSnapshot;
  highRemaining: number;
  critRemaining: number;
  resetRelative: boolean;
}) {
  const weeklyWindow = provider.secondary ?? null;
  const sessionReset = useFormattedResetTime(
    provider.primary.resetsAt,
    provider.primary.resetDescription,
    resetRelative,
  );
  const weeklyReset = useFormattedResetTime(
    weeklyWindow?.resetsAt ?? null,
    weeklyWindow?.resetDescription ?? null,
    resetRelative,
  );
  const sessionTone = provider.error ? "crit" : quotaTone(provider.primary, highRemaining, critRemaining);
  const weeklyTone = provider.error || !weeklyWindow ? "crit" : quotaTone(weeklyWindow, highRemaining, critRemaining);
  const plan = provider.planName ?? "ChatGPT Plus";

  return (
    <article className="floatbar__card" data-tauri-drag-region>
      <header className="floatbar__header" data-tauri-drag-region>
        <div className="floatbar__brand" data-tauri-drag-region>
          <GaugeIcon />
          <div data-tauri-drag-region>
            <strong data-tauri-drag-region>Codex</strong>
            <span data-tauri-drag-region>{provider.error ? "读取失败" : "实时余量"}</span>
          </div>
        </div>
        <span className="floatbar__plan" data-tauri-drag-region>
          {plan}
        </span>
      </header>

      <QuotaBlock
        icon="clock"
        label="5 小时"
        window={provider.error ? null : provider.primary}
        reset={provider.error ? provider.error : sessionReset}
        tone={sessionTone}
      />
      <QuotaBlock
        icon="calendar"
        label="周限额"
        window={provider.error ? null : weeklyWindow}
        reset={provider.error ? provider.error : weeklyReset}
        tone={weeklyTone}
      />
    </article>
  );
}

export default function FloatBar({ state }: { state: BootstrapState }) {
  const { providers } = useProviders({ refreshOnMount: false });
  const startDrag = useCallback((event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    void getCurrentWindow().startDragging().catch(() => {});
  }, []);

  useEffect(() => {
    document.body.classList.add("floatbar-window");
    return () => {
      document.body.classList.remove("floatbar-window");
    };
  }, []);

  const [settings, setSettings] = useState<SettingsSnapshot>(state.settings);

  useEffect(() => {
    const intervalMs = Math.max(60_000, settings.refreshIntervalSecs * 1000);
    const tick = () => {
      void refreshProvidersIfStale().catch(() => {});
    };
    tick();
    const id = setInterval(tick, intervalMs);
    return () => clearInterval(id);
  }, [settings.refreshIntervalSecs]);

  useEffect(() => {
    const unlisten = listen(FLOAT_BAR_CONFIG_CHANGED_EVENT, () => {
      void getSettingsSnapshot().then(setSettings).catch(() => {});
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const visible = useMemo(() => {
    return providers.filter((provider) => provider.providerId === CODEX_PROVIDER_ID).slice(0, 1);
  }, [providers]);

  const lastResizeRef = useRef<{ w: number; h: number } | null>(null);
  const resizeRafRef = useRef<number | null>(null);
  const resizeToContent = useCallback(() => {
    const el = document.querySelector<HTMLElement>(".floatbar");
    if (!el) return;
    if (resizeRafRef.current !== null) {
      cancelAnimationFrame(resizeRafRef.current);
    }
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      const rect = el.getBoundingClientRect();
      const padding = 12;
      const w = Math.ceil(rect.width + padding);
      const h = Math.ceil(rect.height + padding);
      const last = lastResizeRef.current;
      if (last && Math.abs(last.w - w) <= 1 && Math.abs(last.h - h) <= 1) return;
      lastResizeRef.current = { w, h };
      void resizeFloatBar(w, h).catch(() => {});
    });
  }, []);

  useEffect(() => {
    resizeToContent();
  }, [resizeToContent, visible.length, settings.resetTimeRelative]);

  useEffect(() => {
    const el = document.querySelector<HTMLElement>(".floatbar");
    if (!el || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(resizeToContent);
    observer.observe(el);
    return () => observer.disconnect();
  }, [resizeToContent]);

  useEffect(
    () => () => {
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
      }
    },
    [],
  );

  const highRemaining = 100 - settings.highUsageThreshold;
  const critRemaining = 100 - settings.criticalUsageThreshold;
  const opacityFraction = Math.max(0.3, Math.min(1, settings.floatBarOpacity / 100));

  return (
    <div
      className="floatbar"
      data-tauri-drag-region
      onMouseDown={startDrag}
      style={{ opacity: opacityFraction, "--floatbar-scale": settings.floatBarScale / 100 } as CSSProperties}
    >
      {visible.length === 0 ? (
        <div className="floatbar__empty" data-tauri-drag-region>
          正在读取 Codex
        </div>
      ) : (
        visible.map((provider) => (
          <CodexCard
            key={provider.providerId}
            provider={provider}
            highRemaining={highRemaining}
            critRemaining={critRemaining}
            resetRelative={settings.resetTimeRelative}
          />
        ))
      )}
    </div>
  );
}
