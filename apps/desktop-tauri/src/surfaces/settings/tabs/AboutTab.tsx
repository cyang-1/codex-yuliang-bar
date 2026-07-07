import { useEffect, useState } from "react";
import { useLocale } from "../../../hooks/useLocale";
import { useUpdateState } from "../../../hooks/useUpdateState";
import { getAppInfo, openExternalUrl } from "../../../lib/tauri";
import { Field, Select, Toggle } from "../../../components/FormControls";
import type { AppInfoBridge, UpdateChannel } from "../../../types/bridge";
import type { TabProps } from "../../Settings";
import codexbarIcon from "../../../assets/codexbar-icon.png";

const TEXT = {
  loading: "\u6b63\u5728\u52a0\u8f7d...",
  version: "\u7248\u672c",
  upstream: "\u4e0a\u6e38\u9879\u76ee",
  original: "\u539f\u59cb\u9879\u76ee",
  error: "\u9519\u8bef\uff1a",
  checking: "\u6b63\u5728\u68c0\u67e5...",
  checkUpdates: "\u68c0\u67e5\u66f4\u65b0",
  updateAvailable: "\u53d1\u73b0\u65b0\u7248\u672c",
  download: "\u4e0b\u8f7d",
  releasePage: "\u67e5\u770b\u53d1\u5e03\u9875",
  downloading: "\u6b63\u5728\u4e0b\u8f7d...",
  ready: "\u66f4\u65b0\u5df2\u51c6\u5907\u5b89\u88c5",
  installAndRestart: "\u5b89\u88c5\u5e76\u91cd\u542f",
  latest: "\u5df2\u7ecf\u662f\u6700\u65b0\u7248\u672c",
  copyrightPrefix: "\u8fd9\u662f\u4e13\u4e3a Codex \u4f59\u91cf\u76d1\u63a7\u6539\u9020\u7684 Windows \u5c0f\u7ec4\u4ef6\uff0c\u57fa\u4e8e",
  copyrightSuffix: "\uff0c\u9075\u5faa MIT License\u3002",
} as const;

const ABOUT_LINKS = [
  {
    label: TEXT.upstream,
    url: "https://github.com/Finesssee/Win-CodexBar",
  },
  {
    label: TEXT.original,
    url: "https://github.com/steipete/CodexBar",
  },
] as const;

export default function AboutTab({ settings, set, saving }: TabProps) {
  const { t } = useLocale();
  const [appInfo, setAppInfo] = useState<AppInfoBridge | null>(null);
  const { updateState, checkNow, download, apply, openRelease } =
    useUpdateState();
  const [hasChecked, setHasChecked] = useState(false);
  const [linkError, setLinkError] = useState<string | null>(null);

  useEffect(() => {
    void getAppInfo().then(setAppInfo);
  }, []);

  const handleCheck = () => {
    setHasChecked(true);
    checkNow();
  };

  const openAboutLink = (url: string) => {
    setLinkError(null);
    openExternalUrl(url).catch((error) => {
      setLinkError(String(error));
    });
  };

  if (!appInfo) {
    return (
      <section className="settings-section">
        <p className="settings-section__hint">{TEXT.loading}</p>
      </section>
    );
  }

  const isBusy =
    updateState.status === "checking" ||
    updateState.status === "downloading";

  return (
    <section className="settings-section about-section">
      <div className="about-header">
        <img className="about-icon" src={codexbarIcon} alt="Codex 余量条" />
        <div className="about-title-block">
          <h2 className="about-title">{appInfo.name}</h2>
          <p className="about-version">
            {TEXT.version} {appInfo.version}
            {appInfo.buildNumber !== "dev" && ` (${appInfo.buildNumber})`}
          </p>
          <p className="about-tagline">{appInfo.tagline}</p>
        </div>
      </div>

      <div className="about-links">
        {ABOUT_LINKS.map((link) => (
          <button
            key={link.url}
            type="button"
            className="about-link"
            onClick={() => openAboutLink(link.url)}
          >
            {link.label}
          </button>
        ))}
      </div>
      {linkError && <p className="about-update-msg">{TEXT.error}{linkError}</p>}

      <div className="about-divider" />

      <div className="about-update-controls">
        <Field
          label={t("AutoDownloadUpdates")}
          description={t("AutoDownloadUpdatesHelper")}
          leading
        >
          <Toggle
            checked={settings.autoDownloadUpdates}
            disabled={saving}
            onChange={(v) => set({ autoDownloadUpdates: v })}
          />
        </Field>

        <div className="about-channel-row">
          <Field label={t("UpdateChannelChoice")}>
            <Select
              value={settings.updateChannel}
              disabled={saving}
              options={[
                { value: "stable", label: t("UpdateChannelStableOption") },
                { value: "beta", label: t("UpdateChannelBetaOption") },
              ]}
              onChange={(v) => set({ updateChannel: v as UpdateChannel })}
            />
          </Field>
          <p className="about-channel-description">
            {t("UpdateChannelChoiceHelper")}
          </p>
        </div>
      </div>

      <div className="about-actions">
        <button
          className="credential-btn credential-btn--primary"
          disabled={isBusy}
          onClick={handleCheck}
        >
          {updateState.status === "checking" ? TEXT.checking : TEXT.checkUpdates}
        </button>

        {updateState.status === "available" && (
          <div className="about-update-row">
            <span className="about-update-msg">
              {TEXT.updateAvailable} {updateState.version}
            </span>
            {updateState.canDownload ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={download}
              >
                {TEXT.download}
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                {TEXT.releasePage}
              </button>
            )}
          </div>
        )}

        {updateState.status === "downloading" && (
          <span className="about-update-msg">
            {TEXT.downloading}
            {updateState.progress != null &&
              ` ${Math.round(updateState.progress * 100)}%`}
          </span>
        )}

        {updateState.status === "ready" && (
          <div className="about-update-row">
            <span className="about-update-msg">{TEXT.ready}</span>
            {updateState.canApply ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={apply}
              >
                {TEXT.installAndRestart}
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                {TEXT.releasePage}
              </button>
            )}
          </div>
        )}

        {updateState.status === "error" && (
          <span className="about-update-msg">
            {TEXT.error}{updateState.error}
          </span>
        )}

        {updateState.status === "idle" && hasChecked && (
          <span className="about-update-msg">{TEXT.latest}</span>
        )}
      </div>

      <p className="about-copyright">
        {TEXT.copyrightPrefix}{" "}
        <button
          type="button"
          className="about-link about-link--inline"
          onClick={() => openAboutLink("https://github.com/Finesssee/Win-CodexBar")}
        >
          Win-CodexBar / CodexBar
        </button>
        {TEXT.copyrightSuffix}
      </p>
    </section>
  );
}
