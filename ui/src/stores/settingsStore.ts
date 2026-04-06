import { create } from "zustand";
import { getAppSettings, type AppSettings } from "../commands/settings";

type Theme = "light" | "dark" | "sepia";

interface SettingsState {
  theme: Theme;
  focusMode: boolean;
  appSettings: AppSettings | null;
  setTheme: (theme: Theme) => void;
  toggleFocusMode: () => void;
  loadSettings: () => Promise<void>;
  applyWritingSettings: () => void;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  theme: (localStorage.getItem("chickenscratch-theme") as Theme) || "dark",
  focusMode: false,
  appSettings: null,

  setTheme: (theme: Theme) => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("chickenscratch-theme", theme);
    set({ theme });
  },

  toggleFocusMode: () => {
    set((s) => ({ focusMode: !s.focusMode }));
  },

  loadSettings: async () => {
    try {
      const settings = await getAppSettings();
      set({ appSettings: settings });
      // Apply theme from settings
      const theme = settings.general.theme as Theme;
      document.documentElement.setAttribute("data-theme", theme);
      localStorage.setItem("chickenscratch-theme", theme);
      set({ theme });
      // Apply writing settings
      get().applyWritingSettings();
    } catch {
      // Use defaults
    }
  },

  applyWritingSettings: () => {
    const { appSettings } = get();
    if (!appSettings) return;
    const w = appSettings.writing;
    const root = document.documentElement;
    root.style.setProperty("--editor-font", `"${w.font_family}", Georgia, serif`);
    root.style.setProperty("--editor-font-size", `${w.font_size}px`);
    root.style.setProperty(
      "--editor-paragraph-margin",
      w.paragraph_style === "indent" ? "0" : "0.6em 0"
    );
    root.style.setProperty(
      "--editor-text-indent",
      w.paragraph_style === "indent" ? "2em" : "0"
    );
  },
}));

// Apply saved theme on load
const saved = localStorage.getItem("chickenscratch-theme");
if (saved) {
  document.documentElement.setAttribute("data-theme", saved);
}
