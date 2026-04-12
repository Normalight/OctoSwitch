import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";

type UseDragToReorderArgs<T> = {
  /** Persist the final order after drag ends. Called with ordered IDs. */
  persistOrder: (orderedIds: string[]) => Promise<void>;
  /** Extract a stable ID from an item. */
  getId: (item: T) => string;
  /** The busy flag — drag is disabled while busy. */
  busy: boolean;
};

function idsShallowEqual(a: string[], b: string[]) {
  return a.length === b.length && a.every((id, i) => id === b[i]);
}

export function useDragToReorder<T extends { id: string }>(
  items: T[],
  { persistOrder, getId, busy }: UseDragToReorderArgs<T>
) {
  const [orderedIds, setOrderedIds] = useState<string[]>([]);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  /** Card currently under pointer while dragging (for drop-target highlight). */
  const [dragHoverId, setDragHoverId] = useState<string | null>(null);
  const orderedIdsRef = useRef<string[]>([]);
  const dragStartOrderRef = useRef<string[]>([]);
  const draggingIdRef = useRef<string | null>(null);
  const hoverTargetIdRef = useRef<string | null>(null);
  const getIdRef = useRef(getId);
  const persistOrderRef = useRef(persistOrder);
  getIdRef.current = getId;
  persistOrderRef.current = persistOrder;
  draggingIdRef.current = draggingId;

  /** 仅用 id 列表的指纹驱动同步，避免 items / getId 引用每帧变化导致死循环 */
  const itemsIdsFingerprint = JSON.stringify(items.map((item) => getIdRef.current(item)));

  // Initialize/reconcile orderedIds when items change
  useEffect(() => {
    const incoming = JSON.parse(itemsIdsFingerprint) as string[];
    setOrderedIds((prev) => {
      if (prev.length === 0 && incoming.length === 0) {
        return prev;
      }
      if (prev.length === 0) {
        return incoming;
      }
      const incomingSet = new Set(incoming);
      const kept = prev.filter((id) => incomingSet.has(id));
      for (const id of incoming) {
        if (!kept.includes(id)) kept.push(id);
      }
      return idsShallowEqual(kept, prev) ? prev : kept;
    });
  }, [itemsIdsFingerprint]);

  // Keep ref in sync
  useEffect(() => {
    orderedIdsRef.current = orderedIds;
  }, [orderedIds]);

  // Pointer-up listener: persist if order actually changed
  useEffect(() => {
    if (!draggingId) return;
    const onPointerUp = () => {
      const before = dragStartOrderRef.current;
      const after = orderedIdsRef.current;
      setDragHoverId(null);
      setDraggingId(null);
      if (before.length === after.length && before.every((id, idx) => id === after[idx])) {
        return;
      }
      void persistOrderRef.current([...after]);
    };
    window.addEventListener("pointerup", onPointerUp, { once: true });
    return () => window.removeEventListener("pointerup", onPointerUp);
  }, [draggingId]);

  const moveById = useCallback((fromId: string, toId: string) => {
    if (!fromId || !toId || fromId === toId) return;
    setOrderedIds((prev) => {
      const next = [...prev];
      const fromIdx = next.indexOf(fromId);
      const toIdx = next.indexOf(toId);
      if (fromIdx < 0 || toIdx < 0 || fromIdx === toIdx) return prev;
      const [moved] = next.splice(fromIdx, 1);
      next.splice(toIdx, 0, moved);
      return next;
    });
  }, []);

  /** While dragging, follow pointer with hit-testing (avoids wrong highlights after DOM reorder). */
  useEffect(() => {
    if (!draggingId) {
      hoverTargetIdRef.current = null;
      return;
    }

    let raf = 0;

    const onPointerMove = (ev: globalThis.PointerEvent) => {
      if (raf !== 0) return;
      raf = requestAnimationFrame(() => {
        raf = 0;
        const active = draggingIdRef.current;
        if (!active) return;

        const root = document.querySelector(".sortable-list--dragging");
        if (!root) return;

        const el = document.elementFromPoint(ev.clientX, ev.clientY);
        if (!el || !root.contains(el)) {
          if (hoverTargetIdRef.current !== null) {
            hoverTargetIdRef.current = null;
            setDragHoverId(null);
          }
          return;
        }

        const row = el.closest("[data-sortable-id]") as HTMLElement | null;
        if (!row || !root.contains(row)) {
          if (hoverTargetIdRef.current !== null) {
            hoverTargetIdRef.current = null;
            setDragHoverId(null);
          }
          return;
        }

        const tid = row.dataset.sortableId ?? null;

        if (!tid || tid === active) {
          if (hoverTargetIdRef.current !== null) {
            hoverTargetIdRef.current = null;
            setDragHoverId(null);
          }
          return;
        }

        if (tid === hoverTargetIdRef.current) return;
        hoverTargetIdRef.current = tid;
        setDragHoverId(tid);
        moveById(active, tid);
      });
    };

    window.addEventListener("pointermove", onPointerMove);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      if (raf !== 0) cancelAnimationFrame(raf);
      hoverTargetIdRef.current = null;
    };
  }, [draggingId, moveById]);

  const orderedItems = useMemo(() => {
    const gid = getIdRef.current;
    const byId = new Map(items.map((item) => [gid(item), item]));
    return orderedIds
      .map((id) => byId.get(id))
      .filter((item): item is T => Boolean(item));
  }, [items, orderedIds]);

  const startDrag = (id: string, e: ReactPointerEvent<HTMLDivElement>) => {
    if (busy) return;
    e.preventDefault();
    dragStartOrderRef.current = [...orderedIdsRef.current];
    setDragHoverId(id);
    setDraggingId(id);
  };

  return {
    orderedItems,
    orderedIds,
    draggingId,
    dragHoverId,
    startDrag,
    moveById,
    setOrderedIds,
  };
}
