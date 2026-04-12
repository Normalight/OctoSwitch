import { useCallback, useState } from "react";
import { ConfirmDialog } from "../components/Dialogs";
import { GroupCard } from "../components/GroupCard";
import { GroupEditModal } from "../components/GroupEditModal";
import { useModelGroups } from "../hooks/useModelGroups";
import { useModels } from "../hooks/useModels";
import { useProviders } from "../hooks/useProviders";
import { useI18n } from "../i18n";
import { tauriApi } from "../lib/api/tauri";
import { mapCommonDbError } from "../lib/mapModelBindingError";
import { segmentHasSlash } from "../lib/modelSlugValidation";
import { useDragToReorder } from "../hooks/useDragToReorder";
import type { ModelBinding, ModelGroup } from "../types";

export function ModelsPage() {
  const { t } = useI18n();
  const { models, loading, refresh: refreshModels } = useModels();
  const { groups, refresh: refreshGroups } = useModelGroups();
  const { providers } = useProviders();

  const [busy, setBusy] = useState(false);

  const {
    orderedItems: orderedGroups,
    draggingId: pointerDraggingGroupId,
    dragHoverId: groupDragHoverId,
    startDrag: startGroupPointerDrag,
  } = useDragToReorder(groups, {
    persistOrder: async (orderedIds) => {
      setBusy(true);
      try {
        for (const [idx, id] of orderedIds.entries()) {
          await tauriApi.updateModelGroup(id, { sort_order: idx });
        }
        await refreshGroups();
      } catch (e) {
        setGroupErr(mapCommonDbError(e, t));
      } finally {
        setBusy(false);
      }
    },
    getId: (g) => g.id,
    busy,
  });

  /* ---- inline errors ---- */
  const [groupErr, setGroupErr] = useState("");
  const [memberErr, setMemberErr] = useState("");

  // Client-side dialogs
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmMsg, setConfirmMsg] = useState({ title: "", message: "" });
  const [confirmAction, setConfirmAction] = useState<(() => void) | null>(null);

  const [groupModal, setGroupModal] = useState<
    { open: false } | { open: true; mode: "create" } | { open: true; mode: "edit"; group: ModelGroup }
  >({ open: false });
  const [groupAliasDraft, setGroupAliasDraft] = useState("");

  const [memPickProvider, setMemPickProvider] = useState("");
  const [memPickBinding, setMemPickBinding] = useState("");

  const providerLabel = (id: string) => providers.find((p) => p.id === id)?.name ?? id;

  const membersOf = useCallback(
    (gid: string) => models.filter((m) => m.group_ids.includes(gid)),
    [models]
  );

  /* ---- group CRUD ---- */
  const openCreateGroup = () => {
    setGroupAliasDraft("");
    setGroupErr("");
    setGroupModal({ open: true, mode: "create" });
  };

  const openEditGroup = (g: ModelGroup) => {
    setGroupAliasDraft(g.alias);
    setGroupErr("");
    setGroupModal({ open: true, mode: "edit", group: g });
  };

  const saveGroup = async () => {
    const alias = groupAliasDraft.trim();
    if (!alias) {
      setGroupErr(t("models.errAlias"));
      return;
    }
    if (segmentHasSlash(alias)) {
      setGroupErr(t("groups.errAliasNoSlash"));
      return;
    }
    setBusy(true);
    setGroupErr("");
    try {
      if (groupModal.open && groupModal.mode === "create") {
        await tauriApi.createModelGroup({ alias });
      } else if (groupModal.open && groupModal.mode === "edit") {
        await tauriApi.updateModelGroup(groupModal.group.id, { alias });
      }
      setGroupModal({ open: false });
      await refreshGroups();
    } catch (e) {
      setGroupErr(mapCommonDbError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const removeGroup = async (g: ModelGroup) => {
    setConfirmMsg({
      title: t("models.deleteGroupTitle"),
      message: t("models.deleteGroupBody", { alias: g.alias }),
    });
    setConfirmAction(() => async () => {
      setBusy(true);
      try {
        await tauriApi.deleteModelGroup(g.id);
        setGroupModal({ open: false });
        await refreshGroups();
        await refreshModels();
      } catch (e) {
        setGroupErr(mapCommonDbError(e, t));
      } finally {
        setBusy(false);
      }
    });
    setConfirmOpen(true);
  };

  const toggleGroup = async (g: ModelGroup) => {
    setBusy(true);
    try {
      await tauriApi.toggleModelGroupEnabled(g.id, !g.is_enabled);
      await refreshGroups();
    } catch (e) {
      setGroupErr(mapCommonDbError(e, t));
    } finally {
      setBusy(false);
    }
  };

  /* ---- member CRUD ---- */

  const bindingsForProvider = useCallback(
    (providerId: string, excludeGroupId?: string) =>
      models.filter((m) => {
        if (m.provider_id !== providerId) return false;
        if (!excludeGroupId) return true;
        return !m.group_ids.includes(excludeGroupId);
      }),
    [models]
  );

  const saveAddMember = async () => {
    if (!groupModal.open || groupModal.mode !== "edit") return;
    const gid = groupModal.group.id;
    if (!memPickBinding) {
      setMemberErr(t("groups.pickBinding"));
      return;
    }
    setBusy(true);
    setMemberErr("");
    try {
      await tauriApi.addModelGroupMember(gid, memPickBinding);
      setMemPickProvider("");
      setMemPickBinding("");
      await refreshModels();
      await refreshGroups();
    } catch (e) {
      setMemberErr(mapCommonDbError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const removeMember = async (groupId: string, b: ModelBinding) => {
    setBusy(true);
    try {
      await tauriApi.removeModelGroupMember(groupId, b.id);
      await refreshModels();
      await refreshGroups();
    } catch (e) {
      setMemberErr(mapCommonDbError(e, t));
    } finally {
      setBusy(false);
    }
  };

  const setActiveInGroup = async (groupId: string, bindingId: string) => {
    setBusy(true);
    try {
      await tauriApi.setModelGroupActiveBinding(groupId, bindingId);
      await refreshGroups();
      // Update the stale snapshot so the modal reflects the new active binding immediately
      setGroupModal((prev) => {
        if (!prev.open || prev.mode !== "edit") return prev;
        return { ...prev, group: { ...prev.group, active_binding_id: bindingId } };
      });
    } catch (e) {
      setMemberErr(mapCommonDbError(e, t));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="page-resource models-page page-groups">
      <div className="providers-page-head">
        <div className="providers-page-head__intro">
          <h2 className="page-title providers-page__title">{t("app.groups")}</h2>
          <p className="page-lead muted providers-page-head__lead">{t("groups.lead")}</p>
        </div>
        <button
          type="button"
          className="btn btn--primary btn--sm providers-page-head__add"
          disabled={busy}
          onClick={openCreateGroup}
        >
          {t("models.addGroup")}
        </button>
      </div>

      {loading ? <p className="muted">{t("common.loading")}</p> : null}

      <div
        className={`models-sortable-list sortable-list${pointerDraggingGroupId ? " sortable-list--dragging" : ""}`}
      >
      {orderedGroups.map((g) => {
        const members = membersOf(g.id);
        const active = members.find((m) => g.active_binding_id === m.id);
        const activeProvider = active ? providerLabel(active.provider_id) : "";
        const activeModel = active ? active.model_name : "";

        return (
          <GroupCard
            key={g.id}
            group={g}
            members={members}
            activeProvider={activeProvider}
            activeModel={activeModel}
            busy={busy}
            onDragStart={startGroupPointerDrag}
            reorderUi={{
              activeId: pointerDraggingGroupId,
              hoverId: groupDragHoverId,
            }}
            onEdit={openEditGroup}
            onToggle={toggleGroup}
            t={t}
          />
        );
      })}
      </div>

      {/* ---- group modal ---- */}
      <GroupEditModal
        open={groupModal.open}
        mode={groupModal.open && groupModal.mode === "edit" ? "edit" : "create"}
        group={groupModal.open && groupModal.mode === "edit" ? groupModal.group : undefined}
        aliasDraft={groupAliasDraft}
        onAliasChange={setGroupAliasDraft}
        busy={busy}
        error={groupErr}
        onSave={() => void saveGroup()}
        onDelete={() => void removeGroup(groupModal.open && groupModal.mode === "edit" ? groupModal.group : undefined as any)}
        onClose={() => { setGroupModal({ open: false }); setGroupErr(""); }}
        members={groupModal.open && groupModal.mode === "edit" ? membersOf(groupModal.group.id) : []}
        providerLabel={providerLabel}
        providers={providers}
        bindingsForProvider={bindingsForProvider}
        memPick={{ provider: memPickProvider, binding: memPickBinding }}
        onMemPickChange={(provider, binding) => { setMemPickProvider(provider); setMemPickBinding(binding); setMemberErr(""); }}
        memberError={memberErr}
        onAddMember={() => void saveAddMember()}
        onRemoveMember={(m) => void removeMember(groupModal.open && groupModal.mode === "edit" ? groupModal.group.id : "", m)}
        onSetActive={(m) => void setActiveInGroup(groupModal.open && groupModal.mode === "edit" ? groupModal.group.id : "", m.id)}
        t={t}
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
    </section>
  );
}
