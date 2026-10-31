import React, { createContext, useContext, useMemo } from 'react';
import { appConfig, type AppConfig } from './app-config';

const ConfigContext = createContext<AppConfig>(appConfig);

export const useAppConfig = () => useContext(ConfigContext);

export interface AppProvidersProps {
  children: React.ReactNode;
}

export const AppProviders: React.FC<AppProvidersProps> = ({ children }) => {
  const value = useMemo(() => appConfig, []);

  return (
    <ConfigContext.Provider value={value}>
      {children}
    </ConfigContext.Provider>
  );
};
