import Card, { type CardProps } from "./Card";
import frontend from "@/Assets/Images/frontend.jpg";
import backend from "@/Assets/Images/backend.jpg";
import system from "@/Assets/Images/system.jpg";
import cicd from "@/Assets/Images/cicd.jpg";
import database from "@/Assets/Images/database.jpg";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import EmptyCard from "./EmptyCard";
import CoolHeader from "./CoolHeader";
import CardInfo from "./CardInfo";
import { smallWidth } from "@/Utils/common";
import gsap from "gsap";

const cards: CardProps[] = [
    {
        title: "Frontend",
        level: 1,
        colorPresetName: "Charisma",
        difficulty: 1,
        attack: 1000,
        defense: 1000,
        children: [
            <div className="w-full h-full"
                style={{
                    backgroundImage: `linear-gradient(rgba(255,192,203,0.1), rgba(255,192,203,0.1)), url(${frontend})`,
                    backgroundPosition: `50% 20%`,
                }}
            />,
            <p>Raising sleek, interactive UIs from tiny ideas. Bringing up every pixel with care.</p>
        ],
        details: [
            {
                x: 134, y: 130, dx: 0, dy: 0,
                detail: <div className="w-56 text-left"><b>Core:</b> HTML/CSS/JS, TypeScript</div>,
                paths: [[-45, 60],[0,260]]
            },
            {
                x: 100, y: 180, dx: 0, dy: 0,
                detail: <div className="w-52 text-right"><b>Frameworks:</b> React, Angular, Razor Page, Svelte 5</div>,
                paths: [[135,60],[180,184]]
            },
            {
                x: 140, y: 245, dx: 0, dy: 0,
                detail: <div className="w-36 text-right"><b>Styling:</b> TailwindCSS</div>,
                paths: [[0,160],[45,40],[0,80]]
            }
        ],
    },
    {
        title: "Backend",
        level: 2,
        colorPresetName: "Intelligent",
        difficulty: 4,
        attack: 800,
        defense: 2500,
        children: [
            <div className="w-full h-full"
                style={{
                    backgroundImage: `url(${backend})`,
                    backgroundPosition: `50% 12%`,
                }}
            />,
            <p>Just another Tuesday. Building robust services and APIs that fight off the hordes.</p>
        ],
        details: [
            {
                x: 220, y: 80, dx: 0, dy: 0,
                detail: <div className="w-46 text-left"><b>Languages:</b> Go, C#, Rust, Python, C++, PHP</div>,
                paths: [[45, 60],[0,210]]
            },
            {
                x: 130, y: 85, dx: 0, dy: 0,
                detail: <div className="w-41 text-right"><b>Frameworks:</b> Node.js, FastAPI, ASP.NET</div>,
                paths: [[180,280]]
            },
            {
                x: 110, y: 170, dx: 0, dy: 0,
                detail: <div className="w-54 text-right"><b>Communication:</b> WebSocket, REST, gRPC, JSON-RPC</div>,
                paths: [[180,120],[135,100],[180,60]]
            }
        ],
    },
    {
        title: "Systems",
        level: 3,
        colorPresetName: "Resilience",
        difficulty: 3,
        attack: 1000,
        defense: 2000,
        children: [
            <div className="w-full h-full"
                style={{
                    backgroundImage: `url(${system})`,
                    backgroundPosition: `50% 12%`,
                }}
            />,
            <p>Hold my coffee. Taming the systems that are built to withstand the blaze.</p>
        ],
        details: [
            {
                x: 140, y: 155, dx: 0, dy: 0,
                detail: <div className="w-46 text-left"><b>Containerization:</b> Docker</div>,
                paths: [[135, 60],[180,210]]
            },
            {
                x: 220, y: 80, dx: 0, dy: 0,
                detail: <div className="w-36 text-left"><b>Auth:</b> JWT, OAuth2, Session-based auth</div>,
                paths: [[0, 60],[0,140]]
            },
            {
                x: 75, y: 120, dx: 0, dy: 0,
                detail: <div className="w-46 text-right"><b>Servers:</b> Windows Server, Linux Server</div>,
                paths: [[180,100],[-135,30],[180,80]]
            },
            {
                x: 160, y: 190, dx: 0, dy: 0,
                detail: <div className="w-60 text-left"><b>Networking & Infra:</b> Apache2, nginx, Alfahosting, Domain Management, TLS/SSL, VPS, SSH</div>,
                paths: [[0,120],[45,100],[0,80]]
            }
        ],
    },
    {
        title: "DevOps",
        level: 0.5,
        colorPresetName: "Agility",
        difficulty: 2,
        attack: 500,
        defense: 1500,
        effect: "Fusion",
        children: [
            <div className="w-full h-full"
                style={{
                    backgroundImage: `url(${cicd})`,
                    backgroundPosition: `50% 10%`,
                    backgroundRepeat: "no-repeat",
                    backgroundColor: "white"
                }}
            />,
            <p>Automating workflows with CI/CD, ensuring smooth builds, tests, and deployments.</p>
        ],
        details: [
            {
                x: 140, y: 190, dx: 0, dy: 0,
                detail: <div className="w-38 text-left"><b>Version Control:</b> Git, GitLab, Azure Repos</div>,
                paths: [[45,22],[0,240]]
            },
            {
                x: 140, y: 90, dx: 0, dy: 0,
                detail: <div className="w-27 text-right"><b>CI/CD:</b> Jenkins, Github Actions</div>,
                paths: [[-135,22], [180,220]]
            },
            {
                x: 85, y: 200, dx: 0, dy: 0,
                detail: <div className="w-31 text-right"><b>Testing:</b> unittest, junit, MSTest</div>,
                paths: [[180,40],[135,30],[180,120]]
            }
        ],
    },
    {
        title: "Database",
        level: 1,
        colorPresetName: "Wisdom",
        difficulty: 1,
        effect: "Power",
        children: [
            <div className="w-full h-full"
                style={{
                    backgroundImage: `url(${database})`,
                    backgroundPosition: `50% 0%`,
                    backgroundRepeat: "no-repeat",
                    backgroundColor: "white"
                }}
            />,
            <p>The keeper of secrets. Mastering data integrity to ensure your information is always ready, fast, and sound.</p>
        ],
        details: [
            {
                x: 190, y: 90, dx: 0, dy: 0,
                detail: <div className="w-52 text-left"><b>Databases:</b> PostgreSQL, MySQL, OracleDB, MongoDB</div>,
                paths: [[45,80],[0,166]]
            },
            {
                x: 90, y: 90, dx: 0, dy: 0,
                detail: <div className="w-48 text-right"><b>Concepts:</b> Schema Design, Data Modeling, Indexing</div>,
                paths: [[135,80], [180,160]]
            },
        ],
    },        
];

export default function Skills() {

    const [ selection, setSelection ] = useState<number>();
    const [ compactSelection, setCompactSelection ] = useState<number>();
    const gridRef = useRef<HTMLDivElement>(null);
    const [ isSmall, setIsSmall ] = useState(false);

    useLayoutEffect(() => {
        const onResize = () => {
            setIsSmall((isSmall) => {
                const newIsSmall = window.innerWidth < smallWidth;
                setCompactSelection(compactSelection => {
                    isSmall !== newIsSmall && console.log(isSmall, newIsSmall, compactSelection);
                    if (!isSmall && newIsSmall) {
                        return undefined;
                    }
                    if (isSmall && !newIsSmall) {
                        setSelection(compactSelection);
                    }
                    return compactSelection;
                });
                return newIsSmall;
            });
        };
        onResize();

        window.addEventListener("resize", onResize);
        return () => {
            window.removeEventListener("resize", onResize);
        }
    }, []);

    return (
    <section className="max-w-340 mx-auto">
        <CoolHeader title="Skills" />
        <div className="grid-container w-full" ref={gridRef}>
            {!isSmall && selection !== undefined && cards[selection] && <div className="col-span-full">
                <Card {...cards[selection]} key={selection} expanded={true}/>
            </div>}
            {cards.map((props, index) => (isSmall || selection !== index) ? <Card {
                    ...props
                }
                    key={index}
                    isSmall={isSmall}
                    onClick={(e: React.MouseEvent<HTMLElement>) => {
                        setSelection(index);
                        if (!gridRef.current) return;
                        gsap.to(window, {
                            duration: 0.2,
                            scrollTo: {
                                y: isSmall ? e.currentTarget : gridRef.current,
                                offsetY: 75,
                                autoKill: true
                            },
                            ease: "expo.out"
                        });
                    }}
                    compactSelected={compactSelection === index}
                    onDetailCallback={(toggle) => setCompactSelection(toggle ? index : undefined)}
                />
                : <EmptyCard key={index}/>
            )}
        </div>
    </section>);
}