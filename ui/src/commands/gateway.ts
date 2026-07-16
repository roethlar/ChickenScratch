import { invoke } from "@tauri-apps/api/core";
import { assertDispatchAllowed, type LeaseHandle } from "./barrier";

/**
 * Gated dispatch for every project-mutating Tauri command (plan slice 3).
 * While an epoch-bumping operation holds a barrier lease, non-owner
 * mutations are refused — never deferred — so captured pre-operation
 * arguments can never land under a fresh token after the operation
 * replaces the tree. The lease owner's own dispatches (drain, operation,
 * post-operation reload) pass their handle.
 *
 * Read-only commands keep calling `invoke` directly.
 */
export async function mutatingInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
  lease?: LeaseHandle
): Promise<T> {
  assertDispatchAllowed(command, lease);
  return invoke<T>(command, args);
}
