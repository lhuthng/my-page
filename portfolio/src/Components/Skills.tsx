import Card, { type CardProps } from "./Card";
import frontend from "@/Assets/Images/frontend.jpg";
import backend from "@/Assets/Images/backend.jpg";
import system from "@/Assets/Images/system.jpg";
import cicd from "@/Assets/Images/cicd.jpg";
import database from "@/Assets/Images/database.jpg";
import customCharacter from "@/Assets/Images/custom-character.webp";
import customBackground from "@/Assets/Images/custom-background.webp";
import customForeground from "@/Assets/Images/custom-foreground.webp";
import whiteBackground from "@/Assets/GIFs/white-background.gif";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import EmptyCard from "./EmptyCard";
import CoolHeader from "./CoolHeader";
import { smallWidth } from "@/Utils/common";
import "@/Styles/Skills.css";
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
                detail: <div className="w-36 text-right"><b>Styling:</b> GSAP, TailwindCSS</div>,
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
                    backgroundSize: "120% auto",
                    backgroundPosition: `60% -25%`,
                }}
            />,
            <p>Hold my coffee. Taming the systems that are built to withstand the blaze.</p>
        ],
        details: [
            {
                x: 132, y: 190, dx: 0, dy: 0,
                detail: <div className="w-46 text-right"><b>Containerization:</b> Docker</div>,
                paths: [[135, 60],[180,210]]
            },
            {
                x: 250, y: 80, dx: 0, dy: 0,
                detail: <div className="w-36 text-left"><b>Auth:</b> JWT, OAuth2, Session-based auth</div>,
                paths: [[0, 140]]
            },
            {
                x: 75, y: 120, dx: 0, dy: 0,
                detail: <div className="w-46 text-right"><b>Servers:</b> Windows Server, Linux Server</div>,
                paths: [[180,100],[-135,30],[180,80]]
            },
            {
                x: 205, y: 190, dx: 0, dy: 0,
                detail: <div className="w-60 text-left"><b>Networking & Infra:</b> Apache2, nginx, Alfahosting, Domain Management, TLS/SSL, VPS, SSH</div>,
                paths: [[0,30],[45,100],[0,120]]
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
                detail: <div className="w-32 text-left"><b>Version Control:</b> Git, GitLab, Azure Repos</div>,
                paths: [[45,22],[0,240]]
            },
            {
                x: 140, y: 90, dx: 0, dy: 0,
                detail: <div className="w-20 text-right"><b>CI/CD:</b> Jenkins, Github Actions</div>,
                paths: [[-135,22], [180,220]]
            },
            {
                x: 85, y: 200, dx: 0, dy: 0,
                detail: <div className="w-22 text-right"><b>Testing:</b> unittest, junit, MSTest</div>,
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
                    backgroundPosition: `62% 0%`,
                    backgroundSize: "120% auto",
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
    {
        title: "JackOfAllTrades",
        level: Infinity,
        colorPresetName: "Wildcard",
        difficulty: Infinity,
        effect: "Omnia",
        attack: Infinity,
        defense: Infinity,
        children: [
            <div className="relative w-full h-full [&>div]:absolute [&>div]:w-full [&>div]:h-full bg-white">
                <div style={{
                    backgroundImage: `url(${whiteBackground})`,
                    opacity: 0.3
                }}/>
                <div className="illustration-background"
                    style={{
                        backgroundImage: `url(${customBackground})`,
                        backgroundPosition: `46% 25%`,
                        backgroundSize: "100% auto",
                        backgroundRepeat: "no-repeat",
                    }}
                />
                <div className="illustration-base"
                    style={{
                        backgroundImage: `url(${customCharacter})`,
                        backgroundPosition: `50% 40%`,
                        backgroundSize: "45% auto",
                        backgroundRepeat: "no-repeat",
                    }}
                />
                <div className="illustration-foreground"
                    style={{
                        backgroundImage: `url(${customForeground})`,
                        backgroundPosition: `50% 30%`,
                        backgroundSize: "80% auto",
                        backgroundRepeat: "no-repeat",
                        filter: "blur(1px)"
                    }}
                />
            </div>,
            <p style={{
                backgroundImage: 'linear-gradient(-30deg,red,orange,yellow,violet,orange,yellow,violet)',
                backgroundClip: 'text',
                color: 'rgba(255, 255, 255, 0.75)'
            }}
            >
                The wildcard. A unique blend of skills that powers the entire deck.
            </p>
        ],
        details: [{
                x: 56, y: 102, dx: 0, dy: 0,
                detail: <div className="w-42 text-right"><b>Digital Art:</b> Clip Studio Paint, Adobe Photoshop, Illustrator, Aseprite</div>,
                paths: [[-135,22],[180,160]]
            },
            {
                x: 140, y: 90, dx: 0, dy: 0,
                detail: <div className="w-42 text-left"><b>Video Production:</b> Adobe Premiere, After Effects, Blender</div>,
                paths: [[-45,22], [0,240]]
            },
            {
                x: 200, y: 190, dx: 0, dy: 0,
                detail: <div className="w-31 text-left"><b>Music Production:</b> FL Studio</div>,
                paths: [[0,60],[45,30],[0,120]]
            },
            {
                x: 40, y: 175, dx: 0, dy: 0,
                detail: <div className="w-40 text-right"><b>Programming:</b> Visual Studio, Visual Studio Code</div>,
                paths: [[90, 20], [135,40],[180,90],[135,60]]
            }
        ],
        init: () => {
            gsap.to(".illustration-background", {
                y: 3,
                duration: 4,
                yoyo: true,
                ease: "sine.inOut",
                repeat: -1
            });
            gsap.to(".illustration-background", {
                scale: 1.01,
                duration: 8,
                transformOrigin: "center center",
                yoyo: true,
                ease: "sine.inOut",
                repeat: -1
            });
            gsap.to(".illustration-base", {
                y: 3,
                duration: 6,
                yoyo: true,
                ease: "sine.inOut",
                repeat: -1,
            });
            gsap.to(".illustration-foreground", {
                y: 2,
                duration: 7,
                yoyo: true,
                ease: "sine.inOut",
                repeat: -1,
            });

            const tl = gsap.timeline({
                repeat: -1,
                repeatDelay: 4,
                onRepeat: () => {
                    gsap.set(".illustration-base", { y: 0, scaleX: 1, skewX: 0 }); // Reset for the next repetition
                }
            });

            tl.to(".illustration-base", {
                duration: 0.05,
                opacity: 0.5,
                y: () => Math.random() * 20 - 10,
                ease: "power1.inOut"
            }, 0)
            .to(".illustration-base", {
                duration: 0.05,
                opacity: 1,
                y: 0,
                scaleX: () => 1 + Math.random() * 0.2, 
                skewX: () => Math.random() * 20 - 10,
                transformOrigin: "center center",
                ease: "power1.inOut"
            }, 0.05)
            .to(".illustration-base", {
                duration: 0.05,
                opacity: 0.7,
                y: () => Math.random() * 20 - 10,
                ease: "power1.inOut"
            }, 0.1)
            .to(".illustration-base", {
                duration: 0.05,
                opacity: 1,
                y: 0,
                scaleX: 1,
                skewX: 0,
                ease: "power1.inOut"
            }, 0.15);
        }
    }
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
        <div className="max-w-200 mx-auto px-10 py-40 space-y-10">
            <h1 className="text-4xl">What I Do</h1>
            <blockquote className="pl-10 text-xl space-y-10 ">
                <h2 className="skill-quote relative text-2xl w-fit pl-4 font-bold">"Jack of all trades, master of none."</h2>
                <p className="italic text-justify">That's what they say. But I see it differently. The true power of synergy lies not in a single skill, but in the creative harmony of all of them. I've honed my abilities across frontend and backend development, DevOps, digital art, and media production to deliver cohesive projects that are expertly produced from every angle.</p>
            </blockquote>
            <i className="text-md text-gray-chalk">*To explore each skill in detail, interact with the cards below*</i>
        </div>
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
                        setSelection(selection => {
                            setCompactSelection(compactSelection => {
                                if (gridRef.current && compactSelection === undefined) {
                                    gsap.to(window, {
                                        duration: 0.2,
                                        scrollTo: {
                                            y: isSmall ? e.currentTarget : gridRef.current,
                                            offsetY: isSmall ? window.innerHeight / 2 - 200 : 75,
                                            autoKill: true
                                        },
                                        ease: "expo.out"
                                    });
                                }
                                return undefined;
                            });
                            return index;
                        })
                    }}
                    onDetailCallback={(toggle) => {
                        setCompactSelection(toggle ? index : undefined)
                    }}
                    compactSelected={compactSelection === index}
                />
                : <EmptyCard key={index}/>
            )}
        </div>
    </section>);
}