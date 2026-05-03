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
    if (saved === 'dark' || saved === 'light' || saved === 'auto') return saved;
  } catch {}
  return 'auto';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeMode, setThemeMode] = useState<ThemeMode>(getInitialTheme);

  // 计算解析后的主题（auto 模式根据系统偏好）
  const getResolvedTheme = (mode: ThemeMode): 'light' | 'dark' => {
    if (mode === 'auto') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return mode;
  };

  const resolvedTheme = getResolvedTheme(themeMode);

  useLayoutEffect(() => {
    document.documentElement.setAttribute('data-theme', resolvedTheme);

    // 监听系统主题变化，当处于 auto 模式时自动更新
    if (themeMode === 'auto') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handler = (e: MediaQueryListEvent) => {
        document.documentElement.setAttribute('data-theme', e.matches ? 'dark' : 'light');
      };
      mediaQuery.addEventListener('change', handler);
      return () => mediaQuery.removeEventListener('change', handler);
    }
  }, [themeMode, resolvedTheme]);

  useLayoutEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, themeMode);
    } catch {}
  }, [themeMode]);

  const toggleTheme = () => {
    setThemeMode(prev => {
      if (prev === 'light') return 'dark';
      if (prev === 'dark') return 'auto';
      return 'light';
    });
  };

  const themeConfig = themeMap[resolvedTheme];

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
