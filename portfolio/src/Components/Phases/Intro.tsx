import arrow from '@/Assets/SVGs/arrow-1.svg';
import { useEffect } from 'react';



export default function Intro(
    { setSpecialDiv }
    : { setSpecialDiv: React.Dispatch<React.SetStateAction<React.ReactNode>> }
) {
    useEffect(() => {
        setSpecialDiv(<img className="absolute left-[5rem] sm:left-auto right-auto sm:right-[1rem] bottom-[4rem] sm:bottom-[8rem] w-20 h-auto rotate-35 sm:-rotate-15 animate-[breath_0.75s_ease-in_infinite]" src={arrow}/>);
    }, [setSpecialDiv]);


    return (<section className="flex flex-col gap-6">
        <p>I'm a full-stack developer passionate about building projects from the ground up. I specialize in software design, with a keen interest in microservices and scalable architectures. I enjoy turning ideas into efficient, maintainable solutions that can grow with users' needs.</p>
        <p>If you'd like a quick look at my journey so far, you can explore the interactive timeline below.</p>
    </section>);
}