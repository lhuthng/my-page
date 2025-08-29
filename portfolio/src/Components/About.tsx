import React, { useEffect, useState, type ChangeEvent } from "react";
import '@/Styles/About.css';
import restart from '@/Assets/SVGs/restart.svg';

interface PhaseDetail {
    title: string;
}

const phaseDetails: PhaseDetail[] = [
    {
        title: "First Encounter"
    },
    {
        title: "B.Sc Journey"
    },
    {
        title: "Work Experience"
    },
    {
        title: "M.Sc Adventure"
    },
    {
        title: "Now"
    }
];

export default function About() {
    const [phase, setPhase] = useState<number | null>(null);

    const handleChange = (phaseNumber: number | null = null) => {
        setPhase(phaseNumber);
    }

    return (
        <div className="h-[100dvh] w-[calc(100%-1rem)] sm:w-[calc(100%-5rem)] mx-auto">
            <div className="flex h-20 justify-center items-center">
                <p className="text-4xl after-about-title font-bold text-white-chalk">About me</p>
            </div>
            <div className="w-full h-full flex flex-row-reverse sm:flex-col justify-end sm:justify-start">
                <div className="w-full sm:w-[min(100%,75rem)] h-[calc(100%-14rem)] sm:h-160 ml-4 sm:mx-auto mr-2 my-10 sm:my-0 bg-green-chalk">

                </div>
                <div className="w-auto sm:w-[min(100%,75rem)] h-[calc(100%-14rem)] sm:h-auto ml-0 sm:mx-auto">
                    <div className="relative flex flex-col justify-center w-full h-10">
                        <span className={`${phase !== null ? "opacity-100" : "opacity-0"} absolute text-nowrap transition-all duration-200`}>
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
                                        <input className="milestone cursor-pointer" 
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
                    <div className="relative flex flex-col justify-end items-end w-full h-10">
                        <button className={`${phase !== null ? "opacity-100" : "opacity-0"} left-0 right-auto sm:left-auto sm:right-0 absolute px-4 h-8 font-semibold rounded-xl transition-opacity duration-200 ${phase !== phaseDetails.length - 1 ? "bg-yellow-chalk text-darkboard shadow-[0_0.2rem_0_var(--color-yellow-chalk-dark)] cursor-pointer hover:brightness-110 active:translate-y-[0.2rem] active:shadow-none" : "bg-gray-300 text-gray-600 shadow-[0_0.2rem_0_var(--color-gray-400)]"}`}
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