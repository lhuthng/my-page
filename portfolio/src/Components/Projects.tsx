import CoolHeader from "./CoolHeader";
import React, { useEffect, useRef, useState, type ReactNode } from "react";
import gsap from "gsap";
import { Flip } from "gsap/all";
import ThisPortfolio from "./Projects/ThisPortfolio";
import useDebounce from "./Hooks/debounce";

export default function Projects() {
    const projects = [
        <ThisPortfolio />,
        <ThisPortfolio />,
        <ThisPortfolio />,
        <ThisPortfolio />,
        <ThisPortfolio />,
        <ThisPortfolio />,
    ];
    return <section className="w-full p-4">
        <CoolHeader title="Projects" />
        <div className="w-full inset-au m-auto">
            <div className="grid grid-cols-1 md:grid-cols-[repeat(auto-fit,minmax(20rem,1fr))] grid-rows- gap-4 w-full ">
                {projects.map((project, index) => (
                    <div className="project-container w-full h-120" key={index}>
                        {project}
                    </div>
                ))}
            </div>
        </div>
    </section>
}