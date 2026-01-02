import { useEffect, useRef, useState, Activity, useLayoutEffect } from "react";
import gsap from "gsap";
import ProjectTemplate, { type ProjectProps } from "./ProjectTemplate";
import balatroDemo from "@/Assets/Images/balatro.webp";
import viSrc from "@/Assets/Videos/vi-balatro.webm";
import enSrc from "@/Assets/Videos/en-balatro.webm";
import { Draggable } from "gsap/Draggable";
import github from "@/Assets/SVGs/github.svg";

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
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-white-chalk text-black">
          <h1>Balatro Localization</h1>
          <p>
            Full Vietnamese localization for the popular Poker-builder game
            Balatro, including custom font support and cultural adjustments.
          </p>
          <p>
            <strong>Technologies:</strong> Lua, Bash, Custom Localization Tool
            (HTML/JS)
          </p>
        </div>
      }
      details={
        <div className="bg-orange-chalk w-full h-full text-black overflow-hidden p-4 space-y-4">
          <p>
            As a passionate player of Balatro, I initiated a full Vietnamese
            localization project after discovering the game was built on the
            highly accessible <strong>Löve (Lua) engine</strong>). This required
            me to reverse-engineer the game's source code, gaining a deep
            understanding of its internal logic, data handling, and rendering
            pipelines to correctly inject custom translation and logic hooks.{" "}
            <strong>
              You can interact with the red strip on the video above to see the
              translation in action - it's draggable!
            </strong>
          </p>
          <p>
            To support the Vietnamese language's complex character set, I
            performed essential font engineering. I used{" "}
            <strong>FontForge</strong> to modify and extend the game's typeface,
            ensuring proper rendering of all diacritics. The project also
            included the development of technical tooling, such as a Bash script
            to reliably locate and inject the custom fonts and localization
            files into the game directory.
          </p>
          <p>
            Finally, I built a custom, user-friendly localization tool using{" "}
            <strong>HTML/JavaScript</strong>. This utility streamlined the
            process of reading, modifying, and managing the translation data,
            significantly improving the iteration speed and accuracy of the
            overall localization effort.
          </p>
          <p>
            View the full installation instructions here:{" "}
            <a
              href="https://github.com/lhuthng/balatro-vi-localization"
              target="_blank"
            >
              <i
                className="inline-block w-8 h-8 translate-y-2 hover:scale-105"
                style={{
                  backgroundColor: "black",
                  maskImage: `url(${github})`,
                }}
              />
            </a>
          </p>
          <p>
            For an in-depth write-up on the technical challenges, you can read
            more on my blog{" "}
            <a
              className="font-bold text-blue-400 hover:brightness-110"
              target="_blank"
              href="https://blog.huuthang.site/posts/i-added-vietnamese-localization-to-balatro"
            >
              here
            </a>
            .
          </p>
        </div>
      }
    />
  );
}
