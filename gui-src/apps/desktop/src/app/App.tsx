import React from 'react';
import { AppProviders } from './app-providers';
import { AppRouter } from './app-router';

export const App: React.FC = () => {
  return (
    <AppProviders>
      {/* Global Inset-Shell Treatment wrapping all routes */}
      <div className="global-app-shell">
        <div className="global-app-shell-inner">
          <AppRouter />
        </div>
      </div>
    </AppProviders>
  );
};
