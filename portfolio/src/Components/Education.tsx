import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { useEffect, useRef, useState } from "react"
import '@/Styles/Experience.css'

interface EducationProps {
    init: boolean
}

export default function Education({
    init: onTab
}: EducationProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const iRef = useRef<HTMLElement>(null);
    const leftRef = useRef<HTMLDivElement>(null);
    const rightRef = useRef<HTMLDivElement>(null);
    const tl = useRef<gsap.core.Timeline>(gsap.timeline({ paused: true }));
    const [ initialized, setInitialized ] = useState(false);

    useGSAP(() => {
        if (!iRef.current) return;

        tl.current.set(iRef.current, {
            x: "-50%"
        });

        tl.current.to(iRef.current, {
            rotate: 420,
            duration: 1,
            width: "200px",
            transformOrigin: "center center",
            ease: "circ.in"
        }).to(leftRef.current, {
            x: 0,
            opacity: 1,
            duration: 0.5,
        }).to(rightRef.current, {
            x: 0,
            opacity: 1,
            duration: 0.5,
        }, "-=0.5");
    }, []);

    useGSAP(() => {
        if (!iRef.current) return;
        gsap.to(containerRef.current, {
            scrollTrigger: {
                trigger: containerRef.current as Element,
                start: "top bottom",
                onEnter: () => {
                    setInitialized(true);
                    if (onTab) {
                        tl.current?.play();
                    }
                    else {
                        tl.current?.reverse();
                    }
                },
                once: true,
            },
        });
    }, [onTab])

    return <div ref={containerRef} className="min-h-60 h-[calc(100dvh-20rem)] flex items-center">
        <div className="education-container relative flex flex-col sm:flex-row mx-auto w-full h-100 sm:h-50 justify-center">
            <i ref={iRef} className="absolute content-[''] top-1/2 left-1/2 h-1 rounded-sm bg-white z-10"/>
            <div className="relative w-full h-full overflow-hidden">
                <div ref={leftRef} className="absolute flex flex-col gap-2 sm:justify-center items-center sm:items-start h-40 sm:w-74 right-0 top-1/2 opacity-0 translate-x-full -translate-y-1/2">
                    <h1 className="text-lg xs:text-xl font-bold">B.Sc. in Computer Science</h1>
                    <p className="text-sm xs:text-base">Frankfurt University of Applied Science</p>
                </div>
            </div>
            <div  className="relative w-full h-full overflow-hidden ">
                <div ref={rightRef} className="absolute flex flex-col gap-2 justify-end sm:justify-center items-center sm:items-end h-40 sm:w-74 left-0 top-1/2 opacity-0 -translate-x-full -translate-y-1/2">
                    <h1 className="text-lg xs:text-xl font-bold">M.Sc. in Computer Science</h1>
                    <p className="text-xs xs:text-base">University of Passau</p>
                </div>
            </div>
        </div>
    </div>
}