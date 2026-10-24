import { useId, useRef, useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { PROFILES, type ProfileId } from "../../profiles";
import { useModalFocusTrap } from "../shared/Dialog";

export interface NewProjectChoice {
  /** Present only when the dialog asked for a name (browser shell). */
  name: string | null;
  /** The `project_type` value to store, e.g. "Notes" or "Novel". */
  projectType: string;
}

interface NewProjectDialogProps {
  open: boolean;
  /** Browser shell: no save dialog, so the name is asked here. */
  askName: boolean;
  defaultProfile: ProfileId;
  onSubmit: (choice: NewProjectChoice) => void;
  onCancel: () => void;
}

const ORDER: ProfileId[] = ["notes", "fiction", "document"];

/**
 * What kind of project is this? The answer picks the profile (which parts of
 * the app show and what the folders are called); it can be changed later in
 * Preview → Edit Details, and nothing on disk differs between types.
 */
export function NewProjectDialog({ open, askName, defaultProfile, onSubmit, onCancel }: NewProjectDialogProps) {
  const [name, setName] = useState("");
  const [profileId, setProfileId] = useState<ProfileId>(defaultProfile);
  const titleId = useId();
  const nameId = useId();
  const firstRef = useRef<HTMLInputElement>(null);
  const { dialogRef, onDialogKeyDown } = useModalFocusTrap<HTMLDivElement>(open, onCancel, firstRef);

  if (!open) return null;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit({
      name: askName ? name : null,
      projectType: PROFILES[profileId].defaultType,
    });
  };

  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div
        ref={dialogRef}
        className="new-project-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onDialogKeyDown}
      >
        <div className="compile-header">
          <h3 id={titleId}>New Project</h3>
          <button onClick={onCancel} className="compile-close" aria-label="Close new project dialog">
            <X size={16} />
          </button>
        </div>
        <form onSubmit={submit} className="new-project-form">
          {askName && (
            <div className="compile-field">
              <label htmlFor={nameId}>Name</label>
              <input
                id={nameId}
                ref={firstRef}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Notes"
                autoComplete="off"
              />
            </div>
          )}
          <fieldset className="new-project-types">
            <legend>What kind of project?</legend>
            {ORDER.map((id, index) => (
              <label key={id} className={`new-project-type ${profileId === id ? "selected" : ""}`}>
                <input
                  type="radio"
                  name="project-type"
                  value={id}
                  checked={profileId === id}
                  onChange={() => setProfileId(id)}
                  ref={!askName && index === 0 ? firstRef : undefined}
                />
                <span className="new-project-type-label">{PROFILES[id].label}</span>
                <span className="new-project-type-hint">{PROFILES[id].hint}</span>
              </label>
            ))}
          </fieldset>
          <p className="compile-hint">You can change the type later under Preview → Edit Details.</p>
          <div className="compile-footer">
            <button type="button" className="dialog-btn cancel" onClick={onCancel}>Cancel</button>
            <button type="submit" className="dialog-btn ok" disabled={askName && !name.trim()}>
              Create
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
