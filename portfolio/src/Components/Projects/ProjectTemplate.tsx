import { Flip } from "gsap/all";
import gsap from "gsap";
import { useEffect, useRef, useState, type ReactNode } from "react";

const classState = {
    container: {
        off: ["relative"],
        on: ["fixed"],
    },
    absolute: {
        off: ["w-full", "h-full", "top-0","left-0"],
        on: [
            "w-[calc(100%-2rem)]", "md:w-[calc(100%-4rem)]", 
            "h-[calc(100%-2rem)]", "md:h-[max(40rem,calc(100%-4rem))]", 
            "border-2",
            "top-8", "left-1/2"
        ],
    },
    compact: {
        off: ["w-full", "h-full"],
        on: ["w-full", "md:w-160", "h-80", "md:h-full" ],
    },
    detail: {
        off: ["w-full", "md:w-0", "h-0", "md:h-full"],
        on: ["w-full", "flex-1", "md:h-full" ]
    },
    illustration: {
        off: ["h-60"],
        on: ["h-100"],
    }
}

const turn = (
    element: HTMLElement | Element, 
    stateName: "container" | "absolute" | "compact" | "detail" | "illustration", 
    on: boolean
) => {
    const state = classState[stateName];
    element.classList.remove(...state[on ? "off" : "on"]);
    element.classList.add(...state[on ? "on" : "off"]);
}

interface ProjectProps {
    illustration: ReactNode,    
    description: ReactNode,
    details: ReactNode
}

export default function ProjetTemplate({
    illustration, description, details
} : ProjectProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const [ active, setActive ] = useState(false);


    useEffect(() => {
        const container = containerRef.current;
        const compact = container?.querySelector(".compact-view");
        const absolute = container?.querySelector(".absolute");
        const detail = container?.querySelector(".detail-view");
        const illustration = container?.querySelector(".illustration-view");

        if (!container || !compact || !detail || !illustration || !absolute) return;

        const state = Flip.getState([ container, compact, absolute, illustration,detail ])

        if (active) {
            gsap.set(container, {
                
                zIndex: 50
            });

            gsap.set(absolute, {
                transform: "translateX(-50%)",
                overflowY: "scroll",
            });

            turn(container, "container", true);
            turn(compact, "compact", true);
            turn(absolute, "absolute", true);
            turn(detail, "detail", true);
            turn(illustration, "illustration", true);

            Flip.from(state, {
                duration: 0.5,
                ease: "power2.inOut",
            });
        }
        else {
            gsap.set(absolute, {
                transform: "translateX(0px)",
                overflowY: "hidden",
                scrollTo: 0
            });

            turn(container, "container", false);
            turn(compact, "container", false);
            turn(absolute, "absolute", false);
            turn(detail, "detail", false);
            turn(illustration, "illustration", false);            

            Flip.from(state, {
                duration: 0.5,
                ease: "power2.inOut",
                onComplete: () => {
                    gsap.set(container, {
                        zIndex: "auto"
                    });
                }
            });
        }
    }, [ active ]);

    return <div className="relative flex flex-col w-full h-full top-0 left-0 bg-none"
        ref={containerRef}
        onClick={() => setActive(active => !active)}
    >
        <div className="absolute left-0 top-0 w-full h-full rounded-2xl scrollbar-custom bg-white-chalk scroll-thumb-black overflow-hidden">
            <div className="illustration-view relative w-full h-60">
                <i className="absolute left-0 top-0 w-full h-[calc(100%+40rem)] "/>
                {illustration}
            </div>
            <div className="flex flex-col md:flex-row relative w-full h-full">
                <div className="compact-view relative w-full h-full rounded-t-4xl "
                >
                    {description}
                </div>
                <div className="detail-view relative w-0 rounded-t-none md:rounded-t-4xl ">
                    {details}
                </div>
            </div>
        </div>
    </div>
}