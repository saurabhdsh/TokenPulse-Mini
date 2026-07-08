import { useWindowDrag } from "../hooks/useWindowDrag";

interface WindowTitleBarProps {
  title: string;
  subtitle?: string;
  pinned: boolean;
  onTogglePin: () => void;
  onCollapse: () => void;
  onHide: () => void;
}

const dragRegion = { "data-tauri-drag-region": true } as const;

export function WindowTitleBar({
  title,
  subtitle,
  pinned,
  onTogglePin,
  onCollapse,
  onHide,
}: WindowTitleBarProps) {
  const onDragMouseDown = useWindowDrag();

  return (
    <header
      className="window-titlebar"
      {...dragRegion}
      onMouseDown={onDragMouseDown}
    >
      <div className="window-titlebar-leading" {...dragRegion}>
        <span className="window-titlebar-dot" {...dragRegion} />
        <div {...dragRegion}>
          <div className="window-titlebar-title" {...dragRegion}>{title}</div>
          {subtitle ? (
            <div className="window-titlebar-subtitle" {...dragRegion}>{subtitle}</div>
          ) : null}
        </div>
        <span className="window-titlebar-hint" {...dragRegion}>Drag to move</span>
      </div>
      <div className="window-titlebar-actions no-drag" data-no-drag>
        <TitleBarButton title={pinned ? "Unpin" : "Pin on top"} onClick={onTogglePin} active={pinned}>
          📌
        </TitleBarButton>
        <TitleBarButton title="Collapse to widget" onClick={onCollapse}>
          ⊟
        </TitleBarButton>
        <TitleBarButton title="Hide" onClick={onHide}>
          −
        </TitleBarButton>
      </div>
    </header>
  );
}

function TitleBarButton({
  children,
  onClick,
  title,
  active,
}: {
  children: React.ReactNode;
  onClick: () => void;
  title: string;
  active?: boolean;
}) {
  return (
    <button type="button" className="titlebar-btn" title={title} onClick={onClick} data-active={active}>
      {children}
    </button>
  );
}
