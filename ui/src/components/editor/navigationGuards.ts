import { useProjectStore, type FlowDoc } from "../../stores/projectStore";
import { flushPendingEditorSave } from "./editorRef";

export async function flushEditorBeforeNavigation(): Promise<boolean> {
  try {
    await flushPendingEditorSave();
    return true;
  } catch {
    return false;
  }
}

export async function selectDocumentWithEditorFlush(
  docId: string
): Promise<boolean> {
  if (!(await flushEditorBeforeNavigation())) return false;
  useProjectStore.getState().selectDocument(docId);
  return true;
}

export async function clearDocumentSelectionWithEditorFlush(): Promise<boolean> {
  if (!(await flushEditorBeforeNavigation())) return false;
  useProjectStore.setState({ activeDocId: null, activeDoc: null });
  return true;
}

export async function enterFlowWithEditorFlush(
  docs: FlowDoc[]
): Promise<boolean> {
  if (!(await flushEditorBeforeNavigation())) return false;
  useProjectStore.getState().enterFlow(docs);
  return true;
}

export async function exitFlowWithEditorFlush(): Promise<boolean> {
  if (!(await flushEditorBeforeNavigation())) return false;
  useProjectStore.getState().exitFlow();
  return true;
}
