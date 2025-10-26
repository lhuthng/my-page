import { useEffect, useRef, useState, Activity, useLayoutEffect } from "react";
import gsap from "gsap";
import ProjectTemplate, { type ProjectProps } from "./ProjectTemplate";
import balatroDemo from "@/Assets/Images/balatro.webp";
import viSrc from "@/Assets/Videos/vi-balatro.webm";
import enSrc from "@/Assets/Videos/en-balatro.webm";
import { Draggable } from "gsap/Draggable";

export default function Balatro({ active, onClick }: ProjectProps) {
  const container = useRef<HTMLDivElement>(null);
  const illustration = useRef<HTMLDivElement>(null);
  const separator = useRef<HTMLDivElement>(null);
  const overlay = useRef<HTMLVideoElement>(null);
  const clipPercent = useRef<number>(0.5);
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
    let fixSeparator: (() => void) | undefined;

    if (active) {
      gsap.set(separator.current, { x: "0px", left: "50%" });
      clipPercent.current = 0.5;
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
          const rawX = this.x + rect.width / 2;

          const rawPercent = rawX / rect.width;
          const clampedPercent = Math.max(0, Math.min(1, rawPercent)) * 100;

          overlay.current.style.setProperty(
            "--clip-percent",
            `${clampedPercent}%`,
          );
          clipPercent.current = rawPercent;
        },
      });

      fixSeparator = () => {
        if (!container.current || !overlay.current) {
          return;
        }

        const rect = container.current.getBoundingClientRect();
        gsap.set(separator.current, {
          x: clipPercent.current * rect.width - rect.width / 2,
        });
      };
    }

    if (fixSeparator) window.addEventListener("resize", fixSeparator);

    return () => {
      dragInstances.forEach((instance) => instance.kill());
      if (fixSeparator) window.removeEventListener("resize", fixSeparator);

      gsap.to(overlay.current, {
        opacity: 0,
        duration: 0.5,
      });

      gsap.to(separator.current, {
        opacity: 0,
        duration: 0.5,
      });
    };
  }, [vi, en, active]);

  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      className="h-160 3xs:h-146 2xs:h-120"
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
                  objectPosition: "left 50% top 0%",
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
                  objectPosition: "left 50% top 0%",
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
                className="absolute w-1 h-full bg-red-500 left-1/2 opacity-0"
              />
            </div>
          </Activity>
        </div>
      }
      description={
        <div className="flex flex-col justify-center w-full h-full space-y-4 p-4 text-black">
          <h1>Balatro Localization</h1>
          <p>
            <strong>Brief:</strong> Full Vietnamese localization for the popular
            Poker-builder game Balatro, including custom font support and
            cultural adjustments.
          </p>
          <p>
            <strong>Technology:</strong> Lua, Bash, Custom Localization Tool
            (HTML/JS)
          </p>
        </div>
      }
      details={
        <div className="bg-yellow-500 p-4 w-full overflow-hidden">
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
          <p>Test</p>
        </div>
      }
    />
  );
}
