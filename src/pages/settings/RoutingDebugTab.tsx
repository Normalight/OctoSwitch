import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../../i18n";
import { tauriApi } from "../../lib/api/tauri";
import { CONFIG_IMPORTED } from "../../lib/constants";
import type { RoutingGroupStatus, RoutingStatus } from "../../types";

export function RoutingDebugTab() {
  const { t } = useI18n();
  const [routing, setRouting] = useState<RoutingStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savingKey, setSavingKey] = useState<string | null>(null);
  const [message, setMessage] = useState<{ type: "ok" | "err"; text: string } | null>(null);
  const [selectedMembers, setSelectedMembers] = useState<Record<string, string>>({});

  const groups = routing?.groups ?? [];

  const sortedGroups = useMemo(
    () => [...groups].sort((a, b) => a.alias.localeCompare(b.alias)),
    [groups]
  );

  const loadRouting = async (mode: "load" | "refresh" = "load") => {
    if (mode === "load") setLoading(true);
    else setRefreshing(true);
    setError(null);
    try {
      const status = await tauriApi.getRoutingStatus();
      setRouting(status);
      setSelectedMembers((prev) => {
        const next = { ...prev };
        for (const group of status.groups) {
          next[group.alias] = prev[group.alias] ?? group.active_member ?? group.members[0]?.name ?? "";
        }
        return next;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadRouting("load");
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void listen(CONFIG_IMPORTED, () => {
      void loadRouting("refresh");
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const handleActivate = async (group: RoutingGroupStatus) => {
    const member = selectedMembers[group.alias]?.trim();
    if (!member) {
      setMessage({ type: "err", text: t("settings.routingDebugSelectMember") });
      return;
    }
    setSavingKey(group.alias);
    setMessage(null);
    try {
      const updated = await tauriApi.setGroupActiveMemberByAlias(group.alias, member);
      setRouting((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          groups: prev.groups.map((item) => (item.alias === updated.alias ? updated : item))
        };
      });
      setSelectedMembers((prev) => ({ ...prev, [group.alias]: updated.active_member ?? member }));
      setMessage({
        type: "ok",
        text: t("settings.routingDebugActivated", {
          group: updated.alias,
          member: updated.active_member ?? member
        })
      });
    } catch (e) {
      setMessage({ type: "err", text: String(e) });
    } finally {
      setSavingKey(null);
    }
  };

  return (
    <div className="settings-tab-stack">
      <section className="settings-section settings-section--card card card--compact">
        <div className="settings-section-head">
          <span className="settings-section-icon" aria-hidden>
            <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
              <path d="M4 7h16M4 12h16M4 17h10" />
              <circle cx="18" cy="17" r="3" />
            </svg>
          </span>
          <h3 className="settings-section__title">{t("settings.routingDebugTitle")}</h3>
        </div>
        <p className="form-hint muted" style={{ margin: "0 0 12px" }}>
          {t("settings.routingDebugDesc")}
        </p>
        <div className="settings-section-actions">
          <button
            type="button"
            className="btn btn--primary btn--sm"
            disabled={loading || refreshing}
            onClick={() => void loadRouting("refresh")}
          >
            {refreshing ? t("common.loading") : t("common.refresh")}
          </button>
        </div>
        {message ? (
          <p className={message.type === "ok" ? "form-hint muted" : "form-error"} style={{ marginTop: 12 }}>
            {message.text}
          </p>
        ) : null}
        {error ? <p className="form-error" style={{ marginTop: 12 }}>{error}</p> : null}
        {loading ? (
          <p className="form-hint muted" style={{ marginTop: 12 }}>{t("common.loading")}</p>
        ) : (
          <div className="routing-debug-meta">
            <div className="routing-debug-pill">
              <strong>{t("settings.routingDebugGroupMemberPath")}: </strong>
              <span>
                {routing?.allow_group_member_model_path
                  ? t("settings.routingDebugEnabled")
                  : t("settings.routingDebugDisabled")}
              </span>
            </div>
            <div className="routing-debug-pill">
              <strong>{t("settings.routingDebugGroupCount")}: </strong>
              <span>{sortedGroups.length}</span>
            </div>
          </div>
        )}
      </section>

      {!loading && sortedGroups.map((group) => (
        <section
          key={group.group_id}
          className="settings-section settings-section--card card card--compact"
        >
          <div className="settings-section-head">
            <span className="settings-section-icon" aria-hidden>
              <svg viewBox="0 0 24 24" width={18} height={18} fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="4" width="18" height="16" rx="2" />
                <path d="M7 8h10M7 12h6M7 16h4" />
              </svg>
            </span>
            <div className="routing-debug-heading">
              <h3 className="settings-section__title">{group.alias}</h3>
              <p className="form-hint muted">
                {t("settings.routingDebugActiveMember")}:{" "}
                <strong>{group.active_member ?? t("common.dash")}</strong>
              </p>
            </div>
          </div>

          <div className="routing-debug-toolbar">
            <label className="routing-debug-select">
              <span>{t("settings.routingDebugActivateLabel")}</span>
              <select
                value={selectedMembers[group.alias] ?? ""}
                onChange={(e) =>
                  setSelectedMembers((prev) => ({ ...prev, [group.alias]: e.target.value }))
                }
              >
                {group.members.map((member) => (
                  <option key={member.binding_id} value={member.name}>
                    {member.name}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="btn btn--primary btn--sm"
              disabled={savingKey === group.alias || group.members.length === 0}
              onClick={() => void handleActivate(group)}
            >
              {savingKey === group.alias ? t("common.loading") : t("settings.routingDebugSetActive")}
            </button>
          </div>

          <div className="routing-debug-list">
            {group.members.length === 0 ? (
              <p className="form-hint muted">{t("settings.routingDebugNoMembers")}</p>
            ) : (
              group.members.map((member) => (
                <article
                  key={member.binding_id}
                  className={`routing-debug-member ${member.active ? "is-active" : ""}`}
                >
                  <div className="routing-debug-member__row">
                    <strong>{member.name}</strong>
                    <div className="routing-debug-badges">
                      {member.active ? (
                        <span className="routing-debug-badge routing-debug-badge--active">
                          {t("settings.routingDebugBadgeActive")}
                        </span>
                      ) : null}
                      {!member.enabled ? (
                        <span className="routing-debug-badge routing-debug-badge--disabled">
                          {t("settings.routingDebugBadgeDisabled")}
                        </span>
                      ) : null}
                    </div>
                  </div>
                  <p className="form-hint muted">
                    {t("settings.routingDebugProvider")}: {member.provider_id}
                  </p>
                  <p className="form-hint muted">
                    {t("settings.routingDebugUpstream")}: {member.upstream_model_name}
                  </p>
                  <p className="form-hint muted">
                    {t("settings.routingDebugModelPath")}: {group.alias}/{member.name}
                  </p>
                </article>
              ))
            )}
          </div>
        </section>
      ))}
    </div>
  );
}
