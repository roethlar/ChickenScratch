import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './editor.css'
import App from './App.tsx'
import { DialogProvider } from './components/shared/Dialog.tsx'
import { ToastProvider } from './components/shared/Toast.tsx'
import { ErrorBoundary } from './components/shared/ErrorBoundary.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
      <DialogProvider />
      <ToastProvider />
    </ErrorBoundary>
  </StrictMode>,
)
