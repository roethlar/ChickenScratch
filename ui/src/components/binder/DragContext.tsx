import { createContext, useContext, useState, useCallback, useRef, type ReactNode } from "react";

interface DragState {
  draggingId: string | null;
  dropTargetId: string | null;
  dropPosition: "before" | "after" | "into" | null;
}

interface DragContextType extends DragState {
  startDrag: (nodeId: string) => void;
  setDropTarget: (nodeId: string, position: "before" | "after" | "into") => void;
  clearDropTarget: () => void;
  endDrag: () => string | null; // returns the dragging ID
  onDrop: ((dragId: string, targetId: string, position: "before" | "after" | "into") => void) | null;
  setOnDrop: (handler: (dragId: string, targetId: string, position: "before" | "after" | "into") => void) => void;
}

const Ctx = createContext<DragContextType>({
  draggingId: null,
  dropTargetId: null,
  dropPosition: null,
  startDrag: () => {},
  setDropTarget: () => {},
  clearDropTarget: () => {},
  endDrag: () => null,
  onDrop: null,
  setOnDrop: () => {},
});

export function DragProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<DragState>({
    draggingId: null,
    dropTargetId: null,
    dropPosition: null,
  });
  const onDropRef = useRef<DragContextType["onDrop"]>(null);

  const startDrag = useCallback((nodeId: string) => {
    setState({ draggingId: nodeId, dropTargetId: null, dropPosition: null });
  }, []);

  const setDropTarget = useCallback((nodeId: string, position: "before" | "after" | "into") => {
    setState((s) => ({ ...s, dropTargetId: nodeId, dropPosition: position }));
  }, []);

  const clearDropTarget = useCallback(() => {
    setState((s) => ({ ...s, dropTargetId: null, dropPosition: null }));
  }, []);

  const endDrag = useCallback(() => {
    const { draggingId, dropTargetId, dropPosition } = state;
    if (draggingId && dropTargetId && dropPosition && draggingId !== dropTargetId) {
      onDropRef.current?.(draggingId, dropTargetId, dropPosition);
    }
    const id = draggingId;
    setState({ draggingId: null, dropTargetId: null, dropPosition: null });
    return id;
  }, [state]);

  const setOnDrop = useCallback((handler: DragContextType["onDrop"]) => {
    onDropRef.current = handler;
  }, []);

  return (
    <Ctx.Provider
      value={{
        ...state,
        startDrag,
        setDropTarget,
        clearDropTarget,
        endDrag,
        onDrop: onDropRef.current,
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
