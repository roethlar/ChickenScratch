/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useCallback, useRef, useState, type ReactNode } from "react";

interface DragState {
  draggingId: string | null;
  dropTargetId: string | null;
  dropPosition: "before" | "after" | "into" | null;
}

type DropHandler = (dragId: string, targetId: string, position: "before" | "after" | "into") => void;

interface DragContextType {
  draggingId: string | null;
  dropTargetId: string | null;
  dropPosition: "before" | "after" | "into" | null;
  startDrag: (nodeId: string) => void;
  setDropTarget: (nodeId: string, position: "before" | "after" | "into") => void;
  clearDropTarget: () => void;
  endDrag: () => void;
  setOnDrop: (handler: DropHandler) => void;
}

const Ctx = createContext<DragContextType>({
  draggingId: null,
  dropTargetId: null,
  dropPosition: null,
  startDrag: () => {},
  setDropTarget: () => {},
  clearDropTarget: () => {},
  endDrag: () => {},
  setOnDrop: () => {},
});

export function DragProvider({ children }: { children: ReactNode }) {
  // Visible state drives rendering; a mirror ref lets event handlers read the
  // latest values without re-creating callbacks on every change.
  const [visible, setVisible] = useState<DragState>({
    draggingId: null,
    dropTargetId: null,
    dropPosition: null,
  });
  const stateRef = useRef<DragState>(visible);
  const onDropRef = useRef<DropHandler | null>(null);

  const update = useCallback((patch: Partial<DragState>) => {
    setVisible((prev) => {
      const next = { ...prev, ...patch };
      stateRef.current = next;
      return next;
    });
  }, []);

  const startDrag = useCallback((nodeId: string) => {
    update({ draggingId: nodeId, dropTargetId: null, dropPosition: null });
  }, [update]);

  const setDropTarget = useCallback((nodeId: string, position: "before" | "after" | "into") => {
    update({ dropTargetId: nodeId, dropPosition: position });
  }, [update]);

  const clearDropTarget = useCallback(() => {
    update({ dropTargetId: null, dropPosition: null });
  }, [update]);

  const endDrag = useCallback(() => {
    const { draggingId, dropTargetId, dropPosition } = stateRef.current;
    if (draggingId && dropTargetId && dropPosition && draggingId !== dropTargetId) {
      onDropRef.current?.(draggingId, dropTargetId, dropPosition);
    }
    update({ draggingId: null, dropTargetId: null, dropPosition: null });
  }, [update]);

  const setOnDrop = useCallback((handler: DropHandler) => {
    onDropRef.current = handler;
  }, []);

  return (
    <Ctx.Provider
      value={{
        draggingId: visible.draggingId,
        dropTargetId: visible.dropTargetId,
        dropPosition: visible.dropPosition,
        startDrag,
        setDropTarget,
        clearDropTarget,
        endDrag,
        setOnDrop,
      }}
    >
      {children}
    </Ctx.Provider>
  );
}

export function useDrag() {
  return useContext(Ctx);
}
