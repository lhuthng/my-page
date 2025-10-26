import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";

export default function StyledPortfolio({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={<div></div>}
      description={
        <div className="flex flex-col justify-center w-full h-full space-y-4 p-4 text-black">
          <h1>Digital Creative Portfolio</h1>
          <p>
            <strong>Brief:</strong> Personal portfolio built with React focused
            on digital arts, 3D modeling, and music production, styled with a
            custom, hand-drawn aesthetic.
          </p>
          <p>
            <strong>Technology:</strong> React, HTML5, CSS3, JavaScript
          </p>
        </div>
      }
      details={<div></div>}
    />
  );
}
