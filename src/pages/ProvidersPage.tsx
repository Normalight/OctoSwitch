import { useCallback, useEffect, useMemo, useState } from "react";
import { CopilotAuthModal } from "../components/CopilotAuthModal";
import { ConfirmDialog, ErrorDialog } from "../components/Dialogs";
import { ModelStack } from "../components/ModelStack";
import { ProviderCard } from "../components/ProviderCard";
import { ProviderBindingModal } from "../components/ProviderBindingModal";
import { ProviderEditModal } from "../components/ProviderEditModal";
import { useModels } from "../hooks/useModels";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { mapCommonDbError } from "../lib/mapModelBindingError";
import { segmentHasSlash } from "../lib/modelSlugValidation";
import { useProviders } from "../hooks/useProviders";
import { useDragToReorder } from "../hooks/useDragToReorder";
import type { CopilotAccountStatus, CopilotStatus, DeviceCodeResponse, ModelBinding, ModelGroup, Provider, ProviderSummary } from "../types";

type ProviderModal =
  | { open: false }
  | { open: true; mode: "create" }
  | { open: true; mode: "edit"; provider: Provider };

type ProviderChildBinding = { open: false } | { open: true; mode: "create" } | { open: true; mode: "edit"; binding: ModelBinding };

export function ProvidersPage() {
  const { t } = useI18n();
  const { providers, loading, refresh, fetchProvider } = useProviders();

  const [modelsFetchEnabled, setModelsFetchEnabled] = useState(false);
  useEffect(() => {
    let cancelled = false;
    const id = requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        if (!cancelled) setModelsFetchEnabled(true);
      });
    });
    return () => {
      cancelled = true;
      cancelAnimationFrame(id);
    };
  }, []);

  const { models, loading: modelsLoading, refresh: refreshModels } = useModels(modelsFetchEnabled);
  const modelsListPending = !modelsFetchEnabled || modelsLoading;
  const [groups, setGroups] = useState<ModelGroup[]>([]);
  const [groupsLoaded, setGroupsLoaded] = useState(false);

  const loadGroups = useCallback(async () => {
    try {
      const g = await tauriApi.listModelGroups();
      setGroups(g);
      setGroupsLoaded(true);
    } catch {
      // ignore
    }
  }, []);
  const [modal, setModal] = useState<ProviderModal>({ open: false });
  const [copilotOpen, setCopilotOpen] = useState(false);
  const [copilotStatus, setCopilotStatus] = useState<CopilotStatus | null>(null);
  const [copilotAccounts, setCopilotAccounts] = useState<CopilotAccountStatus[]>([]);

  // Persist pending copilot auth state across modal open/close
  const [pendingDeviceCode, setPendingDeviceCode] = useState<DeviceCodeResponse | null>(() => {
    try {
      const raw = localStorage.getItem("copilot_pending_dc");
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null;
    }
  });

  // Client-side dialogs
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmMsg, setConfirmMsg] = useState({ title: "", message: "" });
  const [confirmAction, setConfirmAction] = useState<(() => void) | null>(null);
  const [errorOpen, setErrorOpen] = useState(false);
  const [errorMsg, setErrorMsg] = useState({ title: "", message: "" });

  const groupAlias = useCallback((id: string) => groups.find((g) => g.id === id)?.alias ?? id, [groups]);

  const apiFormatTag = useMemo(
    () =>
      ({
        anthropic: t("providers.tagAnthropic"),
        openai_chat: t("providers.tagOpenai"),
        openai_responses: t("providers.tagOpenai"),
      }) satisfies Record<string, string>,
    [t]
  );

  // Map provider_id → copilot account for status-colored tags
  const copilotAccountMap = useMemo(() => {
    const m = new Map<string, CopilotAccountStatus>();
    for (const acc of copilotAccounts) {
      m.set(acc.provider_id, acc);
    }
    return m;
  }, [copilotAccounts]);

  const modelsByProvider = useMemo(() => {
    const m = new Map<string, ModelBinding[]>();
    for (const p of providers) {
      m.set(p.id, []);
    }
    if (modelsListPending) return m;
    for (const b of models) {
      const arr = m.get(b.provider_id);
      if (arr) arr.push(b);
    }
    for (const arr of m.values()) {
      arr.sort((a, b) => a.model_name.localeCompare(b.model_name));
    }
    return m;
  }, [providers, models, modelsListPending]);

  const modelsForOpenProvider = useMemo(() => {
    if (!(modal.open && modal.mode === "edit")) return [];
    return modelsByProvider.get(modal.provider.id) ?? [];
  }, [modal, modelsByProvider]);

  const loadCopilotStatus = useCallback(async () => {
    try {
      const s = await tauriApi.getCopilotStatus();
      setCopilotStatus(s);
    } catch {
      setCopilotStatus(null);
    }
  }, []);

  const loadCopilotAccounts = useCallback(async () => {
    try {
      const accounts = await tauriApi.listCopilotAccounts();
      setCopilotAccounts(accounts);
    } catch {
      setCopilotAccounts([]);
    }
  }, []);

  useEffect(() => {
    if (providers.some((p) => p.id === "copilot")) {
      void loadCopilotStatus();
    }
    void loadCopilotAccounts();
  }, [providers, loadCopilotStatus, loadCopilotAccounts]);

  useEffect(() => {
    if (modal.open && modal.mode === "edit" && modal.provider.id === "copilot") {
      void loadCopilotStatus();

      const timer = window.setInterval(() => {
        void loadCopilotStatus();
      }, 30_000);

      return () => window.clearInterval(timer);
    }
  }, [modal, loadCopilotStatus]);

  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKeyRef, setApiKeyRef] = useState("");
  const [apiFormat, setApiFormat] = useState<string>("anthropic");
  const [authMode, setAuthMode] = useState<string>("bearer");
  const [enabled, setEnabled] = useState(true);
  const [showApiKey, setShowApiKey] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [healthMsg, setHealthMsg] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [bindingBusy, setBindingBusy] = useState(false);
  const [childBinding, setChildBinding] = useState<ProviderChildBinding>({ open: false });
  const [bindingModelName, setBindingModelName] = useState("");
  const [bindingUpstream, setBindingUpstream] = useState("");
  const [bindingRoutingErr, setBindingRoutingErr] = useState<string | null>(null);
  const [bindingUpstreamErr, setBindingUpstreamErr] = useState<string | null>(null);
  const [bindingErr, setBindingErr] = useState<string | null>(null);

  const clearChildBindingModal = () => {
    setChildBinding({ open: false });
    setBindingModelName("");
    setBindingUpstream("");
    setBindingRoutingErr(null);
    setBindingUpstreamErr(null);
    setBindingErr(null);
  };

  const validateInlineBinding = (): boolean => {
    let ok = true;
    let re: string | null = null;
    let ue: string | null = null;
    const routing = bindingModelName.trim();
    const upstream = bindingUpstream.trim();
    if (!routing && !upstream) {
      re = t("models.errRoutingOrUpstream");
      ok = false;
    }
    if (bindingUpstream.length > 0 && !upstream) {
      ue = t("models.errUpstreamWhitespace");
      ok = false;
    }
    const modelName = routing || upstream;
    if (modelName && segmentHasSlash(modelName)) {
      re = t("models.errModelNameNoSlash");
      ok = false;
    }
    setBindingRoutingErr(re);
    setBindingUpstreamErr(ue);
    return ok;
  };

  const {
    orderedItems: orderedProviders,
    draggingId: pointerDraggingProviderId,
    dragHoverId: providerDragHoverId,
    startDrag: startProviderPointerDrag,
  } = useDragToReorder(providers, {
    persistOrder: async (orderedIds) => {
      setBusy(true);
      try {
        for (const [idx, id] of orderedIds.entries()) {
          await tauriApi.updateProvider(id, { sort_order: idx });
        }
        await refresh();
      } catch (e) {
        setErrorMsg({ title: t("common.saveFailed"), message: String(e) });
        setErrorOpen(true);
      } finally {
        setBusy(false);
      }
    },
    getId: (p) => p.id,
    busy,
  });

  const resetCreateForm = () => {
    setName("");
    setBaseUrl("");
    setApiKeyRef("");
    setApiFormat("anthropic");
    setAuthMode("bearer");
    setEnabled(true);
    setShowApiKey(false);
    setAdvancedOpen(false);
    setHealthMsg(null);
  };

  const openCreate = () => {
    clearChildBindingModal();
    resetCreateForm();
    setModal({ open: true, mode: "create" });
  };

  const openEdit = async (summary: ProviderSummary) => {
    clearChildBindingModal();
    const full = await fetchProvider(summary.id);
    setName(full.name);
    setBaseUrl(full.base_url);
    setApiKeyRef(full.api_key_ref);
    setApiFormat(full.id === "copilot" ? "openai_chat" : (full.api_format ?? "anthropic"));
    setAuthMode(full.auth_mode ?? "bearer");
    setEnabled(full.is_enabled);
    setShowApiKey(false);
    setAdvancedOpen(false);
    setHealthMsg(null);
    setModal({ open: true, mode: "edit", provider: full });
    if (!groupsLoaded) {
      void loadGroups();
    }
  };

  const save = async () => {
    setBusy(true);
    try {
      if (modal.open && modal.mode === "create") {
        const created = await tauriApi.createProvider({
          name,
          base_url: baseUrl,
          api_key_ref: apiKeyRef,
          timeout_ms: 60000, // TODO(future): configurable per-provider timeout
          max_retries: 10,
          is_enabled: enabled,
          api_format: apiFormat === "anthropic" ? null : (apiFormat as any),
          auth_mode: authMode as any,
        });
        await refresh();
        await refreshModels();
        if (!groupsLoaded) void loadGroups();
        // Fetch full provider for the edit modal (api_key_ref is masked in the summary)
        const full = await fetchProvider(created.id);
        setModal({ open: true, mode: "edit", provider: full });
      } else if (modal.open && modal.mode === "edit") {
        await tauriApi.updateProvider(modal.provider.id, {
          name,
          base_url: baseUrl,
          api_key_ref: apiKeyRef,
          sort_order: modal.provider.sort_order ?? 0,
          is_enabled: enabled,
          api_format: apiFormat === "anthropic" ? null : (apiFormat as any),
          auth_mode: authMode as any,
        });
        setModal({ open: false });
        await refresh();
      }
    } catch (e) {
      setErrorMsg({ title: t("common.saveFailed"), message: mapCommonDbError(e, t) });
      setErrorOpen(true);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!(modal.open && modal.mode === "edit")) return;
    const p = modal.provider;
    setConfirmMsg({
      title: t("providers.deleteConfirmTitle"),
      message: t("providers.deleteConfirmBody", { name: p.name }),
    });
    setConfirmAction(() => async () => {
      setBusy(true);
      try {
        await tauriApi.deleteProvider(p.id);
        setModal({ open: false });
        await refresh();
      } catch (e) {
        setErrorMsg({ title: t("common.deleteFailed"), message: String(e) });
        setErrorOpen(true);
      } finally {
        setBusy(false);
      }
    });
    setConfirmOpen(true);
  };

  const health = async () => {
    if (!(modal.open && modal.mode === "edit")) return;
    setHealthMsg(null);
    setBusy(true);
    try {
      const r = await tauriApi.runProviderHealthCheck(modal.provider.id);
      setHealthMsg(`${r.ok ? t("providers.healthOk") : t("providers.healthBad")} · ${r.latency_ms}ms · ${r.message}`);
    } catch (e) {
      setHealthMsg(String(e));
    } finally {
      setBusy(false);
    }
  };

  const openChildBindingEdit = (binding: ModelBinding) => {
    setChildBinding({ open: true, mode: "edit", binding });
    setBindingModelName(binding.model_name);
    setBindingUpstream(binding.upstream_model_name);
    setBindingRoutingErr(null);
    setBindingUpstreamErr(null);
    setBindingErr(null);
  };

  const openChildBindingCreate = () => {
    setChildBinding({ open: true, mode: "create" });
    setBindingModelName("");
    setBindingUpstream("");
    setBindingRoutingErr(null);
    setBindingUpstreamErr(null);
    setBindingErr(null);
  };

  const submitChildBinding = async () => {
    if (!(modal.open && modal.mode === "edit") || !childBinding.open) return;
    if (!validateInlineBinding()) return;
    const routing = bindingModelName.trim();
    const upstream = bindingUpstream.trim();
    const modelName = routing || upstream;
    const upstreamName = upstream || routing;

    setBindingBusy(true);
    setBindingErr(null);
    try {
      if (childBinding.mode === "edit") {
        await tauriApi.updateModelBinding(childBinding.binding.id, {
          model_name: modelName,
          upstream_model_name: upstreamName,
          input_price_per_1m: 0,
          output_price_per_1m: 0,
          rpm_limit: null,
          tpm_limit: null,
        });
      } else {
        await tauriApi.createModelBinding({
          model_name: modelName,
          provider_id: modal.provider.id,
          upstream_model_name: upstreamName,
          input_price_per_1m: 0,
          output_price_per_1m: 0,
          rpm_limit: null,
          tpm_limit: null,
          is_enabled: true,
        });
      }
      clearChildBindingModal();
      await refreshModels();
    } catch (e) {
      setBindingErr(mapCommonDbError(e, t));
    } finally {
      setBindingBusy(false);
    }
  };

  const removeBindingFromProvider = async (binding: ModelBinding) => {
    setConfirmMsg({
      title: t("models.deleteBindingTitle"),
      message: t("models.deleteBindingBody", { name: binding.model_name }),
    });
    setConfirmAction(() => async () => {
      setBusy(true);
      try {
        await tauriApi.deleteModelBinding(binding.id);
        clearChildBindingModal();
        await refreshModels();
      } catch (e) {
        setErrorMsg({ title: t("common.deleteFailed"), message: mapCommonDbError(e, t) });
        setErrorOpen(true);
      } finally {
        setBusy(false);
      }
    });
    setConfirmOpen(true);
  };

  const handleOpenCopilot = async () => {
    try {
      await tauriApi.getCopilotStatus();
    } catch {
      // ignore
    } finally {
      setCopilotOpen(true);
    }
  };

  return (
    <section className="page-resource page-providers-compact">
      <div className="providers-page-head">
        <div className="providers-page-head__intro">
          <h2 className="page-title providers-page__title">{t("providers.title")}</h2>
          <p className="page-lead muted providers-page-head__lead">{t("providers.lead")}</p>
        </div>
        <button
          type="button"
          className="btn btn--accent-soft btn--sm providers-page-head__add"
          disabled={busy}
          onClick={() => void handleOpenCopilot()}
        >
          {t("copilot.button")}
        </button>
        <button
          type="button"
          className="btn btn--primary btn--sm providers-page-head__add"
          disabled={busy}
          onClick={openCreate}
        >
          {t("providers.add")}
        </button>
      </div>

      {loading && providers.length === 0 ? (
        <p className="muted">{t("common.loading")}</p>
      ) : (
        <div
          className={`provider-list sortable-list${pointerDraggingProviderId ? " sortable-list--dragging" : ""}`}
        >
          {orderedProviders.map((p) => {
          const sub = modelsByProvider.get(p.id) ?? [];
          return (
            <ProviderCard
              key={p.id}
              provider={p}
              models={sub}
              busy={busy}
              onEdit={openEdit}
              onDragStart={startProviderPointerDrag}
              reorderUi={{
                activeId: pointerDraggingProviderId,
                hoverId: providerDragHoverId,
              }}
              groupAlias={groupAlias}
              copilotAccountMap={copilotAccountMap}
              t={t}
              apiFormatTagLabel={apiFormatTag[p.api_format ?? "anthropic"]}
              renderModels={() =>
                sub.length === 0 ? (
                  <p className="provider-card__empty muted">{t("providers.noModels")}</p>
                ) : (
                  <ModelStack
                    bindings={sub}
                    keyPrefix={p.id}
                    maxVisible={10}
                    variant="summary"
                    groupAlias={groupAlias}
                    showUpstreamOnChip={false}
                  />
                )
              }
            />
          );
        })}
      </div>
      )}

      <ProviderEditModal
        open={modal.open}
        mode={modal.open && modal.mode === "edit" ? "edit" : "create"}
        provider={modal.open && modal.mode === "edit" ? modal.provider : undefined}
        form={{
          name, baseUrl, apiKeyRef, apiFormat, authMode,
          enabled, showApiKey, advancedOpen,
        }}
        setForm={{
          name: setName, baseUrl: setBaseUrl, apiKeyRef: setApiKeyRef,
          apiFormat: setApiFormat, authMode: setAuthMode,
          enabled: setEnabled, showApiKey: setShowApiKey, advancedOpen: setAdvancedOpen,
        }}
        busy={busy}
        healthMsg={healthMsg}
        onSave={() => void save()}
        onDelete={() => void remove()}
        onHealth={() => void health()}
        onClose={() => {
          clearChildBindingModal();
          setModal({ open: false });
        }}
        copilotStatus={copilotStatus}
        modelsForProvider={modelsForOpenProvider}
        onAddBinding={openChildBindingCreate}
        onEditBinding={openChildBindingEdit}
        onRemoveBinding={removeBindingFromProvider}
        groupAlias={groupAlias}
        t={t}
      />

      {modal.open && modal.mode === "edit" && childBinding.open && modal.provider.id !== "copilot" ? (
        <ProviderBindingModal
          open
          mode={childBinding.mode === "edit" ? "edit" : "create"}
          providerId={modal.provider.id}
          binding={childBinding.mode === "edit" ? childBinding.binding : null}
          routingName={bindingModelName}
          upstream={bindingUpstream}
          setRoutingName={setBindingModelName}
          setUpstream={setBindingUpstream}
          routingError={bindingRoutingErr}
          upstreamError={bindingUpstreamErr}
          submitError={bindingErr}
          busy={bindingBusy}
          onSubmit={() => void submitChildBinding()}
          onClose={clearChildBindingModal}
          onRequestDelete={
            childBinding.mode === "edit"
              ? () => {
                  removeBindingFromProvider(childBinding.binding);
                }
              : undefined
          }
          groupAlias={groupAlias}
          t={t}
        />
      ) : null}

      <CopilotAuthModal
        open={copilotOpen}
        onClose={() => setCopilotOpen(false)}
        onStatusChange={() => {
          void refresh();
          void loadCopilotStatus();
          void loadCopilotAccounts();
        }}
        existingCopilotAccounts={copilotAccounts}
        pendingDeviceCode={pendingDeviceCode}
        onPendingDeviceCodeChange={setPendingDeviceCode}
      />

      <ConfirmDialog
        title={confirmMsg.title}
        message={confirmMsg.message}
        open={confirmOpen}
        onClose={() => { setConfirmOpen(false); setConfirmAction(null); }}
        onConfirm={() => { if (confirmAction) void confirmAction(); }}
        confirmText={t("common.delete")}
        confirmVariant="danger"
      />

      <ErrorDialog
        title={errorMsg.title}
        message={errorMsg.message}
        open={errorOpen}
        onClose={() => setErrorOpen(false)}
      />
    </section>
  );
}
