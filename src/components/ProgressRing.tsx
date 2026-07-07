import { motion } from "framer-motion";

interface ProgressRingProps {
  pct: number;
  size?: number;
  stroke?: number;
}

export function ProgressRing({ pct, size = 44, stroke = 3.5 }: ProgressRingProps) {
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(pct, 100);
  const offset = circumference - (clamped / 100) * circumference;

  const color =
    pct >= 100 ? "var(--critical)" :
    pct >= 80 ? "#fb923c" :
    pct >= 50 ? "var(--warning)" :
    "var(--accent)";

  return (
    <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="rgba(255,255,255,0.06)"
        strokeWidth={stroke}
      />
      <motion.circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke={color}
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeDasharray={circumference}
        initial={{ strokeDashoffset: circumference }}
        animate={{ strokeDashoffset: offset }}
        transition={{ duration: 1.2, ease: [0.22, 1, 0.36, 1] }}
        style={{ filter: `drop-shadow(0 0 6px ${color})` }}
      />
    </svg>
  );
}
