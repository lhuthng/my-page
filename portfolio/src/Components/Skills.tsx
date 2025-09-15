import Card, { type CardProps } from "./Card";
import frontend from "@/Assets/Images/frontend.jpg";
import backend from "@/Assets/Images/backend.jpg";
import system from "@/Assets/Images/system.jpg";
import cicd from "@/Assets/Images/cicd.jpg";
import database from "@/Assets/Images/database.jpg";
import { useRef, useState } from "react";
import EmptyCard from "./EmptyCard";
import CoolHeader from "./CoolHeader";
import CardInfo from "./CardInfo";

export default function Skills() {
    const cards: CardProps[] = [
        {
            title: "Frontend",
            level: 1,
            colorPresetName: "Charisma",
            difficulty: 1,
            attack: 1000,
            defense: 1000,
            details: [],
            children: [
                <div className="w-full h-full"
                    style={{
                        backgroundImage: `linear-gradient(rgba(255,192,203,0.1), rgba(255,192,203,0.1)), url(${frontend})`,
                        backgroundPosition: `50% 20%`,
                    }}
                />,
                <p>Raising sleek, interactive UIs from tiny ideas. Bringing up every pixel with care.</p>
                // <div>
                //     <ul>
                //         <li><b>Core:</b> HTML/CSS/JS, TypeScript</li>
                //         <li><b>Frameworks:</b> React, Angular, Razor Page, Svelte 5</li>
                //         <li><b>Styling:</b> TailwindCSS</li>
                //     </ul>
                // </div>
            ]
        },
        {
            title: "Backend",
            level: 3,
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
                // <div>
                //     <ul>
                //         <li><b>Languages:</b> Node.js, Go, C#, Rust, C++, PHP</li>
                //         <li><b>Frameworks:</b> Express, Tonic, ASP.NET Core</li>
                //         <li><b>Communication:</b> WebSocket, REST, gRPC, JSON-RPC</li>
                //     </ul>
                // </div>
            ],
            details: [
                {
                    x: 220, y: 80, dx: 0, dy: 0,
                    detail: <div className="w-46 text-left"><b>Languages:</b> Node.js, Go, C#, Rust, C++, PHP</div>,
                    paths: [[45, 60],[0,210]]
                },
                {
                    x: 130, y: 85, dx: 0, dy: 0,
                    detail: <div className="w-41 text-right"><b>Frameworks:</b> Express, Tonic, ASP.NET</div>,
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
            level: 2,
            colorPresetName: "Resilience",
            difficulty: 3,
            attack: 1000,
            defense: 2000,
            details: [],
            children: [
                <div className="w-full h-full"
                    style={{
                        backgroundImage: `url(${system})`,
                        backgroundPosition: `50% 12%`,
                    }}
                />,
                <p>Hold my coffee. Taming the systems that are built to withstand the blaze.</p>
                // <div>
                //     <ul>
                //         <li><b>Containerization:</b> Docker</li>
                //         <li><b>Servers:</b> Windows Server, Linux Server</li>
                //         <li><b>Networking & Infra:</b> Apache2, nginx, Alfahosting, Domain Management, TLS/SSL, VPS, SSH</li>
                //     </ul>
                // </div>
            ]
        },
        {
            title: "DevOps",
            level: 0.5,
            colorPresetName: "Agility",
            difficulty: 2,
            attack: 500,
            defense: 1500,
            effect: "Fusion",
            details: [],
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
                // <div>
                //     <ul>
                //         <li><b>Version Control:</b> Git. GitLab, Azure Repos</li>
                //         <li><b>CI/CD:</b> Jenkins, Github Actions</li>
                //         <li><b>Testing:</b> unittest, junit, MSTest</li>
                //     </ul>
                // </div>
            ]
        },
        {
            title: "Database",
            level: 1,
            colorPresetName: "Wisdom",
            difficulty: 1,
            effect: "Power",
            details: [],
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
                // <div>
                //     <ul>
                //         <li><b>Version Control:</b> Git. GitLab, Azure Repos</li>
                //         <li><b>CI/CD:</b> Jenkins, Github Actions</li>
                //         <li><b>Testing:</b> unittest, junit, MSTest</li>
                //     </ul>
                // </div>
            ]
        },        
    ];

    const [ selection, setSelection ] = useState<number | null>(null);
    const gridRef = useRef<HTMLDivElement>(null);

    return (
    <section className="max-w-340 mx-auto">
        <CoolHeader title="Skills" />
        <div className="grid-container w-full" ref={gridRef}>
            {selection !== null && cards[selection] && <Card {...cards[selection]} expanded={true}/>}
            {cards.map((props, index) => selection !== index ? <Card {
                    ...props
                }
                    key={index}
                    onClick={() => {
                        setSelection(index);
                        if (!gridRef.current) return;
                        const elementPosition = gridRef.current.getBoundingClientRect().top;
                        window.scrollTo({
                            top: window.pageYOffset + elementPosition - 80,
                            behavior: "smooth"
                        });
                    }}
                />
                : <EmptyCard key={index}/>
            )}
        </div>
    </section>);
}