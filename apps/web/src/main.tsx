import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { I18nextProvider } from 'react-i18next';
import App from './App';
import { createI18n } from './i18n';
import { selftestOnStartup } from './engine/selftestOnStartup';

const i18n = createI18n();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nextProvider i18n={i18n}>
      <App />
    </I18nextProvider>
  </StrictMode>,
);

void selftestOnStartup();
