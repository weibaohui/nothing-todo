import { createContext, useContext, useState, useLayoutEffect, type ReactNode } from 'react';
import type { ThemeConfig } from 'antd';
import type { ThemeMode } from '../themes';
import { themeMap } from '../themes';

interface ThemeContextValue {
  themeMode: ThemeMode;
  themeConfig: ThemeConfig;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'app_theme';

function getInitialTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'dark' || saved === 'light') return saved;
  } catch {}
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeMode, setThemeMode] = useState<ThemeMode>(getInitialTheme);

  useLayoutEffect(() => {
    document.documentElement.setAttribute('data-theme', themeMode);
  }, [themeMode]);

  useLayoutEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, themeMode);
    } catch {}
  }, [themeMode]);

  const toggleTheme = () => {
    setThemeMode(prev => (prev === 'light' ? 'dark' : 'light'));
  };

  const themeConfig = themeMap[themeMode];

  return (
    <ThemeContext.Provider value={{ themeMode, themeConfig, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
