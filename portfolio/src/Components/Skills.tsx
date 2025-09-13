import Card, { type CardProps } from "./Card";
import frontend from "@/Assets/Images/frontend.jpg";
import backend from "@/Assets/Images/backend.jpg";
import system from "@/Assets/Images/system.jpg";
import cicd from "@/Assets/Images/cicd.jpg";
import database from "@/Assets/Images/database.jpg";
import { useRef, useState } from "react";

export default function Skills() {
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
            ]
        },
        {
            title: "Systems",
            level: 2,
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
            ]
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
            ]
        },        
    ];

    const [ selection, setSelection ] = useState<number | null>(null);
    const gridRef = useRef<HTMLSelectElement>(null);

    return (<section className="grid-container max-w-340 mx-auto" ref={gridRef}>
        {selection !== null && cards[selection] && <Card {...cards[selection]} expanded={true}/>}
        {cards.map((props, index) => selection !== index && <Card {
            ...props
        }
            onClick={() => {
                setSelection(index);
                if (!gridRef.current) return;
                const elementPosition = gridRef.current.getBoundingClientRect().top;
                window.scrollTo({
                    top: window.pageYOffset + elementPosition - 80,
                    behavior: "smooth"
                });
            }}
        />)}
    </section>);
}