import arrow from '@/Assets/SVGs/arrow-1.svg';
import { useEffect } from 'react';



export default function Intro(
    { setSpecialDiv, onClick }
    : { 
        setSpecialDiv: React.Dispatch<React.SetStateAction<React.ReactNode>>,
        onClick: () => void
    }
) {
    useEffect(() => {
        setSpecialDiv(
            <div className='absolute w-[min(100%,75rem)] h-0 left-1/2 -translate-x-1/2 bottom-0'>
                <img className="absolute left-[5rem] sm:left-[calc(100%-7.5rem)] bottom-20   sm:bottom-40 w-20 h-auto rotate-35 sm:-rotate-15 animate-[breath_0.75s_ease-in_infinite] cursor-pointer" 
                    onClick={onClick}
                    src={arrow}
                />
            </div>
        );
    });


    return (<section className="flex flex-col h-full justify-center gap-6">
        <h1 className="text-2xl sm:text-3xl text-center">Me in a nutshell</h1>
        <p>I'm a full-stack developer passionate about building projects from the ground up. I specialize in software design, with a keen interest in microservices and scalable architectures. I enjoy turning ideas into efficient, maintainable solutions that can grow with users' needs.</p>
        <p>If you'd like a quick look at my journey so far, you can explore the interactive timeline below.</p>
    </section>);
}