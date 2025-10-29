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
        <div className="w-full h-full bg-white border-mpg-green text-black space-y-4 md:rounded-tr-xl not-md:rounded-bl-xl p-2 border-2">
          <h1>
            Scientific Code Optimization: A Shift from GUI Development to
            Performance Engineering
          </h1>
          <p>
            My role at the Max-Planck-Institut für Biophysik began with GUI
            development for scientific visualization. However, I soon pivoted to
            address a critical departmental bottleneck: the slow, unsupported,
            and unverified legacy MATLAB processing pipeline. This operational
            friction was severely limiting research velocity.
          </p>
          <h1>
            Project Focus: Performance Overhaul and Migration (MATLAB to Python)
          </h1>
          <p>
            I led the effort to move all the old code to a new, faster system.
          </p>
          <ul className="list-disc list-inside space-y-2">
            <li>
              <strong>Move the Code:</strong> I took most of the complex data
              process and moved it from old MATLAB scripts to a faster Python
              system.
            </li>
            <li>
              <strong>Make it Faster:</strong> I rewrote the key math steps
              using powerful Python tools like NumPy and Pandas. This made the
              data processing much quicker, letting scientists test ideas right
              away.
            </li>
            <li>
              <strong>Custom Data Parser:</strong> I built a custom Python tool
              to read and write specialized data files with a unique extension.
              This made handling these files much faster, which cut the time
              scientists spent on preparing data by half.
            </li>
          </ul>
        </div>
      }
    />
  );
}
