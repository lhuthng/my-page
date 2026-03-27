import ProjectTemplate, { type ProjectProps } from "./ProjectTemplate";
import dxc from "@/Assets/Images/dxc.webp";

export default function DXC({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div className="flex justify-center items-center w-full h-full bg-white-chalk">
          <div
            className="w-[calc(100%-2rem)] h-[calc(100%-4rem)] bg-center bg-no-repeat bg-contain"
            style={{
              backgroundImage: `url(${dxc})`,
            }}
          />
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-deep-violet text-white-chalk">
          <h1>Enterprise E-Commerce Engineering &amp; Modernization</h1>
          <p>
            Engineered responsive UIs and optimized backend logic for enterprise
            e-commerce platforms, focusing on high-scale stability, REST API
            integrations, and automated regional system migrations.
          </p>
          <p>
            <strong>Technologies:</strong> React, TypeScript, JavaScript, .NET
            Framework, OracleDB, IIS, Task Scheduler, Azure DevOps
          </p>
        </div>
      }
      details={
        <div className="w-full h-full bg-white text-black space-y-4 border-deep-violet md:rounded-tr-xl not-md:rounded-bl-xl p-4 box-border border-2">
          <h1>
            <strong>Career Launch: From Intern to Programmer Analyst</strong>
          </h1>
          <p>
            My time at DXC began as a compulsory university internship. My
            performance led to a full-time Programmer Analyst offer upon
            completion, immediately solidifying my role in supporting
            large-scale enterprise applications across multiple business units.
          </p>

          <div className="flex not-sm:flex-col gap-4 [&>div]:space-y-4">
            <div>
              <h1>
                <strong>
                  Project 1: Enterprise E-Commerce & Internal Tooling (Baker &
                  Taylor)
                </strong>
              </h1>
              <p>
                I engineered high-performance front-end solutions and optimized
                data handling for a high-volume e-commerce ecosystem.
              </p>
              <ul className="list-disc list-inside">
                <li>
                  <strong>Modern UI Engineering: </strong>Developed responsive
                  user interfaces and internal tools using{" "}
                  <strong>React and TypeScript</strong>, ensuring a type-safe
                  and scalable codebase.
                </li>
                <li>
                  <strong>API & Data Integration: </strong>Integrated front-end
                  services with .NET Framework REST APIs and OracleDB backends
                  to maintain data integrity across the platform.
                </li>
                <li>
                  <strong>System Reliability: </strong>Optimized application
                  stability by debugging complex API logic and managing
                  automated batch scripts via Task Scheduler.
                </li>
              </ul>
            </div>
            <div>
              <h1>
                <strong>
                  Project 2: Internal System Regional Migration (Benefits &
                  Claims)
                </strong>
              </h1>
              <p>
                I assisted in migrating a critical internal benefits and claims
                system between regions to accommodate local culture and benefit
                requirements. My team focused on the benefits and claims
                modules.
              </p>
              <ul className="list-disc list-inside">
                <li>
                  <strong>Migration Focus: </strong>Managed the migration of the
                  benefits and claims system, tailoring features and logic to
                  align with the new region's cultural and benefit structures.
                </li>
                <li>
                  <strong>Technical Resolution: </strong>Resolved tasks
                  including extensive system testing, modifying APIs, updating
                  database stored procedures, and adjusting batch scripts to
                  handle regional processing and data rules.
                </li>
              </ul>
            </div>
          </div>
        </div>
      }
    />
  );
}
