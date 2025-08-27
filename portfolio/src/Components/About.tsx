import React, { useEffect, useState, type ChangeEvent } from "react";
import '@/Styles/About.css';

interface PhaseDetail {
    title: string;
}

const phaseDetails: PhaseDetail[] = [
    {
        title: "Discovering Interest"
    },
    {
        title: "Building Knowledge"
    },
    {
        title: "Stepping into Practices"
    },
    {
        title: "Deepening Expertise"
    },
    {
        title: "Current Chapter"
    }
];

export default function About() {
    const [phase, setPhase] = useState<number | null>(null);

    const handleChange = (phaseNumber: number) => {
        setPhase(phaseNumber);
    }

    return (
        <div className="h-[200dvh] w-[calc(100%-10rem)] flex flex-col mx-auto items-center">
            <div className="h-30">

            </div>
            <div className="flex flex-col justify-center w-full h-10">
                <span className={`${phase !== null ? "opacity-100" : "opacity-0"} transition-all duration-200`}>
                    Phase {phase !== null && phase + 1}: {phase !== null && phaseDetails[phase]?.title}
                </span>
            </div>
            <div className="w-full flex [&>span]:transition-all [&>span]:duration-200 [&>span]:bg-yellow-chalk">
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
                            <div className={`absolute top-full w-30 text-center ${phase === null ? "block" : "hidden"}`}>
                                {detail.title}
                            </div>
                        </div>
                        <span className={`${(phase === null || phase === index) ? "flex-1" : "flex-[0.1]"} m-1 rounded-full cursor-pointer hover:scale-y-125 hover:brightness-110`}
                            onClick={() => handleChange(index)}
                        />
                    </React.Fragment>
                ))}
            </div>
            <div className="flex flex-col justify-end items-end w-full h-15">
                <button className={`${phase !== null ? "opacity-100" : "opacity-0"} w-30 h-8 font-semibold rounded-xl transition-opacity duration-200 ${phase !== phaseDetails.length - 1 ? "bg-yellow-chalk text-darkboard shadow-[0_0.2rem_0_var(--color-yellow-chalk-dark)] cursor-pointer hover:brightness-110 active:translate-y-[0.2rem] active:shadow-none" : "bg-gray-300 text-gray-600 shadow-[0_0.2rem_0_var(--color-gray-400)]"}`}
                    onClick={() => handleChange(phase !== null ? phase + 1 : 0)}
                    disabled={phase === phaseDetails.length - 1}
                >
                    <span>Next</span>
                </button>
            </div>
        </div>
    )
}