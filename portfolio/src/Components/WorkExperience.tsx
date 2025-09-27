import type { ReactNode } from "react"
import { Children, useEffect, useLayoutEffect, useRef, useState } from "react"
import '@/Styles/Experience.css'
import gsap from "gsap"
import { useGSAP } from "@gsap/react"

interface ExperienceInfoProps {
    title: string,
    duration: string
    organization: string,
    location?: string,
    technologies: string[],
    project: string,
    additional?: {
        externalDocument?: {
            name: string,
            url: string
        }[],
        caseStudy?: string
    },
    children: ReactNode
}

export default function WorkExperience({
    title, duration, organization, technologies, project, location, additional, children
}: ExperienceInfoProps) {
    const [ expanded, setExpanded ] = useState(false);
    const borderRef = useRef<HTMLDivElement>(null);
    const slotRef = useRef<HTMLDivElement>(null);

    useGSAP(() => {
        gsap.utils.toArray(".cell-adjust").forEach((border) => {
            gsap.to(border as Element, {
                scrollTrigger: {
                    trigger: border as Element,
                    start: "center bottom",
                    toggleClass: "is-visible",
                    once: true
                }
            });
        });
        gsap.utils.toArray(".details-slot").forEach(slot => {
            gsap.to(slot as Element, {
                scrollTrigger: {
                    trigger: slot as Element,
                    start: "top bottom",
                    toggleClass: "is-visible",
                    once: true
                }
            });
        });
    });

    useEffect(() => {
        borderRef.current?.classList.toggle('open', expanded);
        slotRef.current?.classList.toggle('open', expanded);
    }, [expanded]);


    return (<div className={`cell-adjust w-full space-y-2 bg-blackboard z-0}`}>
        <div ref={borderRef} className="custom-border z-11"><i/><i/></div>
        <div className="relative m-0 pb-2 bg-blackboard space-y-4 z-10">
            <div className="relative space-y-4 p-4 pb-2">
                <div className="relative w-full">
                    <div className="absolute inset-y-1/2 w-full h-[2px] bg-white z-0"></div>
                    <div className="relative title flex font-bold text-2xl w-fit bg-white text-blackboard z-10">{title}</div>
                </div>
                <div className="flex justify-between italic">
                    <div><div className="font-bold">{organization}</div><div>{location ?? "(remote)"}</div></div>
                    <span className="text-gray-chalk">{duration}</span>
                </div>
                <div><b>Technologies:</b> {technologies.join(', ')}.</div>
                <div><b>Project:</b> {project}</div>
            </div>        
            <div className="px-4"><b>Contributions:</b> <span className="text-xs rounded-full border-1 px-1 cursor-pointer hover:brightness-140 transition-all duration-100" 
                style={{
                    borderColor: "var(--color-gray-chalk)",
                    color: expanded ? "var(--color-blackboard)" : "var(--color-gray-chalk)",
                    backgroundColor: expanded ? "var(--color-gray-chalk)" : "transparent"
                }}
                onClick={() => setExpanded(expanded => !expanded)}
            >{expanded ? "collapse" : "expand" }</span></div>
        </div>          
        <div ref={slotRef} className="details-slot not-lg:bg-blackboard overflow-y-hidden z-9">
            <ul className="contributions-marker bg-blackboard space-y-2 pl-10 pr-4 pb-4">
                {Children.map(children, (child, index) => <li key={index}>{child}</li>)}
                {additional && <li className="list-none flex" 
                    style={{
                        justifyContent: additional.externalDocument && additional.caseStudy ? 
                            "space-between" 
                            : additional.externalDocument ? 
                                "flex-start"
                                : "flex-end"
                    }}
                >
                    {additional?.externalDocument && 
                        <span className="text-sm">
                            Links: <span className="space-x-2">{additional.externalDocument.map((doc, index) => <a 
                                key={index} 
                                className="underline text-nowrap" 
                                href={doc.url} 
                                target="_blank"
                            >
                                {doc.name}
                            </a>)}</span>
                        </span>
                    }
                    {additional?.caseStudy && <span className="text-sm opacity-60 text-blue-chalk hover:brightness-110 cursor-pointer text-nowrap"><span className="select-none opacity-20">[ </span>Read the Case Study<span className="select-none opacity-20"> ]</span></span>}
                </li>}
            </ul>
        </div>
    </div>)
}