import React, { useEffect, useRef, useState, type ChangeEvent, type ReactElement } from "react";
import '@/Styles/About.css';
import restart from '@/Assets/SVGs/restart.svg';
import Intro from "./Phases/Intro";
import FirstEncounter from "./Phases/FirstEncounter";
import BSJourney from "./Phases/BSJourney";
import Work from "./Phases/Work";
import MSAdventure from "./Phases/MSAdventure";
import Now from "./Phases/Now";

interface PhaseDetail {
    id?: number;
    fading?: boolean;
    title: string;
    Component: ReactElement
}

const introPhase: PhaseDetail = {
    id: -1,
    title: "Intro",
    Component: <Intro />
};

const phaseDetails: PhaseDetail[] = [
    {
        title: "First Encounter",
        Component: <FirstEncounter />
    },
    {
        title: "B.Sc Journey",
        Component: <BSJourney />
    },
    {
        title: "Work Experience",
        Component: <Work />
    },
    {
        title: "M.Sc Adventure",
        Component: <MSAdventure />
    },
    {
        title: "Now",
        Component: <Now />
    }
];

export default function About() {
    const [phase, setPhase] = useState<number | null>(null);
    const idRef = useRef(0);
    const firstId = useRef(0);
    const [phaseList, setPhaseList] = useState<PhaseDetail[]>([
        introPhase
    ])
    const PhaseComponent = phase !== null && phaseDetails[phase]?.Component ? phaseDetails[phase].Component : Intro;

    const handleChange = (newPhaseNumber: number | null = null) => {

        let reload = true;
        setPhase(phaseNumber => {
            reload = (newPhaseNumber !== phaseNumber);
            return newPhaseNumber;
        });
            
        if (!reload) return;

        const newPhase = newPhaseNumber === null || phaseDetails[newPhaseNumber] === undefined
            ? { ...introPhase, id: idRef.current }
            : { ...phaseDetails[newPhaseNumber], id: idRef.current };

        setPhaseList(phaseList => {
            const head = phaseList.slice(0, firstId.current + 1).map(phase => ({
                ...phase,
                fading: true
            }));

            idRef.current = newPhase.id + 1;
            return [...head, ...phaseList.slice(firstId.current), newPhase];
        });

        firstId.current++;

        setTimeout(() => {
            firstId.current--;
            setPhaseList(phaseList => phaseList.slice(1));
        }, 1000)
    };

    return (
        <div className="flex flex-col w-[calc(100%-1rem)] sm:w-[calc(100%-5rem)] mx-auto">
            <div className="flex h-20 justify-center items-center">
                <p className="text-4xl after-about-title w-full text-center font-bold text-white-chalk">About me</p>
            </div>
            <div className="flex h-[min(max(calc(100dvh-8rem),46.5rem),65rem)] w-full flex-row-reverse sm:flex-col justify-end sm:justify-start">
                <div className="relative w-full sm:w-[min(100%,75rem)] h-full ml-4 sm:mx-auto mr-2 overflow-y-auto scrollbar-custom bg-darkboard/70 rounded-2xl drop-shadow-darkboard shadow-xl text-justify text-lg xs:text-xl sm:text-2xl">
                    {phaseList.map(({id, fading, Component}) => <div 
                        key={id} 
                        className={`absolute top-0 left-0 p-4 w-full h-full opacity-0 transition-opacity ${fading? "animate-[fadeOut_0.4s_ease-in_forwards]" : "animate-[fadeIn_0.4s_ease-in_forwards]"}`}
                    >{Component}</div>)}
                </div>
                <div className="w-auto sm:w-[min(100%,75rem)] h-[calc(100%-6rem)] sm:h-auto ml-0 sm:mx-auto">
                    <div className="relative flex flex-col justify-center w-full h-10">
                        <span className={`${phase !== null ? "opacity-100" : "opacity-0"} hidden sm:block absolute text-nowrap transition-all duration-200`}>
                            Phase {phase !== null && phase + 1}: {phase !== null && phaseDetails[phase]?.title}
                        </span>
                    </div>
                    <div className="flex flex-col sm:flex-row items-center w-auto sm:w-full h-full sm:h-auto gap-2">
                        <span className="w-6 h-6 rounded-full bg-yellow-chalk cursor-pointer hover:brightness-110 hover:scale-110 transition-all duration-200"
                            onClick={() => handleChange()}
                        >
                            <img className="filter-blackboard translate-x-[0.5px] translate-y-[-0.5px] hover:-rotate-180 hover:translate-y-[1px] transition-all duration-200" src={restart}></img>
                        </span>
                        <div className="flex flex-col sm:flex-row w-auto sm:w-full h-full sm:h-auto [&>span]:transition-all [&>span]:duration-200 [&>span]:bg-yellow-chalk [&>span]:hover:bg-orange-chalk">
                            {phaseDetails.map((detail, index) => (
                                <React.Fragment key={index}>
                                    <div className="relative flex flex-col items-center">
                                        <input className={`milestone cursor-pointer ${phase === index ? 'checked' : ''}`} 
                                            type="radio" 
                                            name={`phase-${index + 1}`} 
                                            value={index} 
                                            checked={phase === index} 
                                            onChange={() => handleChange(index)}
                                        />
                                        <div className={`absolute left-0 text-left top-full w-20 ${phase === null ? "hidden sm:block" : "hidden"}`}>
                                            {detail.title}
                                        </div>
                                    </div>
                                    <span className={`${(phase === null || phase === index) ? "flex-1" : "flex-[0.1]"} m-1 rounded-full cursor-pointer hover:scale-x-125 sm:hover:scale-x-100 hover:scale-y-100 sm:hover:scale-y-125 hover:brightness-110`}
                                        onClick={() => handleChange(index)}
                                    />
                                </React.Fragment>
                            ))}
                        </div>
                    </div>            
                    <div className="relative flex flex-col justify-end items-center w-14 sm:w-full h-10 p-auto sm:p-10">
                        <button className={`${phase !== null ? "opacity-100" : "opacity-0"} left-0 right-auto sm:left-auto sm:right-0 absolute w-full sm:w-auto  px-auto sm:px-4 h-8 font-semibold rounded-xl transition-opacity duration-200 ${phase !== phaseDetails.length - 1 ? "bg-yellow-chalk text-darkboard shadow-[0_0.2rem_0_var(--color-yellow-chalk-dark)] cursor-pointer hover:brightness-110 active:translate-y-[0.2rem] active:shadow-none" : "bg-gray-300 text-gray-600 shadow-[0_0.2rem_0_var(--color-gray-400)]"}`}
                            onClick={() => handleChange(phase !== null ? phase + 1 : 0)}
                            disabled={phase === phaseDetails.length - 1}
                        >
                            <span>Next</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    )
}