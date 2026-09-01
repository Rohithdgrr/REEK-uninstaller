import React from "react";

type Props = { children: React.ReactNode };
type State = { hasError: boolean; error: string | null };

export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(err: unknown): State {
    return { hasError: true, error: err instanceof Error ? err.message : String(err) };
  }

  componentDidCatch(error: unknown, info: React.ErrorInfo) {
    console.error("[ErrorBoundary]", error, info);
    // Log to file via Tauri if available — fire-and-forget
    try {
      const msg = error instanceof Error ? error.stack ?? error.message : String(error);
      console.error(msg);
    } catch {}
  }

  handleReload = () => {
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-[#0A0A0A] flex flex-col items-center justify-center p-8 text-center">
          <div className="rounded-[16px] border border-[rgba(225,29,72,0.25)] bg-[#141414] p-8 max-w-lg w-full">
            <h1 className="text-[18px] font-semibold text-[#F5F0EB]">Something broke — Mahakali stumbled</h1>
            <p className="text-[13px] text-[#A8A39E] mt-2 break-all">{this.state.error ?? "Unknown error"}</p>
            <div className="flex gap-3 justify-center mt-6">
              <button
                onClick={this.handleReload}
                className="rounded-full bg-[#E11D48] text-white px-6 py-2 text-sm font-medium hover:bg-[#C91A40] transition"
              >
                Reload
              </button>
              <button
                onClick={() => this.setState({ hasError: false, error: null })}
                className="rounded-full border border-[rgba(255,255,255,0.12)] text-[#A8A39E] px-6 py-2 text-sm hover:text-white transition"
              >
                Dismiss
              </button>
            </div>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
