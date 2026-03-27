import { useEffect, useState } from "react";
import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";
import avatar from "@/Assets/Images/avatar.webp";
import skills from "@/Assets/Images/skills-demo.webp";

export default function StyledPortfolio({ active, onClick }: ProjectProps) {
  const [filterId, setFilterId] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setFilterId((prev) => (prev + Math.floor(Math.random() * 4) + 1) % 5);
    }, 200);

    return () => {
      clearInterval(interval);
    };
  }, []);

  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div
          className="flex justify-center items-center w-full h-full"
          style={{
            backgroundColor: "rgba(230, 213, 255)",
          }}
        >
          <div
            className="relative p-4 rounded-lg"
            style={{
              backgroundColor: "hsl(52 100% 91%)",
            }}
          >
            <div
              className="absolute top-0 left-0"
              style={{
                borderWidth: "1.5rem",
                borderStyle: "solid",
                borderColor:
                  "rgba(230, 213, 255) hsl(52 80% 70%) hsl(52 80% 70%) rgba(230, 213, 255)",
                borderBottomRightRadius: "1rem",
                boxShadow:
                  "0 1px 1px rgba(0,0,0,0.3),1px 1px 1px rgba(0,0,0,0.2)",
              }}
            />
            <img
              className="bg-contain bg-no-repeat  "
              src={avatar}
              style={{
                filter: `url(#squiggly-${filterId}) drop-shadow(0px 10px 2px rgba(0, 0, 0, 0.6))`,
              }}
            />
          </div>
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-purple-300 text-black font-omori-2 text-2xl">
          <h1 className="font-omori-1 text-3xl!">Digital Creative Portfolio</h1>
          <p>
            A highly stylized React-based showcase for my 3D modeling, digital
            arts, and music production, featuring a custom hand-drawn "notebook"
            UI.
          </p>
          <p>
            <strong>Stack:</strong> React, Custom CSS3, GSAP, Multimedia APIs
          </p>
        </div>
      }
      details={
        <div className="bg-purple-300 w-full h-full not-md:border-t-2 md:border-l-2  text-black font-omori-2 text-2xl p-4 space-y-4">
          <p>
            This project serves as a curated digital gallery for my creative
            outputs outside of traditional software engineering—bridging the gap
            between technical development and artistic production.
          </p>

          <div className="space-y-4">
            <h2 className="font-omori-1 text-3xl">Artistic Features & Style</h2>
            <ul className="space-y-4 [&>li>strong]:font-extrabold">
              <li className="flex flex-col">
                <figure className="py-2">
                  <img
                    className="mx-auto rounded-xl border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]"
                    src={skills}
                    style={{
                      height: "auto",
                      width: "min(90%, 40rem)",
                    }}
                    alt="Hand-drawn skills interface snippet"
                  />
                  <figcaption className="text-center text-sm mt-2 font-bold italic">
                    Custom "Omori-style" Skill Tree Interface
                  </figcaption>
                </figure>
                <p className="mt-2">
                  <strong>Immersive Hand-Drawn UI:</strong> Developed a unique
                  "notebook" aesthetic inspired by <em>Omori</em>. The entire
                  interface is built from custom-sketched assets and hand-coded
                  CSS3 borders, breaking away from standard "flat" design to
                  create a tactile, organic user experience.
                </p>
              </li>
              <li>
                <strong>Multimedia Gallery:</strong> Engineered a custom media
                handler in React to manage high-fidelity 3D renders, digital
                illustrations, and embedded audio tracks with synchronized UI
                feedback.
              </li>
              <li>
                <strong>Thematic Consistency:</strong> Integrated specialized
                typography and custom SVG-masked filters to ensure all
                components—from buttons to cards—remain consistent with the
                hand-drawn theme.
              </li>
            </ul>
          </div>
        </div>
      }
    />
  );
}
