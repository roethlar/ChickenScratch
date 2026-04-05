import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './editor.css'
import App from './App.tsx'
import { DialogProvider } from './components/shared/Dialog.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
    <DialogProvider />
  </StrictMode>,
)
