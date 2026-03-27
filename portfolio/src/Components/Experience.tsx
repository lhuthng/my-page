import CoolHeader from "./CoolHeader";
import "@/Styles/Experience.css";
import WorkExperience from "./WorkExperience";
import UserSVG from "./UserSVG";
import { useEffect, useRef, useState } from "react";
import { useGSAP } from "@gsap/react";
import { Flip } from "gsap/Flip";
import gsap from "gsap";
import Education from "./Education";
import { ScrollTrigger } from "gsap/all";

export default function Experience() {
  const containerRef = useRef<HTMLDivElement>(null);
  const iRef = useRef<HTMLElement>(null);
  const leftRef = useRef<HTMLSpanElement>(null),
    rightRef = useRef<HTMLSpanElement>(null);
  const [tab, setTab] = useState<"work" | "education">("work");
  const workContainer = useRef<HTMLDivElement>(null);
  const educationContainer = useRef<HTMLDivElement>(null);
  const toEdu = useRef<gsap.core.Timeline>(null);

  const changeI = (
    ref: React.RefObject<HTMLElement | null>,
    leftPadding: number,
    widthPadding: number,
  ) => {
    if (!ref.current || !iRef.current) return;
    iRef.current.style.left = `${ref.current.offsetLeft - leftPadding}px`;
    iRef.current.style.width = `${ref.current.offsetWidth + leftPadding + widthPadding}px`;
  };

  const attach = (
    ref: React.RefObject<HTMLElement | null>,
    duration: number,
    leftPadding: number,
    widthPadding: number,
  ) => {
    if (!ref.current || !iRef.current) return;

    const state = Flip.getState(iRef.current);

    changeI(ref, leftPadding, widthPadding);

    Flip.from(state, {
      duration,
      ease: "power1.inOut",
    });
  };

  const changeTab = (tab: "work" | "education") => {
    switch (tab) {
      case "work": {
        attach(leftRef, 0.25, 6, 0);
        toEdu?.current?.reverse();
        break;
      }
      case "education": {
        attach(rightRef, 0.25, 0, -2);
        toEdu?.current?.play();
        break;
      }
    }
    setTab(tab);
  };

  useEffect(() => {
    changeI(tab === "work" ? leftRef : rightRef, 2, 0);
  }, []);

  useGSAP(() => {
    if (!educationContainer.current || !workContainer.current) return;

    gsap.set(workContainer.current, {
      opacity: 1,
      display: "block",
      pointerEvents: "all",
    });
    gsap.set(educationContainer.current, {
      opacity: 0,
      display: "none",
      pointerEvents: "none",
    });

    toEdu.current = gsap.timeline({ paused: true });

    toEdu.current.to(workContainer.current, {
      opacity: 0,
      display: "none",
      pointerEvents: "none",
      duration: 0.2,
      ease: "power1.in",
    });

    toEdu.current.to(
      educationContainer.current,
      {
        opacity: 1,
        display: "block",
        pointerEvents: "all",
        duration: 0.2,
        ease: "power1.out",
      },
      "-=0.2",
    );
  }, []);

  return (
    <section className="max-w-340 mx-auto pb-4 ">
      <div className="w-[calc(100%-4rem)] lg:w-auto max-w-300 mx-auto">
        <CoolHeader title="Experience & Education" />
        <div
          ref={containerRef}
          className="experience-selector grid grid-cols-2 gap-2 mx-auto w-60 py-10"
        >
          <UserSVG
            className="w-20 ml-auto cursor-pointer"
            fill={tab === "work" ? "white" : "none"}
            stroke={tab === "work" ? "none" : "white"}
            strokeWidth={tab === "work" ? 0 : 2}
            onClick={() => changeTab("work")}
          />
          <UserSVG
            className="w-20 -scale-x-100 cursor-pointer"
            fill={tab === "education" ? "white" : "none"}
            stroke={tab === "education" ? "none" : "white"}
            strokeWidth={tab === "education" ? 0 : 2}
            onClick={() => changeTab("education")}
          />
          <div
            className={`grid grid-cols-2 col-span-2 text-2xl select-none transition-transform duration-200 ${tab === "work" ? "skew-x-10" : "-skew-x-7"}`}
          >
            <i
              ref={iRef}
              className="content-[''] absolute top-0 bottom-0 backdrop-invert z-11 cursor-pointer"
            />
            <span
              ref={leftRef}
              className="relative ml-auto z-10 pr-1 cursor-pointer"
              onClick={() => changeTab("work")}
            >
              Work
            </span>
            <span
              ref={rightRef}
              className="relative pl-1 z-10 cursor-pointer"
              onClick={() => changeTab("education")}
            >
              Education
            </span>
          </div>
        </div>
        <div>
          <div ref={workContainer} className="lg:pb-50">
            <div className="grid-flex py-4 space-x-2">
              <WorkExperience
                title="Freelance Web Developer"
                duration="May 2025 - Dec 2025"
                organization="Limpext"
                technologies={["HTML/CSS/JS", "Svelte 5", "PHP", "Apache"]}
                project="Designed, developed, and launched custom, professional websites for Limpext."
                additional={{
                  externalDocument: [
                    {
                      name: "limpext.de",
                      url: "https://limpext.de",
                    },
                    {
                      name: "limmobilien.de",
                      url: "https://limmobilien.de/",
                    },
                  ],
                }}
              >
                <span>
                  <b>Frontend Architecture: </b> Developed SEO-optimized
                  frontend using Svelte and GSAP, implementing custom Leaflet
                  map integrations and full localization support.
                </span>
                <span>
                  <b>Backend Systems: </b> Architected backend workflows with
                  PHP to manage secure data handling and lead generation from
                  contact forms.
                </span>
                <span>
                  <b>Server Management: </b> Deployed and managed production
                  servers using Apache, including web routing and environment
                  configuration.
                </span>
              </WorkExperience>
              <WorkExperience
                title="Programmer Analyst"
                duration="Mar 2020 – Feb 2022"
                organization="DXC Technology Vietnam"
                location="Ho Chi Minh City"
                technologies={[
                  "React",
                  "TypeScript",
                  "JavaScript",
                  ".NET Framework",
                  "OracleDB",
                  "Task Scheduler",
                  "Azure DevOps",
                  "Git",
                ]}
                project="Engineered responsive UIs and optimized backend logic for enterprise e-commerce tools and internal sales platforms."
                additional={{
                  externalDocument: [
                    {
                      name: "Certificate of Employment",
                      url: "https://github.com/lhuthng/lhuthng/blob/main/letters/Certificate%20of%20Employment.pdf",
                    },
                  ],
                }}
              >
                <span>
                  <b>Frontend Engineering: </b> Engineered responsive UIs for
                  enterprise e-commerce tools using React, TypeScript, and
                  JavaScript.
                </span>
                <span>
                  <b>Full-stack Integration: </b> Integrated front-end services
                  with .NET Framework REST APIs and OracleDB backends.
                </span>
                <span>
                  <b>System Optimization: </b> Optimized system reliability by
                  debugging API logic and maintaining automated batch scripts
                  via Task Scheduler.
                </span>
                <span>
                  <b>DevOps & Collaboration: </b> Streamlined development
                  through Azure DevOps workflows, contributing to code reviews
                  and Git-based collaboration.
                </span>
              </WorkExperience>
              <WorkExperience
                title="Scientific Assistant"
                duration="Jul 2022 – Oct 2024"
                organization="Max-Planck-Institut für Biophysik"
                location="Frankfurt am Main"
                technologies={[
                  "Python",
                  "MATLAB",
                  "NumPy",
                  "SciPy",
                  "Pandas",
                  "Matplotlib",
                  "pytest",
                ]}
                project="Modernized 3D data processing pipelines and migrated legacy codebases to high-performance Python environments."
                additional={{
                  externalDocument: [
                    {
                      name: "cryoCAT",
                      url: "https://github.com/turonova/cryoCAT",
                    },
                    {
                      name: "gapstop_tm",
                      url: "https://gitlab.mpcdf.mpg.de/bturo/gapstop_tm",
                    },
                    {
                      name: "Recommendation Letter",
                      url: "https://github.com/lhuthng/lhuthng/blob/main/letters/HuuThangLe_recommendation.pdf",
                    },
                  ],
                }}
              >
                <span>
                  <b>Pipeline Modernization: </b> Modernized 3D data processing
                  pipelines by migrating legacy MATLAB codebases to Python,
                  optimizing mathematical data collection.
                </span>
                <span>
                  <b>Feature Engineering: </b> Engineered custom file parsers
                  for specialized data extensions and integrated new features
                  into high-complexity codebases.
                </span>
                <span>
                  <b>Automated Testing: </b> Ensured code reliability by
                  implementing comprehensive automated testing suites using
                  pytest.
                </span>
              </WorkExperience>
              <WorkExperience
                title="Teaching Assistant"
                duration="Mar 2021 – Oct 2021"
                organization="Frankfurt University of Applied Science"
                location="Frankfurt am Main"
                technologies={["Java"]}
                project="Guided master's students through a course on competitive programming, focusing on fundamental algorithms and problem-solving techniques."
              >
                <span>
                  <b>Mentorship & Instruction: </b>Mentored students on key data
                  structures and algorithms, including dynamic programming,
                  graph theory, and computational geometry, improving their
                  problem-solving skills.
                </span>
                <span>
                  <b>Technical Expertise: </b>Provided direct, real-time
                  feedback on code, identified algorithmic flaws, and suggested
                  optimizations, enhancing the students' coding efficiency and
                  correctness.
                </span>
              </WorkExperience>
            </div>
          </div>
          <div ref={educationContainer} className="hidden w-full ">
            <Education init={tab === "education"} />
          </div>
        </div>
      </div>
    </section>
  );
}
