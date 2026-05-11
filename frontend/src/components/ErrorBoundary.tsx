import { Component, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('ErrorBoundary caught:', error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      const isDev = typeof window !== 'undefined' && window.location.hostname === 'localhost';
      const displayMessage = isDev
        ? this.state.error?.message
        : '页面发生错误，请稍后重试。';
      return (
        <div style={{ padding: '24px', textAlign: 'center' }} role="alert">
          <h3>出错了</h3>
          <p style={{ color: '#666', fontSize: 14 }}>{displayMessage}</p>
          <button
            onClick={() => window.location.reload()}
            style={{ marginTop: 12, padding: '6px 16px', cursor: 'pointer' }}
          >
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}