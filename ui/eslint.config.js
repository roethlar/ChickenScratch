import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
  },
  {
    // Components must dispatch project mutations through src/commands/*
    // so the operation-barrier gate sees every write; a component-level
    // `invoke` bypasses it (epoch-guard plan slice 3, review round 7).
    files: ['src/components/**/*.{ts,tsx}', 'src/stores/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@tauri-apps/api/core',
              importNames: ['invoke'],
              message:
                'Dispatch Tauri commands through src/commands/* (barrier-gated), not directly from components.',
            },
          ],
        },
      ],
    },
  },
  {
    // The desktop runtime is reachable from exactly two files: the command
    // transport (IPC vs HTTP) and the platform seam (dialogs, links, close
    // hook). Everything else must stay shell-agnostic so the same UI runs in
    // the Tauri app and in the chikn-web browser shell (ADR-007).
    files: ['src/**/*.{ts,tsx}'],
    ignores: ['src/commands/transport.ts', 'src/platform.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@tauri-apps/api/core',
              importNames: ['invoke'],
              message:
                'Dispatch commands through src/commands/* (barrier-gated), not directly from components.',
            },
          ],
          patterns: [
            {
              group: ['@tauri-apps/*'],
              message:
                'Only src/commands/transport.ts and src/platform.ts may touch the Tauri runtime; go through them.',
            },
          ],
        },
      ],
    },
  },
])
