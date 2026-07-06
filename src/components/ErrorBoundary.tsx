import { Component, ReactNode } from 'react'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error) {
    console.error('Error caught by boundary:', error)
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-gray-900 text-white flex items-center justify-center p-4">
          <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-8 max-w-md">
            <h2 className="text-2xl font-bold text-red-300 mb-4">Something went wrong</h2>
            <p className="text-gray-300 mb-4">
              An unexpected error occurred. Please refresh the page to continue.
            </p>
            {this.state.error && (
              <details className="text-sm text-gray-400 mb-6 p-3 bg-gray-800 rounded border border-gray-700">
                <summary className="cursor-pointer font-medium text-gray-300 mb-2">
                  Error details
                </summary>
                <pre className="whitespace-pre-wrap break-words">
                  {this.state.error.toString()}
                </pre>
              </details>
            )}
            <button
              onClick={() => window.location.reload()}
              className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-medium transition-colors"
            >
              Refresh Page
            </button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
