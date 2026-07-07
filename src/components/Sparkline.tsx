import { useMemo } from "react";
import type { HourlyPoint } from "../types";

interface SparklineProps {
  data: HourlyPoint[];
  width?: number;
  height?: number;
  field?: "cost" | "tokens";
}

export function Sparkline({ data, width = 120, height = 28, field = "cost" }: SparklineProps) {
  const path = useMemo(() => {
    if (!data.length) return "";
    const values = data.map((d) => (field === "cost" ? d.cost : d.tokens));
    const max = Math.max(...values, 0.001);
    const min = Math.min(...values, 0);
    const range = max - min || 1;
    const step = width / Math.max(values.length - 1, 1);

    return values
      .map((v, i) => {
        const x = i * step;
        const y = height - ((v - min) / range) * (height - 4) - 2;
        return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  }, [data, width, height, field]);

  const areaPath = path ? `${path} L${width},${height} L0,${height} Z` : "";

  return (
    <svg width={width} height={height} style={{ overflow: "visible" }}>
      <defs>
        <linearGradient id="sparkGrad" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.35" />
          <stop offset="100%" stopColor="var(--accent)" stopOpacity="0" />
        </linearGradient>
      </defs>
      {areaPath && <path d={areaPath} fill="url(#sparkGrad)" />}
      {path && (
        <path
          d={path}
          fill="none"
          stroke="var(--accent)"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          opacity={0.9}
        />
      )}
    </svg>
  );
}
