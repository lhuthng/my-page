import CoolHeader from "./CoolHeader";
import { useState } from "react";
import ThisPortfolio from "./Projects/ThisPortfolio";
import Balatro from "./Projects/Balatro";
import DXC from "./Projects/DXC";
import MaxPlanck from "./Projects/MaxPlanck";
import Limpext from "./Projects/Limpext";
import StyledPortfolio from "./Projects/StyledPortfolio";

export default function Projects() {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const projects = [
    DXC,
    MaxPlanck,
    Limpext,
    Balatro,
    StyledPortfolio,
    ThisPortfolio,
  ];
  return (
    <section className="w-full p-4">
      <CoolHeader title="Projects" />
      <div className="w-full inset-au max-w-340 m-auto">
        <ul className="grid grid-cols-1 md:grid-cols-[repeat(auto-fit,minmax(20rem,1fr))] grid-rows- gap-4 w-full ">
          {projects.map((ProjectComponent, index) => (
            <li className="project-container w-full" key={index}>
              <ProjectComponent
                active={index === activeIndex}
                onClick={(active) => setActiveIndex(active ? index : null)}
              />
            </li>
          ))}
          <li className="col-span-full w-full flex items-center justify-center bg-white rounded-2xl text-black p-4">
            <p>
              More projects can be found on{" "}
              <a
                className="inline-block w-16 hover:font-semibold text-blue-500"
                href="https://blog.huuthang.site"
              >
                my blog
              </a>
            </p>
          </li>
        </ul>
      </div>
    </section>
  );
}
