import { useEffect, useRef, useState, Activity, useLayoutEffect } from "react";
import gsap from "gsap";
import ProjectTemplate from "./ProjectTemplate";
import balatroDemo from "@/Assets/Images/balatro.webp";
import viSrc from "@/Assets/Videos/vi-balatro.mp4";
import enSrc from "@/Assets/Videos/en-balatro.mp4";
import { Draggable } from "gsap/Draggable";

interface BalatroProps {
  active: boolean;
  onClick: (active: boolean) => void;
}

export default function Balatro({ active, onClick }: BalatroProps) {
  const container = useRef<HTMLDivElement>(null);
  const illustration = useRef<HTMLDivElement>(null);
  const separator = useRef<HTMLDivElement>(null);
  const overlay = useRef<HTMLVideoElement>(null);
  const [vi, setVi] = useState<string | undefined>();
  const [en, setEn] = useState<string | undefined>();

  useEffect(() => {
    if (!illustration.current) {
      return;
    }

    const onScrolled = gsap.to(illustration.current, {
      scrollTrigger: {
        trigger: illustration.current,
        start: "top bottom",
        once: true,
        onEnter: () => {
          setVi(viSrc);
          setEn(enSrc);
        },
      },
    });

    return () => {
      onScrolled.kill();
    };
  }, [illustration]);

  useLayoutEffect(() => {
    if (!separator.current || !overlay.current) {
      return;
    }

    let dragInstances: globalThis.Draggable[] = [];

    if (active) {
      gsap.set(separator.current, { x: "0px", left: "50%" });
      gsap.to([overlay.current, separator.current], {
        opacity: 1,
        duration: 0.5,
      });
      overlay.current.style.setProperty("--clip-percent", "50%");

      dragInstances = Draggable.create(separator.current, {
        type: "x",
        bounds: container.current,
        onDrag: function () {
          if (!overlay.current || !container.current) {
            return;
          }

          const rect = container.current.getBoundingClientRect();
          const rawX = this.x + rect.width / 2; // Position from left edge

          const containerWidth = rect.width;

          const rawPercent = (rawX / containerWidth) * 100;
          const clampedPercent = Math.max(0, Math.min(100, rawPercent));

          overlay.current.style.setProperty(
            "--clip-percent",
            `${clampedPercent}%`,
          );
        },
      });
    }

    return () => {
      dragInstances.forEach((instance) => instance.kill());

      gsap.to(overlay.current, {
        opacity: 0,
        duration: 0.1,
      });

      gsap.to(separator.current, {
        opacity: 0,
        duration: 0.1,
      });
    };
  }, [vi, en, active]);

  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div
          ref={illustration}
          className="w-full h-full transition-all duration-1000"
          style={{
            backgroundImage: `url(${balatroDemo})`,
            backgroundPosition: "center top",
            backgroundSize: "cover",
          }}
        >
          <Activity mode={vi && en ? "visible" : "hidden"}>
            <div ref={container} className="absolute inset-0 w-full h-full">
              <video
                className="absolute inset-0 w-full h-full"
                style={{
                  objectFit: "cover",
                  objectPosition: "left 50% top -1rem",
                }}
                src={vi}
                autoPlay
                loop
                muted
                playsInline
              />
              <video
                ref={overlay}
                className="absolute inset-0 w-full h-full opacity-0"
                style={{
                  objectFit: "cover",
                  objectPosition: "left 50% top -1rem",
                  clipPath:
                    "polygon(0% 0%, var(--clip-percent) 0%, var(--clip-percent) 100%, 0% 100%)",
                }}
                src={en}
                autoPlay
                loop
                muted
                playsInline
              />
              <div
                ref={separator}
                className="absolute w-1 h-full bg-red-500 left-1/2 z-10 opacity-0"
              />
            </div>
          </Activity>
        </div>
      }
      description={
        <div className="flex w-full h-full items-center text-black">
          <h1 className="mx-auto">Balatro Localization</h1>
        </div>
      }
      details={<div className="bg-yellow-500 w-full h-full"></div>}
    />
  );
}
