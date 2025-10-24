import CoolHeader from "./CoolHeader";
import React, { useEffect, useRef, useState, type ReactNode } from "react";
import gsap from "gsap";
import { Flip } from "gsap/all";
import ThisPortfolio from "./Projects/ThisPortfolio";
import useDebounce from "./Hooks/debounce";
import Balatro from "./Projects/Balatro";

export default function Projects() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const projects = [Balatro];
  return (
    <section className="w-full p-4">
      <CoolHeader title="Projects" />
      <div className="w-full inset-au m-auto">
        <div className="grid grid-cols-1 md:grid-cols-[repeat(auto-fit,minmax(20rem,1fr))] grid-rows- gap-4 w-full ">
          {projects.map((ProjectComponent, index) => (
            <div className="project-container w-full h-120" key={index}>
              <ProjectComponent
                active={index === activeIndex}
                onClick={(active) => setActiveIndex(active ? index : null)}
              />
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
