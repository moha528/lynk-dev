import { useCallback, useEffect, useRef } from "react";

type SidebarResizerProps = {
  onResize: (width: number) => void;
  minWidth?: number;
  maxWidth?: number;
};

export function SidebarResizer({ onResize, minWidth = 180, maxWidth = 480 }: SidebarResizerProps) {
  const dragging = useRef(false);

  const onMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!dragging.current) return;
      const next = Math.min(maxWidth, Math.max(minWidth, e.clientX));
      onResize(next);
    },
    [onResize, minWidth, maxWidth],
  );

  const onMouseUp = useCallback(() => {
    dragging.current = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);

  useEffect(() => {
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [onMouseMove, onMouseUp]);

  const onMouseDown = () => {
    dragging.current = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };

  return (
    <div
      aria-orientation="vertical"
      aria-label="Resize sidebar"
      onMouseDown={onMouseDown}
      className="group relative w-px shrink-0 cursor-col-resize bg-(--color-border)"
    >
      <span className="absolute inset-y-0 -left-1 -right-1 transition-colors hover:bg-(--color-accent)/30" />
    </div>
  );
}
