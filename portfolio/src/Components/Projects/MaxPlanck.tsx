import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";

export default function MaxPlanck({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={<div></div>}
      description={
        <div className="flex flex-col justify-center w-full h-full space-y-4 p-4 text-black">
          <h1>Scientific Code Optimization</h1>
          <p>
            <strong>Brief:</strong> Performance overhaul of complex scientific
            processing, migrating legacy MATLAB scripts to a faster, fully
            tested Python environment.
          </p>
          <p>
            <strong>Technology:</strong> Python (NumPy, SciPy, Pandas, Pytest),
            MATLAB
          </p>
        </div>
      }
      details={<div></div>}
    />
  );
}
