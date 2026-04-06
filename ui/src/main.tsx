import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './editor.css'
import App from './App.tsx'
import { DialogProvider } from './components/shared/Dialog.tsx'
import { ToastProvider } from './components/shared/Toast.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
    <DialogProvider />
    <ToastProvider />
  </StrictMode>,
)
