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
          <h1>3D Data Pipeline Modernization</h1>
          <p>
            Modernized high-complexity 3D data processing pipelines by migrating
            legacy MATLAB codebases to high-performance Python environments,
            integrating automated testing and custom file parsing.
          </p>
          <p>
            <strong>Technologies:</strong> Python (NumPy, SciPy, Pandas,
            Pytest), MATLAB
          </p>
        </div>
      }
      details={
        <div className="w-full h-full bg-white border-mpg-green text-black space-y-4 md:rounded-tr-xl not-md:rounded-bl-xl p-4 border-2 box-border">
          <h1>
            <strong>
              Scientific Code Optimization: From Visualization to Performance
              Engineering
            </strong>
          </h1>
          <p>
            My role at the Max-Planck-Institut für Biophysik began with GUI
            development for scientific visualization. However, I soon pivoted to
            address a critical departmental bottleneck: a slow, unverified
            legacy MATLAB processing pipeline that was limiting research
            velocity.
          </p>
          <h1>
            <strong>
              Project Focus: 3D Pipeline Migration &amp; Optimization (MATLAB to
              Python)
            </strong>
          </h1>
          <p>
            I executed the technical migration of core 3D data processing
            workflows to a modern, scalable Python ecosystem.
          </p>
          <ul className="list-disc list-inside space-y-2">
            <li>
              <strong>System Migration:</strong> Ported complex mathematical
              data processing logic from legacy MATLAB scripts to a
              high-performance Python environment, ensuring 1:1 functional
              parity.
            </li>
            <li>
              <strong>Mathematical Optimization:</strong> Rewrote core
              computational steps using <strong>NumPy and Pandas</strong>,
              significantly accelerating processing speeds and enabling
              real-time data iteration for researchers.
            </li>
            <li>
              <strong>Specialized File Parsers:</strong> Engineered custom
              Python parsers for specialized data extensions, automating data
              ingestion and reducing manual preparation time by 50%.
            </li>
            <li>
              <strong>Quality Assurance:</strong> Established code reliability
              by implementing a comprehensive automated testing suite using{" "}
              <strong>pytest</strong>
              to validate scientific data integrity.
            </li>
          </ul>
        </div>
      }
    />
  );
}
