import { useEffect, useState } from "react";
import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";
import avatar from "@/Assets/Images/avatar.webp";

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
                filter: `url(#squiggly-${filterId})`,
              }}
            />
          </div>
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-purple-300 text-black font-omori-2 text-2xl">
          <h1 className="font-omori-1 text-3xl!">Digital Creative Portfolio</h1>
          <p>
            Personal portfolio built with React focused on showing digital arts,
            3D modeling, and music production, styled with a custom, hand-drawn
            aesthetic.
          </p>
          <p>
            <strong>Technology:</strong> React, HTML5, CSS3, JavaScript
          </p>
        </div>
      }
      details={<div className="bg-white w-full h-full"></div>}
    />
  );
}
