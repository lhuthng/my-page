import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";

export default function Limpext({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={<div></div>}
      description={
        <div className="flex flex-col justify-center w-full h-full space-y-4 p-4 text-black">
          <h1>Client Web Development & SEO</h1>
          <p>
            <strong>Brief:</strong> Developed custom, high-performance landing
            pages using Svelte 5 and Tailwind CSS, featuring dynamic
            localization and deployment support.
          </p>
          <p>
            <strong>Technology:</strong> Svelte 5, HTML5, CSS3, JavaScript,
            Tailwind CSS, GSAP, PHP, Apache, SEO
          </p>
        </div>
      }
      details={<div></div>}
    />
  );
}
