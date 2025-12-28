import '@/Styles/Landing.css';
import { useEffect, useRef, useState } from "react";
import Lottie from './Lottie';
import useInterval from "@/Utils/Hooks/interval";
import useRandomizer from "@/Utils/Hooks/randomizer";

import me from "@/Assets/Images/me.webp";
import wink from "@/Assets/Lotties/wink.json";
import gear from "@/Assets/Lotties/gear.json";
import robot from "@/Assets/Lotties/robot.json";
import rocket from "@/Assets/Lotties/rocket.json";
import coffee from "@/Assets/Lotties/coffee.json";

import at from "@/Assets/SVGs/at.svg";
import github from "@/Assets/SVGs/github.svg";
import linkedin from "@/Assets/SVGs/linkedin.svg";


const roles = [
    "Programmer Analyst",
    "Full-stack Developer",
    "Data & Algorithm Engineer",
    "Freelancer"
];

const coolEmojis = [
    gear, robot, rocket, coffee
];

export default function Landing() {
    const breathInTime = 50, breathOutTime = 25, waitTime = 2000;

    const [ index, setIndex ] = useState(0);
    const [ role, setRole ] = useState("");
    const targetRole = useRef(roles[index]);

    const [ onStartedBreathIn, onStoppedBreathIn, onEndedBreathIn ] = useInterval();
    const [ onStartedBreathOut, onStoppedBreathOut, onEndedBreathOut ] = useInterval();
    const [ onStartedWaiting, onStoppedWaiting, onEndedWaiting ] = useInterval();

    const getRandomEmoji = useRandomizer(coolEmojis);

    const [ isWaiting, setIsWaiting ] = useState(false);

    useEffect(() => {
        const startBI = () => {
            if (targetRole === undefined) return true;

            let isEnded = false;
            setRole(role => {
                const length = role.length;
                if (targetRole.current === undefined || length === targetRole.current.length) {
                    isEnded = true;
                    return role;
                }
                return targetRole.current.substring(0, length + 1);
            });
            return isEnded;
        };

        const startBO = () => {
            if (targetRole === undefined) return true;

            let isEnded = false;
            setRole(role => {
                const length = role.length;
                if (targetRole.current === undefined || length === 0) {
                    isEnded = true;
                    return role;
                }
                return targetRole.current.substring(0, length - 1);
            });

            if (isEnded) {
                setIndex(index => {
                    const next = (index + 1) % roles.length;
                    targetRole.current = roles[next];
                    return next;
                });
            }
            return isEnded;
        }

        const startW = () => {
            return true;
        }

        onEndedBreathIn(() => {
            onStartedWaiting(startW, waitTime);
            setTimeout(() => setIsWaiting(true), breathInTime / 2);
        });
        onEndedWaiting(() => {
            onStartedBreathOut(startBO, breathOutTime);
            setIsWaiting(false);
        });
        onEndedBreathOut(() => onStartedBreathIn(startBI, breathInTime));

        onStartedBreathIn(startBI, breathInTime);
        
        return () => {
            onStoppedBreathIn();
            onStoppedBreathOut();
            onStoppedWaiting();
        }
    }, [])

    return (
        <div className="flex relative flex-col h-[max(100dvh,50rem)] lg:h-[max(100dvh,35rem)] w-full gap-10 justify-center items-center">
            <div className='absolute w-full h-full opacity-5 bg-[url("@/Assets/SVGs/pattern.svg")] animate-[panBackground_120s_linear_infinite]' style={{backgroundSize: "25rem"}}/>
            <div className="flex flex-col w-[min(50rem,100%)] lg:w-220 text-2xl gap-10">
                <div className="flex flex-col-reverse lg:flex-row gap-10 lg:gap-0 justify-between items-center">
                    <div className="w-[min(37.5rem,calc(100%-1rem))] text-2xl opacity-0 animate-[fadeIn_0.5s_ease-in_forwards]">
                        <div className="h-24 sm:h-auto">
                            <span>Hi <Lottie className="inline-block w-6 h-6 translate-y-1" animationData={wink} loop={true}/>, my name is <span className="text-transparent bg-clip-text bg-linear-to-r from-yellow-chalk to-green-chalk">Huu Thang Le</span></span>
                            <br/>
                            <span>I'm a <span className="text-transparent bg-clip-text bg-linear-to-r from-blue-chalk to-purple-chalk">{role}</span>
                            {isWaiting && <span>&nbsp;<Lottie className="inline-block w-6 h-6 translate-y-1" animationData={getRandomEmoji(true)} loop={true}/></span>}<span className={`inline-block w-0.5 h-8 bg-white-chalk ${isWaiting ? "animate-blink" : ""}`}>&nbsp;</span></span>
                        </div>
                        <div className="mt-10">
                            <p>Passionate about computer science, I've been coding and building projects for years. Welcome to my portfolio!</p>
                        </div>
                    </div>
                    <div className="w-50 lg:w-60 h-50 lg:h-60 opacity-0 animate-[fadeIn_0.5s_ease-in_forwards]">
                        <img className="colorful-frame w-full h-full border-5 border-transparent rounded-full" src={me}></img>
                    </div>
                </div>
                <div className="flex flex-col gap-5 mx-auto lg:mx-0 w-[min(37.5rem,calc(100%-1rem))] opacity-0 animate-[fadeIn_0.5s_ease-in_forwards]">
                    <div>
                        <span>
                            <button className="animate-[shiftDown_0.5s_ease_forwards] px-4 h-10 rounded-3xl transition-opacity duration-200 text-darkboard duolingo-button yellow-button">
                                <span>Let's connect!</span>
                            </button>
                        </span>
                        <span className='text-lg text-gray-400'> , or simply via</span>
                    </div>
                    <div className='space-x-4'>
                        <span className='opacity-0 animate-[fadeIn_0.5s_ease-in_0.1s_forwards]'>
                            <button className='animate-[shiftDown_0.5s_ease_0.1s_forwards] w-12 h-12 duolingo-button yellow-button'>
                                <img className='w-[60%] mx-auto translate-x-[0.5px] filter-blackboard' src={linkedin}></img>
                            </button>
                        </span>
                        <span className='opacity-0 animate-[fadeIn_0.5s_ease-in_0.2s_forwards]'>
                            <button className='animate-[shiftDown_0.5s_ease_0.2s_forwards] w-12 h-12 duolingo-button yellow-button'>
                                <img className='w-[80%] mx-auto translate-x-[0.5px] filter-blackboard' src={github}></img>
                            </button>
                        </span>
                        <span className='opacity-0 animate-[fadeIn_0.5s_ease-in_0.3s_forwards]'>
                            <button className='animate-[shiftDown_0.5s_ease_0.3s_forwards] w-12 h-12 duolingo-button yellow-button'>
                                <img className='w-[60%] mx-auto filter-blackboard' src={at}></img>
                            </button>
                        </span>
                    </div>
                </div>
            </div>
        </div>
    )
}