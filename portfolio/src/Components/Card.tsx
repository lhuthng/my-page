import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import "@/Styles/Card.css"
import CardInfo, { type CardInfoProps } from "./CardInfo";

const colors = [ "white", "black", "dark-charcoal", "yellow", "orange", "blue", "cyan", "navi", "green", "aquamarine", "lime", "pink", "purple", "silver", "light-gray", "orange-red", "salmon" ] as const;
type Color = typeof colors[number]

const toRGB = (color: Color, alpha: number = 1) => `rgba(var(--${color}-chalk), ${alpha})`;

interface CardPreset {
    title: Color
    background: Color
    description: Color
    text: Color
    highlight?: Color,
    textAlt?: Color
}

const defaultPreset: CardPreset = {
    title: "orange",
    background: "yellow",
    description: "orange",
    text: "black",
};

const attributes = [ "Resilience", "Agility", "Intelligent", "Charisma", "Wisdom" ] as const;
type Attribute = typeof attributes[number];

const cardPresetBank: Partial<Record<
    Attribute, 
    CardPreset | null
>> = {
    Resilience: {
        title: "orange",
        background: "yellow",
        description: "orange",
        text: "dark-charcoal",
    },  
    Intelligent: {
        title: "blue",
        background: "cyan",
        description: "blue",
        highlight: "navi",
        text: "white",
        textAlt: "blue",
    },
    Agility: {
        title: "aquamarine",
        background: "green",
        description: "aquamarine",
        text: "white"
    },
    Charisma: {
        title: "purple",
        background: "pink",
        description: "purple",
        text: "black",
    }, 
    Wisdom: {
        title: "light-gray",
        background: "silver",
        description: "light-gray",
        text: "black",
    }
}

export interface CardProps {
    title: string,
    level: number,
    colorPresetName: Attribute,
    difficulty: number,
    attack?: number,
    defense?: number,
    effect?: string,
    expanded?: boolean,
    details: CardInfoProps[],
    onClick?: () => void,
    children: [
        illustration: ReactNode, 
        description: ReactNode
    ]
}

export default function Card(
    { 
        title,
        level,
        colorPresetName,
        difficulty,
        attack,
        defense,
        effect,
        expanded,
        details,
        onClick,
        children
    }: CardProps
) {
    const cardRef = useRef<HTMLDivElement>(null);
    const illustrationRef = useRef<HTMLDivElement>(null);
    const detailRef = useRef<HTMLDivElement>(null);
    const [ cardOffset, setCardOffset ] = useState<[number, number]>([0, 0]);

    const [ illustration, description ] = children;
    const colorPreset = cardPresetBank[colorPresetName] ?? defaultPreset;

    const handleMouseMove = (e: React.MouseEvent<HTMLDivElement, MouseEvent>) => {
        if (!cardRef.current) return;

        const rect = cardRef.current.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        const centerX = rect.width / 2;
        const centerY = rect.height / 2;

        const dirX = (x - centerX);
        const dirY = -(y - centerY);

        const deg = 16 * Math.sqrt(dirX * dirX + dirY * dirY) / rect.height;

        cardRef.current.style.transform = `rotate3d(${dirY}, ${dirX}, 0, ${deg}deg)`;

        if (!illustrationRef.current) return;

        illustrationRef.current.style.transform = `translate(${-dirX / 40}px, ${(dirY-16) / 40}px)`;
    };

    const handleMouseLeave = () => {
        if (!cardRef.current) return;
        cardRef.current.style.transform = "";
        if (!illustrationRef.current) return;
        illustrationRef.current.style.transform = "";
    };

    const cardContainer = <div className="card-container perspective-midrange mx-auto my-2 w-70 h-100 cursor-pointer"
        onMouseMove={handleMouseMove}
        onMouseEnter={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        onClick={onClick}
    >  
        <div className="w-full h-full font-cabin-sketch"
            ref={cardRef}
        >
            <div className="level flex flex-col absolute -left-2 -top-1 w-18 h-18 justify-center items-center border-4 pointer-events-none rounded-xl z-1"
                style={{
                    backgroundColor: toRGB(colorPreset.background),
                    borderColor: toRGB(colorPreset.highlight ?? colorPreset.title),
                    color: toRGB(colorPreset.textAlt ?? colorPreset.text)
                }}
            >
                <span className="text-3xl font-medium h-6">{level}</span>
                <span className="text-[0.5rem]">yrs</span>
            </div>
            <div className="card flex flex-col w-full h-full rounded-2xl border-8 border-double border-blackboard transition-shadow duration-100 overflow-hidden"
                style={{
                    borderColor: toRGB(colorPreset.highlight ?? "black"),
                    backgroundColor: toRGB(colorPreset.background)
                }}
            >
                <div className="relative w-full h-7 text-darkboard text-xl">
                    <div className="absolute flex left-0 top-0 w-full h-13 items-center z-2"
                        style={{
                            backgroundImage: `linear-gradient(to bottom, ${toRGB(colorPreset.title)} 40px, transparent 12px)`,
                            maskImage: `linear-gradient(to bottom, black 28px, rgba(0,0,0,0.6) 120%)`,
                            color: toRGB(colorPreset.text)
                        }}
                    >
                        <span className="pl-16 font-bold z-1">
                            {title}
                        </span>
                        <div className="absolute left-0 top-[calc(100%-24px)]">
                            <div className="absolute left-16 top-1.5 text-xs">
                                {'☆'.repeat(difficulty)}
                            </div>
                            <svg className="w-full h-6 z-1">
                                <path 
                                    d="M0 0 L212 0 L188 24 L0 24 Z"
                                    fill={toRGB(colorPreset.title)}
                                />
                            </svg>
                        </div>
                    </div>
                </div>
                <div className="relative flex flex-col flex-1 justify-end overflow-hidden z-1"
                    style={{
                        transformStyle: "preserve-3d"
                    }}
                >
                    <div className="absolute left-1/2 top-1/2 -translate-1/2 w-[120%] h-[120%] z-2"
                        ref={illustrationRef}
                    >
                        {illustration}
                    </div>
                    <div className="relative w-full h-43 text-lg z-3"
                        style={{
                            backgroundImage: `linear-gradient(to top, ${toRGB(colorPreset.description)} 160px, transparent 12px)`,
                            maskImage: 'linear-gradient(to top, black 130px, rgba(0,0,0,0.6) 120%)',
                            color: toRGB(colorPreset.text)
                        }}
                    >
                        <div className="absolute left-0 bottom-[calc(100%-24px)]">
                            <div className="absolute right-0 m-1 text-xs">
                                {`[${colorPresetName} ${effect ? "/ " + effect : ""} / Effect]`}
                            </div>
                            <svg className="w-full h-6 -z-1">
                                <path
                                    d="M270 0 L92 0 L68 24 L270 24 Z"
                                    fill={toRGB(colorPreset.description)}
                                />
                            </svg>
                        </div>
                        <div className="flex flex-col h-full pt-8">
                            <div className="flex-1 px-2">
                                {description}
                            </div>
                            <div className="flex w-full h-4 text-xs justify-between px-2"
                                style={{
                                    backgroundColor: toRGB(colorPreset.background),
                                    color: toRGB(colorPreset.textAlt ?? colorPreset.text)
                                }}
                            >
                                <span>1<sup>st</sup> Edition</span>
                                {attack && defense && <span>atk/{attack} def/{defense}</span>}
                            </div>
                        </div>  
                    </div>
                </div>
            </div>
        </div>
    </div>;

    const findCardOffset = () => {
        if (detailRef.current && cardRef.current) {
            const [ detailRect, cardRect ] = [
                detailRef.current.getBoundingClientRect(),
                cardRef.current.getBoundingClientRect()
            ]

            setCardOffset([detailRect.left - cardRect.left, detailRect.top - cardRect.top]);
        }
    }

    useLayoutEffect(() => {
        findCardOffset();
    }, [ detailRef ]);

    useEffect(() => {
        window.addEventListener('resize', findCardOffset);
        return () => {
            window.removeEventListener('resize', findCardOffset);
        }
    }, [])

    return !expanded ? 
        cardContainer
        : <div className="flex relative justify-center col-span-full h-auto">
            {cardContainer}
            <div className="absolute"
                ref={detailRef}
            >
                {details.map((detail, index) => <CardInfo {...detail}
                    className="rounded-lg p-2 border-2"
                    style={{
                        backgroundColor: toRGB(colorPreset.background),
                        borderColor: toRGB(colorPreset.highlight ?? colorPreset.title),
                        color: toRGB(colorPreset.textAlt ?? colorPreset.text),
                    }}
                    dx={cardOffset[0]} 
                    dy={cardOffset[1]} 
                    key={index}
                    strokeColor={toRGB(colorPreset.highlight ?? colorPreset.title)}
                />)}
            </div>
        </div>
}