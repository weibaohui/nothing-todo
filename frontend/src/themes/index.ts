import type { ThemeConfig } from 'antd';
import { theme } from 'antd';

const sharedToken = {
  colorPrimary: '#0891b2',
  colorSuccess: '#22c55e',
  colorWarning: '#f59e0b',
  colorError: '#ef4444',
  colorInfo: '#3b82f6',
  borderRadius: 12,
  borderRadiusLG: 16,
  borderRadiusSM: 8,
  fontFamily: "'JetBrains Mono', 'SF Mono', 'Cascadia Code', monospace",
  fontSize: 14,
  controlHeight: 40,
  lineHeight: 1.5,
};

const sharedComponents = {
  Button: {
    borderRadius: 10,
    controlHeight: 40,
    paddingInline: 20,
  },
  Card: {
    borderRadius: 16,
    paddingLG: 24,
  },
  Modal: {
    borderRadiusLG: 16,
    paddingContentHorizontalLG: 24,
  },
  Input: {
    borderRadius: 10,
    paddingInline: 14,
  },
  Select: {
    borderRadius: 10,
  },
  Tag: {
    borderRadius: 6,
  },
  Switch: {
    colorPrimary: '#0891b2',
  },
};

export const lightTheme: ThemeConfig = {
  algorithm: theme.defaultAlgorithm,
  token: {
    ...sharedToken,
    colorBgContainer: '#ffffff',
    colorBgLayout: '#f8fafc',
    colorText: '#0f172a',
    colorTextSecondary: '#475569',
    colorBorder: '#e2e8f0',
    colorBorderSecondary: '#f1f5f9',
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.08)',
    boxShadowSecondary: '0 8px 24px rgba(0, 0, 0, 0.12)',
  },
  components: sharedComponents,
};

export const darkTheme: ThemeConfig = {
  algorithm: theme.darkAlgorithm,
  token: {
    ...sharedToken,
    colorBgContainer: '#1e1e2e',
    colorBgLayout: '#11111b',
    colorText: '#cdd6f4',
    colorTextSecondary: '#a6adc8',
    colorBorder: '#313244',
    colorBorderSecondary: '#262637',
    boxShadow: '0 4px 12px rgba(0, 0, 0, 0.3)',
    boxShadowSecondary: '0 8px 24px rgba(0, 0, 0, 0.4)',
  },
  components: sharedComponents,
};

export type ThemeMode = 'light' | 'dark';

export const themeMap: Record<ThemeMode, ThemeConfig> = {
  light: lightTheme,
  dark: darkTheme,
};
