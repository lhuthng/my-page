import ProjectTemplate, { type ProjectProps } from "./ProjectTemplate";

export default function DXC({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={<div></div>}
      description={
        <div className="flex flex-col justify-center w-full h-full space-y-4 p-4 text-black">
          <h1>E-Commerce Platform Maintenance & Tooling</h1>
          <p>
            <strong>Brief:</strong> Support for a large-scale, legacy e-commerce
            system, including critical transaction bug resolution and the
            creation of internal API testing tools.
          </p>
          <p>
            <strong>Technology:</strong> AngularJS, CSS, JavaScript, .NET
            Framework (WebAPI), C#, MongoDB, IIS, Windows Server
          </p>
        </div>
      }
      details={<div></div>}
    />
  );
}
