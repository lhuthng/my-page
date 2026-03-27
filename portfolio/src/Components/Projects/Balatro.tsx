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
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-white-chalk text-black border-2 border-black">
          <h1>Game Reverse Engineering & Localization</h1>
          <p>
            Developed a full Vietnamese localization for the roguelike Balatro.
            This involved reverse-engineering Lua-based game logic, engineering
            custom pixel-font glyphs, and building automated regex-based
            extraction tools.
          </p>
          <p>
            <strong>Technologies:</strong> Lua (Löve Engine), Bash, FontForge,
            JavaScript, Regex
          </p>
        </div>
      }
      details={
        <div className="bg-orange-chalk w-full h-full text-black overflow-hidden p-4 space-y-4 border-2 border-black">
          <p>
            As a fan of Balatro, I initiated this project to bring the game to
            the Vietnamese community. By reverse-engineering the{" "}
            <strong>Löve (Lua) engine</strong>, I gained access to the game's
            source code, allowing me to hook into the rendering pipeline and
            modify internal data structures.
            <br />
            <strong className="block mt-2">
              Interaction: You can drag the red strip on the video above to
              toggle between the English and Vietnamese versions in real-time.
            </strong>
          </p>
          <p>
            <strong>The Glyph Challenge:</strong> Balatro uses a specialized
            pixel-art typeface. To support Vietnamese diacritics while
            preserving the game's aesthetic, I used <strong>FontForge</strong>{" "}
            and sprite-sheet manipulation to manually engineer missing glyphs
            into the game's font atlas, ensuring perfect alignment with the
            original pixel grid.
          </p>
          <p>
            <strong>Automation Tooling:</strong> To manage thousands of strings
            across complex Lua files, I developed a{" "}
            <strong>custom web utility (HTML/JS)</strong>. This tool used{" "}
            <strong>Regex</strong> to safely isolate translatable text from
            executable code, preventing syntax errors during injection. I also
            authored
            <strong>Bash scripts</strong> to automate the extraction and
            repacking of game resources (the <code>.love</code> archive).
          </p>
          <div className="flex items-center gap-4 pt-2">
            <a
              href="https://github.com/lhuthng/balatro-vi-localization"
              target="_blank"
              rel="noopener noreferrer"
            >
              <i
                className="inline-block w-8 h-8 hover:scale-110 transition-transform"
                style={{
                  backgroundColor: "black",
                  maskImage: `url(${github})`,
                  WebkitMaskImage: `url(${github})`,
                  maskRepeat: "no-repeat",
                  maskPosition: "center",
                }}
              />
            </a>
            <p>
              Read the full technical breakdown on my blog:{" "}
              <a
                className="font-bold text-blue-700 hover:underline"
                target="_blank"
                href="https://blog.huuthangle.site/posts/i-added-vietnamese-localization-to-balatro"
              >
                huuthangle.site
              </a>
            </p>
          </div>
        </div>
      }
    />
  );
}
