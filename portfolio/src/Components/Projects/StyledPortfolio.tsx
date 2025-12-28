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
            Personal portfolio built with React focused on showing digital arts,
            3D modeling, and music production, styled with a custom, hand-drawn
            aesthetic.
          </p>
          <p>
            <strong>Technology:</strong> React, HTML5, CSS3, JavaScript
          </p>
        </div>
      }
      details={
        <div className="bg-purple-300 w-full h-full not-md:border-t-2 md:border-l-2 border-black text-black font-omori-2 text-2xl p-4">
          <p>
            This is a personal portfolio built with React to display my creative
            work: digital arts, 3D modeling, and music production.
          </p>

          <div>
            Key Features and Style
            <ul className="[&>li>strong]:font-extrabold">
              <li className="flex flex-col">
                <figure className="py-4">
                  <img
                    className="mx-auto rounded-xl overflow-hidden"
                    src={skills}
                    style={{
                      height: "auto",
                      width: "min(80%, 40rem)",
                    }}
                  />
                  <figcaption className="text-center">A snippet</figcaption>
                </figure>
                <strong>Custom Design:</strong> The entire UI is styled with a
                custom, hand-drawn, notebook aesthetic, directly inspired by the
                game Omori. I used custom CSS3 and unique assets to create this
                look.
              </li>
              <li>
                <strong>Art Gallery:</strong> The site uses React to manage the
                art gallery, ensuring a responsive and smooth experience for
                viewing all media types, including high-resolution images and
                embedded music.
              </li>
              <li>
                <strong>Technology:</strong> Built as a single-page application
                using React, HTML5, CSS3, and JavaScript.
              </li>
            </ul>
          </div>
        </div>
      }
    />
  );
}
