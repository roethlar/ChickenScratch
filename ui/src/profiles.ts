/**
 * Project profiles (docs/plans/PLAN_PROJECT_PROFILES.md).
 *
 * The `.chikn` format is genre-neutral; words like Characters, Locations,
 * Corkboard and Timeline are vocabulary this UI paints on top. A project's
 * stored `project_type` picks a profile that says which of those parts to
 * show and what the standard folders are called on screen. Renames are
 * display-only: `manuscript/`, `research/`, `trash/` keep their names on disk,
 * so a project stays readable by every tool and by every profile.
 */
import { useProjectStore } from "./stores/projectStore";

export type ProfileId = "fiction" | "notes" | "document";
export type StandardFolder = "Manuscript" | "Research" | "Trash";

export interface ProfileFeatures {
  /** Characters / Locations sections and entity creation. */
  entities: boolean;
  corkboard: boolean;
  timeline: boolean;
  /** Plot threads: Threads tab and the inspector's thread chips. */
  threads: boolean;
  /** The inspector's Scene section (POV, location, story time, …). */
  sceneFields: boolean;
  /** Session word targets in Statistics and the floating badge. */
  sessionTargets: boolean;
  /** Preview: type-aware section headers between documents. */
  previewSectionTitles: boolean;
}

export interface Profile {
  id: ProfileId;
  label: string;
  /** One line shown next to the type choice. */
  hint: string;
  features: ProfileFeatures;
  folderLabels: Record<StandardFolder, string>;
  /** Backend template ids offered by "New from Template". */
  templateIds: string[];
  /** Inspector Label presets. */
  labelPresets: string[];
  /** `project_type` values that map to this profile. */
  typeOptions: string[];
  /** The `project_type` New Project stores when this profile is chosen. */
  defaultType: string;
}

const ALL_ON: ProfileFeatures = {
  entities: true,
  corkboard: true,
  timeline: true,
  threads: true,
  sceneFields: true,
  sessionTargets: true,
  previewSectionTitles: true,
};

const ALL_OFF: ProfileFeatures = {
  entities: false,
  corkboard: false,
  timeline: false,
  threads: false,
  sceneFields: false,
  sessionTargets: false,
  previewSectionTitles: false,
};

export const PROFILES: Record<ProfileId, Profile> = {
  fiction: {
    id: "fiction",
    label: "Fiction",
    hint: "Characters, locations, plot threads, corkboard and timeline.",
    features: ALL_ON,
    folderLabels: { Manuscript: "Manuscript", Research: "Research", Trash: "Trash" },
    templateIds: ["scene", "chapter", "character", "setting", "outline"],
    labelPresets: ["Scene", "Chapter", "Outline", "Notes", "Research"],
    typeOptions: ["Novel", "Short Story", "Novella", "Screenplay"],
    defaultType: "Novel",
  },
  notes: {
    id: "notes",
    label: "Notes",
    hint: "Plain notes and Markdown pages; no fiction tools.",
    features: ALL_OFF,
    folderLabels: { Manuscript: "Notes", Research: "Attachments", Trash: "Trash" },
    templateIds: ["meeting", "decision", "howto", "daily"],
    labelPresets: ["Note", "Meeting", "Idea", "Reference", "Todo"],
    typeOptions: ["Notes"],
    defaultType: "Notes",
  },
  document: {
    id: "document",
    label: "Document",
    hint: "Essays, reports and technical writing with section headers.",
    features: { ...ALL_OFF, previewSectionTitles: true },
    folderLabels: { Manuscript: "Document", Research: "Research", Trash: "Trash" },
    templateIds: ["outline", "howto", "decision"],
    labelPresets: ["Section", "Draft", "Reference", "Appendix"],
    typeOptions: ["Essay", "Document"],
    defaultType: "Document",
  },
};

/** Every selectable project type, grouped by profile, in display order. */
export const PROJECT_TYPE_OPTIONS: { value: string; profile: ProfileId }[] = (
  ["notes", "fiction", "document"] as ProfileId[]
).flatMap((id) => PROFILES[id].typeOptions.map((value) => ({ value, profile: id })));

/**
 * Empty or unknown types keep today's full fiction view, so no existing
 * project loses a panel when it is opened by a newer app.
 */
export function profileFor(projectType: string | null | undefined): Profile {
  const wanted = (projectType ?? "").trim().toLowerCase();
  if (!wanted) return PROFILES.fiction;
  for (const profile of Object.values(PROFILES)) {
    if (profile.typeOptions.some((t) => t.toLowerCase() === wanted)) return profile;
  }
  return PROFILES.fiction;
}

/** The profile of the open project (fiction when none is open). */
export function useProfile(): Profile {
  const projectType = useProjectStore((s) => s.project?.metadata.project_type ?? null);
  return profileFor(projectType);
}

/** On-screen name for a standard folder; anything else is shown as stored. */
export function folderLabel(storedName: string, profile: Profile): string {
  return (profile.folderLabels as Record<string, string>)[storedName] ?? storedName;
}
