import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";

import mpi from "@/Assets/SVGs/mpi.svg";

export default function MaxPlanck({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div className="flex justify-center items-center w-full p-2 h-full bg-white">
          <img className="w-full h-full max-w-120 bg-contain" src={mpi} />
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full bg-mpg-green space-y-4 p-4 text-white-chalk">
          <h1>Scientific Code Optimization</h1>
          <p>
            Performance overhaul of complex scientific processing, migrating
            legacy MATLAB scripts to a faster, fully tested Python environment.
          </p>
          <p>
            <strong>Technologies:</strong> Python (NumPy, SciPy, Pandas,
            Pytest), MATLAB
          </p>
        </div>
      }
      details={
        <div className="w-full h-full bg-white border-mpg-green md:rounded-tr-xl not-md:rounded-bl-xl p-2 border-2">
          Yo
        </div>
      }
    />
  );
}
