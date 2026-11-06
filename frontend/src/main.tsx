import React from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './style.css'
import './source-theme.css'
import './readability.css'
import './ui-craft.css'
import { applyReadability } from './ReadabilityPanel'

applyReadability()

createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>)
