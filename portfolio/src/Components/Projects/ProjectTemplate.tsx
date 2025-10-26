import { Flip } from "gsap/all";
import gsap from "gsap";
import { useEffect, useRef, useState, type ReactNode } from "react";
import ClosingButton from "./ClosingButton";
import "@/Styles/ProjectTemplate.css";

const classState = {
  container: {
    off: ["relative"],
    on: ["fixed"],
  },
  panel: {
    off: ["w-full", "h-full", "top-0", "left-0"],
    on: [
      "w-[calc(100%-1rem)]",
      "md:w-[calc(100%-2rem)]",
      "h-[calc(100%-1rem)]",
      "md:h-[max(40rem,calc(100%-2rem))]",
      "border-2",
      "md:top-4",
      "top-2",
      "left-1/2",
    ],
  },
  compact: {
    off: ["w-full"],
    on: ["w-full", "md:w-[clamp(20rem,30%,40rem)]", "md:rounded-tr-none"],
  },
  detail: {
    off: ["w-full", "md:w-0", "h-0", "md:h-full"],
    on: ["w-full", "flex-1", "md:h-full", "md:rounded-tl-none"],
  },
  illustration: {
    off: ["h-60"],
    on: ["h-[clamp(15rem,40%,25rem)]"],
  },
};

const turn = (
  element: HTMLElement | Element,
  stateName: "container" | "panel" | "compact" | "detail" | "illustration",
  on: boolean,
) => {
  const state = classState[stateName];
  element.classList.remove(...state[on ? "off" : "on"]);
  element.classList.add(...state[on ? "on" : "off"]);
};

export interface ProjectProps {
  active: boolean;
  onClick: (active: boolean) => void;
}

interface ProjectTemplateProps {
  active: boolean;
  onClick: (active: boolean) => void;
  className?: string;
  illustration: ReactNode;
  description: ReactNode;
  details: ReactNode;
}

export default function ProjectTemplate({
  active,
  onClick,
  className,
  illustration,
  description,
  details,
}: ProjectTemplateProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    const compact = container?.querySelector(".compact-view");
    const panel = container?.querySelector(".panel");
    const detail = container?.querySelector(".detail-view");
    const illustration = container?.querySelector(".illustration-view");

    if (!container || !compact || !detail || !illustration || !panel) return;

    const state = Flip.getState([
      container,
      compact,
      panel,
      illustration,
      detail,
    ]);
    const duration = 0.5;

    if (active) {
      gsap.set(container, {
        zIndex: 50,
      });

      gsap.set(panel, {
        transform: "translateX(-50%)",
        overflowY: "scroll",
      });

      turn(container, "container", true);
      turn(compact, "compact", true);
      turn(panel, "panel", true);
      turn(detail, "detail", true);
      turn(illustration, "illustration", true);

      Flip.from(state, {
        duration,
        ease: "power2.inOut",
      });
    } else {
      gsap.set(panel, {
        transform: "translateX(0px)",
        overflowY: "hidden",
        scrollTo: 0,
      });

      turn(container, "container", false);
      turn(compact, "compact", false);
      turn(panel, "panel", false);
      turn(detail, "detail", false);
      turn(illustration, "illustration", false);

      Flip.from(state, {
        duration,
        ease: "power2.inOut",
        onComplete: () => {
          gsap.set(container, {
            zIndex: "auto",
          });
        },
      });
    }
  }, [active]);

  return (
    <div className={className ?? "h-130"}>
      <div
        className="relative flex flex-col w-full h-full top-0 left-0 bg-none"
        ref={containerRef}
      >
        <i
          className="absolute w-full h-full cursor-not-allowed"
          onClick={() => onClick(!active)}
        />
        <div
          className="panel absolute flex flex-col left-0 top-0 w-full h-full rounded-2xl scrollbar-custom bg-white-chalk scroll-thumb-black overflow-hidden"
          onClick={() => !active && onClick(true)}
        >
          <div className="illustration-view relative w-full shrink-0 h-60">
            <i className="absolute left-0 top-0 w-full" />
            <div style={{ position: "relative", height: "calc(100% + 1rem)" }}>
              {illustration}
            </div>
          </div>
          <div className="flex flex-col md:flex-row relative w-full max-h-fit">
            <div className="compact-view relative rounded-t-xl w-full h-fit bg-white-chalk">
              {description}
            </div>
            <div className="detail-view relative rounded-t-xl w-0 h-0 md:h-auto overflow-hidden">
              {details}
            </div>
          </div>
        </div>

        <ClosingButton
          className="absolute right-6 top-4 md:right-10 md:top-8 z-12"
          onClick={() => active && onClick(false)}
          active={active}
          width={40}
          height={40}
        />
      </div>
    </div>
  );
}
