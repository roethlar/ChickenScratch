import { create } from "zustand";

type Theme = "light" | "dark" | "sepia";

interface SettingsState {
  theme: Theme;
  focusMode: boolean;
  setTheme: (theme: Theme) => void;
  toggleFocusMode: () => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  theme: (localStorage.getItem("chickenscratch-theme") as Theme) || "dark",
  focusMode: false,

  setTheme: (theme: Theme) => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("chickenscratch-theme", theme);
    set({ theme });
  },

  toggleFocusMode: () => {
    set((s) => ({ focusMode: !s.focusMode }));
  },
}));

// Apply saved theme on load
const saved = localStorage.getItem("chickenscratch-theme");
if (saved) {
  document.documentElement.setAttribute("data-theme", saved);
}
