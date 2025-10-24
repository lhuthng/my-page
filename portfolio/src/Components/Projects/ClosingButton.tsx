import { useEffect, useRef } from "react";
import gsap from "gsap";

interface ClosingButtonProps {
  className?: string;
  width: number;
  height: number;
  active: boolean;
  onClick: () => void;
}

export default function ClosingButton({
  width,
  height,
  active,
  onClick,
  className,
}: ClosingButtonProps) {
  const svg = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (svg.current === null) {
      return;
    }

    const paths = svg.current.querySelectorAll("path");

    gsap.to(paths, {
      scale: active ? 1 : 0,
      duration: 0.5,
      x: active ? 0 : 4,
      y: active ? 0 : -4,
      transformOrigin: "center center",
    });
  }, [active]);

  return (
    <button
      className={className}
      onClick={onClick}
      style={{
        cursor: active ? "pointer" : "default",
      }}
    >
      <svg
        ref={svg}
        className="hover:brightness-110 hover:scale-105"
        width={width}
        height={height}
        viewBox="-3 -3 16 16"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M0 0 L10 10"
          stroke="gray"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          fill="none"
        />
        <path
          d="M10 0 L0 10"
          stroke="gray"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          fill="none"
        />
      </svg>
    </button>
  );
}
