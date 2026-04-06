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
  // Use refs for mutable state that needs to be read in event handlers
  // Use useState only to trigger re-renders for visual feedback
  const stateRef = useRef<DragState>({
    draggingId: null,
    dropTargetId: null,
    dropPosition: null,
  });
  const onDropRef = useRef<DropHandler | null>(null);
  const [, forceRender] = useState(0);

  const update = useCallback((patch: Partial<DragState>) => {
    Object.assign(stateRef.current, patch);
    forceRender((n) => n + 1);
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
        draggingId: stateRef.current.draggingId,
        dropTargetId: stateRef.current.dropTargetId,
        dropPosition: stateRef.current.dropPosition,
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
