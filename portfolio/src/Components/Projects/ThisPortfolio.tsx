import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";

export default function ThisPortfolio({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={<div></div>}
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-orange-chalk text-black">
          <h1>Personal Web Server</h1>
          <p>
            A VPS-based, full-stack environment that hosts this portfolio, a
            personal blog, backend services, and live development demos for
            clients. The entire platform features automated deployment via
            GitHub Actions (CI/CD). The architecture demonstrates advanced
            server management using Nginx as a reverse proxy alongside Apache2,
            and utilizes both React and Svelte with a Node.js backend.
          </p>
          <p>
            <strong>Technology:</strong> GitHub Actions (CI/CD), VPS, Nginx,
            Apache2, Node.js, React, Svelte, UI/UX Design, HTML5, CSS3,
            JavaScript
          </p>
        </div>
      }
      details={<div></div>}
    />
  );
}
