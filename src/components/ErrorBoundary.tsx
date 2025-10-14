import { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

interface State {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Error caught by boundary:", error, errorInfo);
    this.props.onError?.(error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        this.props.fallback || (
          <div
            className="p-4 border border-red-500 rounded bg-red-50"
            role="alert"
          >
            <h2 className="text-red-700 font-semibold">Something went wrong</h2>
            <details className="mt-2 text-sm text-red-600">
              <summary className="cursor-pointer font-medium">
                Error details
              </summary>
              <div className="mt-2 p-2 bg-red-100 rounded">
                {this.state.error?.toString()}
              </div>
            </details>
            <div className="mt-4 flex gap-2">
              <button
                className="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2"
                onClick={() => this.setState({ hasError: false })}
                aria-label="Try again"
              >
                Try again
              </button>
              <button
                className="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2"
                onClick={() => window.location.reload()}
                aria-label="Reload page"
              >
                Reload page
              </button>
            </div>
          </div>
        )
      );
    }

    return this.props.children;
  }
}
