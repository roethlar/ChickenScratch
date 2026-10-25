import type { ReactNode } from "react";

/**
 * Phone-width home for the binder (PLAN_RELOAD_SAFETY_MOBILE slice 2): an
 * off-canvas panel over a scrim. Closed, it renders nothing, so the binder's
 * own effects and listeners only exist while it is on screen. Tapping the
 * scrim closes it; App also closes it when a document is picked.
 */
export function BinderDrawer({
  open,
  onClose,
  children,
}: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  if (!open) return null;
  return (
    <>
      <div className="binder-scrim" onClick={onClose} aria-hidden="true" />
      <div className="binder-drawer" role="dialog" aria-label="Binder">
        {children}
      </div>
    </>
  );
}
