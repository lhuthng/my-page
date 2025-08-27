import me from "@/Assets/Images/me.webp";
import '@/Styles/Landing.css';
import { useEffect, useRef, useState } from "react";
import useInterval from "./Hooks/interval";
import useRandomizer from "./Hooks/randomizer";

enum State {
    WAIT, BREATH_IN, BREATH_OUT
}

const roles = [
    "Programmer Analyst",
    "Full-stack Developer",
    "Data & Algorithm Engineer"
];

const coolEmojis = [
    "🌐", "👨‍💻", "📊", "🧩", "⚙️", "🖥️"
];

export default function Landing() {
    const breathTime = 50, waitTime = 1500;

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
            setIsWaiting(true);
        });
        onEndedWaiting(() => {
            onStartedBreathOut(startBO, breathTime);
            setIsWaiting(false);
        });
        onEndedBreathOut(() => onStartedBreathIn(startBI, breathTime));

        onStartedBreathIn(startBI, breathTime);
        
        return () => {
            onStoppedBreathIn();
            onStoppedBreathOut();
            onStoppedWaiting();
        }
    }, [])

    return (
        <div className="h-[100dvh] w-full">
            <div className="flex justify-center items-center">
                <div className="w-100 text-2xl">
                    <p>Hi 👋, my name is <span className="text-transparent bg-clip-text bg-linear-to-r from-yellow-chalk to-green-chalk">Huu Thang Le</span></p>
                    <p>I'm a <span className="text-transparent bg-clip-text bg-linear-to-r from-blue-chalk to-purple-chalk">{role}</span>
                    {isWaiting && <span>&nbsp;{getRandomEmoji()}</span>}<span className={`inline-block w-0.5 h-8 bg-white-chalk ${isWaiting ? "animate-blink" : ""}`}>&nbsp;</span></p>
                </div>
                <div className="w-50 h-50">
                    <img className="colorful-frame w-full h-full border-5 border-transparent rounded-full" src={me}></img>
                </div>
            </div>
        </div>
    )
}