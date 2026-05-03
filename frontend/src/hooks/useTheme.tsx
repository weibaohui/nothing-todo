import { createContext, useContext, useState, useLayoutEffect, useMemo, type ReactNode } from 'react';
import type { ThemeConfig, ConfigProviderProps } from 'antd';
import { theme } from 'antd';
import type { ThemeMode } from '../themes';
import { themeMap } from '../themes';
import { createStyles } from 'antd-style';
import clsx from 'clsx';

export type VisualMode = 'none' | 'frosted' | 'glass' | 'illustration';

interface ThemeContextValue {
  themeMode: ThemeMode;
  visualMode: VisualMode;
  themeConfig: ThemeConfig;
  visualConfig: ConfigProviderProps;
  toggleTheme: () => void;
  setVisualMode: (mode: VisualMode) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'app_theme';
const VISUAL_STORAGE_KEY = 'app_visual_mode';

// Glass styles using antd-style
const useGlassStyles = createStyles(({ css, token }) => ({
  glassBorder: css({
    boxShadow: [
      `${token.boxShadowSecondary}`,
      `inset 0 0 5px 2px rgba(255, 255, 255, 0.3)`,
      `inset 0 5px 2px rgba(255, 255, 255, 0.2)`,
    ].join(','),
  }),
  glassBox: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 15%, transparent)`,
    backdropFilter: 'blur(12px)',
  }),
  notBackdropFilter: css({
    backdropFilter: 'none',
  }),
  cardRoot: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 40%, transparent)`,
    backdropFilter: 'blur(12px)',
  }),
  modalContainer: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 15%, transparent)`,
    backdropFilter: 'blur(12px)',
  }),
  buttonRoot: css({
    background: 'transparent',
    color: token.colorText,
    borderColor: 'rgba(255, 255, 255, 0.2)',

    '&:hover': {
      background: 'rgba(255,255,255,0.2)',
      color: `color-mix(in srgb, ${token.colorText} 90%, transparent)`,
    },

    '&:active': {
      background: 'rgba(255,255,255,0.1)',
      color: `color-mix(in srgb, ${token.colorText} 80%, transparent)`,
    },
  }),
  dropdownRoot: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 15%, transparent)`,
    backdropFilter: 'blur(12px)',
    borderRadius: token.borderRadiusLG,
  }),
  inputRoot: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 15%, transparent)`,
    backdropFilter: 'blur(12px)',
  }),
  selectRoot: css({
    background: `color-mix(in srgb, ${token.colorBgContainer} 15%, transparent)`,
    backdropFilter: 'blur(12px)',
  }),
}));

// Illustration styles using antd-style
const useIllustrationStyles = createStyles(({ css }) => ({
  illustrationBorder: css({
    border: `3px solid #2C2C2C`,
  }),
  illustrationBox: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `4px 4px 0 #2C2C2C`,
  }),
  buttonRoot: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `4px 4px 0 #2C2C2C`,
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.5px',
  }),
  modalContainer: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `4px 4px 0 #2C2C2C`,
  }),
  tooltipRoot: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `4px 4px 0 #2C2C2C`,
  }),
  popupBox: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `4px 4px 0 #2C2C2C`,
    borderRadius: 12,
    backgroundColor: '#FFFFFF',
  }),
  progressRail: css({
    border: `3px solid #2C2C2C`,
    boxShadow: `2px 2px 0 #2C2C2C`,
  }),
  progressTrack: css({
    border: 'none',
    boxShadow: 'none',
  }),
  inputNumberActions: css({
    width: 12,
  }),
}));

function getInitialTheme(): ThemeMode {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'dark' || saved === 'light') return saved;
  } catch {}
  return 'light';
}

function getInitialVisualMode(): VisualMode {
  try {
    const saved = localStorage.getItem(VISUAL_STORAGE_KEY);
    if (saved === 'none' || saved === 'frosted' || saved === 'glass' || saved === 'illustration') return saved;
  } catch {}
  return 'none';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [themeMode, setThemeMode] = useState<ThemeMode>(getInitialTheme);
  const [visualMode, setVisualMode] = useState<VisualMode>(getInitialVisualMode);
  const { styles: glassStyles } = useGlassStyles();
  const { styles: illustrationStyles } = useIllustrationStyles();

  useLayoutEffect(() => {
    document.documentElement.setAttribute('data-theme', themeMode);
  }, [themeMode]);

  useLayoutEffect(() => {
    if (visualMode === 'none') {
      document.documentElement.removeAttribute('data-visual');
    } else {
      document.documentElement.setAttribute('data-visual', visualMode);
    }
  }, [visualMode]);

  useLayoutEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, themeMode);
    } catch {}
  }, [themeMode]);

  useLayoutEffect(() => {
    try {
      localStorage.setItem(VISUAL_STORAGE_KEY, visualMode);
    } catch {}
  }, [visualMode]);

  const toggleTheme = () => {
    setThemeMode(prev => (prev === 'light' ? 'dark' : 'light'));
  };

  const themeConfig = themeMap[themeMode];

  const visualConfig = useMemo<ConfigProviderProps>(() => {
    if (visualMode === 'none') {
      return {};
    }

    if (visualMode === 'illustration') {
      return {
        theme: {
          algorithm: theme.defaultAlgorithm,
          token: {
            colorText: '#2C2C2C',
            colorPrimary: '#52C41A',
            colorSuccess: '#51CF66',
            colorWarning: '#FFD93D',
            colorError: '#FA5252',
            colorInfo: '#4DABF7',
            colorBorder: '#2C2C2C',
            colorBorderSecondary: '#2C2C2C',
            lineWidth: 3,
            lineWidthBold: 3,
            borderRadius: 12,
            borderRadiusLG: 16,
            borderRadiusSM: 8,
            controlHeight: 40,
            controlHeightSM: 34,
            controlHeightLG: 48,
            fontSize: 15,
            fontWeightStrong: 600,
            colorBgBase: '#FFF9F0',
            colorBgContainer: '#FFFFFF',
          },
          components: {
            Button: {
              primaryShadow: 'none',
              dangerShadow: 'none',
              defaultShadow: 'none',
              fontWeight: 600,
            },
            Modal: {
              boxShadow: 'none',
            },
            Card: {
              boxShadow: '4px 4px 0 #2C2C2C',
              colorBgContainer: '#FFF0F6',
            },
            Tooltip: {
              colorBorder: '#2C2C2C',
              colorBgSpotlight: 'rgba(100, 100, 100, 0.95)',
              borderRadius: 8,
            },
            Select: {
              optionSelectedBg: 'transparent',
            },
            Slider: {
              dotBorderColor: '#237804',
              dotActiveBorderColor: '#237804',
              colorPrimaryBorder: '#237804',
              colorPrimaryBorderHover: '#237804',
            },
          },
        },
        button: {
          classNames: {
            root: illustrationStyles.buttonRoot,
          },
        },
        modal: {
          classNames: {
            container: illustrationStyles.modalContainer,
          },
        },
        alert: {
          className: illustrationStyles.illustrationBorder,
        },
        colorPicker: {
          arrow: false,
          classNames: {
            root: illustrationStyles.illustrationBox,
          },
        },
        popover: {
          classNames: {
            container: illustrationStyles.illustrationBox,
          },
        },
        tooltip: {
          arrow: false,
          classNames: {
            root: illustrationStyles.tooltipRoot,
            container: illustrationStyles.illustrationBox,
          },
        },
        dropdown: {
          classNames: {
            root: illustrationStyles.popupBox,
          },
        },
        select: {
          classNames: {
            root: illustrationStyles.illustrationBox,
            popup: {
              root: illustrationStyles.popupBox,
            },
          },
        },
        input: {
          classNames: {
            root: illustrationStyles.illustrationBox,
          },
        },
        inputNumber: {
          classNames: {
            root: illustrationStyles.illustrationBox,
            actions: illustrationStyles.inputNumberActions,
          },
        },
        progress: {
          classNames: {
            rail: illustrationStyles.progressRail,
            track: illustrationStyles.progressTrack,
          },
          styles: {
            rail: {
              height: 16,
            },
            track: {
              height: 10,
            },
          },
        },
      };
    }

    // Glass/frosted mode
    return {
      button: {
        classNames: ({ props }: { props: Record<string, unknown> }) => ({
          root: clsx(
            glassStyles.buttonRoot,
            props.variant === 'primary' && glassStyles.glassBorder,
          ),
        }),
      },
      card: {
        classNames: {
          root: clsx(glassStyles.cardRoot, glassStyles.glassBorder),
        },
      },
      modal: {
        classNames: {
          container: clsx(glassStyles.modalContainer, glassStyles.glassBorder),
        },
      },
      dropdown: {
        classNames: {
          root: clsx(glassStyles.dropdownRoot, glassStyles.glassBorder),
        },
      },
      input: {
        classNames: {
          root: clsx(glassStyles.inputRoot, glassStyles.glassBorder),
        },
      },
      select: {
        classNames: {
          root: clsx(glassStyles.selectRoot, glassStyles.glassBorder),
        },
      },
    };
  }, [visualMode, glassStyles, illustrationStyles]);

  return (
    <ThemeContext.Provider value={{ themeMode, visualMode, themeConfig, visualConfig, toggleTheme, setVisualMode }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
