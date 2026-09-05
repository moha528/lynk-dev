import { Component, type ErrorInfo, type ReactNode } from "react";

import { Button } from "./ui/Button";

type Props = { children: ReactNode };
type State = { error: Error | null };

/**
 * Isolates a module crash from the shell.
 *
 * Without it, a single bad render in Git or Dev takes the whole window down —
 * including the settings and the tray behaviour. Mount one per module and key
 * it on the module id so switching modules clears a previous crash.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("module crash:", error, info.componentStack);
  }

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
        <p className="text-sm font-medium text-(--color-danger)">Ce module a planté.</p>
        <p className="max-w-md break-words font-mono text-xs text-(--color-muted)">
          {error.message}
        </p>
        <Button variant="outline" size="sm" onClick={() => this.setState({ error: null })}>
          Réessayer
        </Button>
      </div>
    );
  }
}
