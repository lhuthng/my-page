import CoolHeader from "./CoolHeader";
import '@/Styles/Experience.css';
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
    const leftRef = useRef<HTMLSpanElement>(null), rightRef = useRef<HTMLSpanElement>(null);
    const [ tab, setTab ] = useState<"work" | "education">("work");
    const workContainer = useRef<HTMLDivElement>(null);
    const educationContainer = useRef<HTMLDivElement>(null);
    const toEdu = useRef<gsap.core.Timeline>(null);

    const changeI = (ref: React.RefObject<HTMLElement | null>, leftPadding: number, widthPadding: number) => {
        if (!ref.current || !iRef.current) return;
        iRef.current.style.left = `${ref.current.offsetLeft - leftPadding}px`;
        iRef.current.style.width = `${ref.current.offsetWidth + leftPadding + widthPadding}px`;
    }

    const attach = (ref: React.RefObject<HTMLElement | null>, duration: number, leftPadding: number, widthPadding: number) => {
        if (!ref.current || !iRef.current) return;

        const state = Flip.getState(iRef.current);

        changeI(ref, leftPadding, widthPadding);

        Flip.from(state, {
            duration,
            ease: "power1.inOut",
        });
    }

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
    }

    useEffect(() => {
        changeI(tab === "work" ? leftRef : rightRef, 2, 0);
    }, []);


    useGSAP(() => {
        if (!educationContainer.current || !workContainer.current) return;

        gsap.set(workContainer.current, {
            opacity: 1,
            display: "block",
            pointerEvents: "all"
        });
        gsap.set(educationContainer.current, {
            opacity: 0,
            display: "none",
            pointerEvents: "none"
        });

        toEdu.current = gsap.timeline({ paused: true });
    
        toEdu.current.to(workContainer.current, {
            opacity: 0,
            display: "none",
            pointerEvents: "none",
            duration: 0.2,
            ease: "power1.in"
        });

        toEdu.current.to(educationContainer.current, {
            opacity: 1,
            display: "block",
            pointerEvents: "all",
            duration: 0.2,
            ease: "power1.out"
        }, "-=0.2");
    }, []);


    return (<section className="max-w-340 mx-auto pb-4 ">
        <div className="w-[calc(100%-4rem)] lg:w-auto max-w-300 mx-auto">
            <CoolHeader title="Experience & Education" />
            <div ref={containerRef} className="experience-selector grid grid-cols-2 gap-2 mx-auto w-60 py-10">
                <UserSVG className="w-20 ml-auto cursor-pointer" 
                    fill={tab === "work" ? "white" : "none"}
                    stroke={tab === "work" ? "none" : "white"}
                    strokeWidth={tab === "work" ? 0 : 2}
                    onClick={() => changeTab("work")}
                />
                <UserSVG className="w-20 -scale-x-100 cursor-pointer" 
                    fill={tab === "education" ? "white" : "none"}
                    stroke={tab === "education" ? "none" : "white"}
                    strokeWidth={tab === "education" ? 0 : 2}
                    onClick={() => changeTab("education")}
                />
                <div className={`grid grid-cols-2 col-span-2 text-2xl select-none transition-transform duration-200 ${tab === "work" ? "skew-x-10" : "-skew-x-7"}`}>
                    <i ref={iRef} className="content-[''] absolute top-0 bottom-0 backdrop-invert z-11 cursor-pointer"/>
                    <span ref={leftRef} className="relative ml-auto z-10 pr-1 cursor-pointer" onClick={() => changeTab("work")}>Work</span>
                    <span ref={rightRef} className="relative pl-1 z-10 cursor-pointer" onClick={() => changeTab("education")}>Education</span>
                </div>
            </div>
            <div>
                <div ref={workContainer} className="lg:pb-50" >
                    <div className="grid-flex py-4 space-x-2">
                        <WorkExperience
                            title="Full-stack Developer (freelance)"
                            duration="May 2025 - Present"
                            organization="Limpext"
                            technologies={['HTML/CSS/JS', 'Svelte 5', 'PHP', 'Apache']}
                            project="Designed, developed, and launched a custom, professional website for Limpext."
                        >
                            <span><b>Enhanced Visibility: </b> Drove a significant increase in web traffic and improved organic search rankings by implementing comprehensive SEO strategies.</span>
                            <span><b>Full-stack Development: </b> Oversaw all aspects of development, from designing the front-end user interface to crafting the back-end logic.</span>
                        </WorkExperience>
                        <WorkExperience
                            title="Programmer Analyst"
                            duration="January 2020 – February 2021"
                            organization="DXC Technology Vietnam"
                            location="Ho Chi Minh City"
                            technologies={['HTML/CSS/JS', 'AngularJS', 'C#', 'ASP.NET Framework', 'MongoDB', 'OracleDB', 'IIS', 'Task Scheduler']}
                            project="Resolved technical debt and built new features to maintain and enhance enterprise applications for a variety of clients."
                        >
                            <span><b>Front-end Implementation: </b> Utilized AngularJS to implement user-facing features, ensuring a seamless and reliable experience across multiple client projects.</span>
                            <span><b>API Development: </b> Developed new API endpoints using ASP.NET to enhance an internal portal, streamlining data retrieval for an internal sales team.</span>
                            <span><b>Bug Resolution: </b> Fixed critical bugs related to the customer shopping cart and its database, directly improving the stability and reliability of a core e-commerce platform.</span>
                        </WorkExperience>
                        <WorkExperience
                            title="Scientific Assistant"
                            duration="July 2022 – October 2024"
                            organization="Max-Planck Institute für Biophysics"
                            location="Frankfurt am Main"
                            technologies={['Python', 'MATLAB', 'NumPy', 'SciPy', 'Pandas', 'Matplotlib', 'pytest']}
                            project="Transformed a legacy data processing workflow by porting and optimizing MATLAB scripts to Python, enhancing collaboration and research efficiency."
                        >
                            <span><b>Performance Optimization: </b>Accelerated data processing by rewriting legacy MATLAB scripts in Python and leveraging optimized libraries like NumPy and Pandas.</span>
                            <span><b>Workflow Automation: </b>Developed a custom Python module to parse and process a specialized data format, reducing manual data handling time by 50%.</span>
                            <span><b>Quality Assurance: </b> Authored a comprehensive suite of unit tests using pytest, establishing a robust quality assurance framework that ensured the accuracy and integrity of scientific data analysis.</span>
                        </WorkExperience>
                        <WorkExperience
                            title="Teaching Assistant"
                            duration="March 2021 – October 2021"
                            organization="Frankfurt University of Applied Science"
                            location="Frankfurt am Main"
                            technologies={['Java']}
                            project="Guided master's students through a course on competitive programming, focusing on fundamental algorithms and problem-solving techniques."
                        >
                            <span><b>Mentorship & Instruction: </b>Mentored students on key data structures and algorithms, including dynamic programming, graph theory, and computational geometry, improving their problem-solving skills.</span>
                            <span><b>Technical Expertise: </b>Provided direct, real-time feedback on code, identified algorithmic flaws, and suggested optimizations, enhancing the students' coding efficiency and correctness.</span>
                        </WorkExperience>
                    </div>
                </div>
                <div ref={educationContainer} className="hidden w-full "
                >
                    <Education init={tab === "education"}/>
                </div>
            </div>
            
        </div>
    </section>);
}