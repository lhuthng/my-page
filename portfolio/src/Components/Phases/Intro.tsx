import arrow from '@/Assets/SVGs/arrow-1.svg';
import { useEffect } from 'react';
import evolution from '@/Assets/SVGs/evolution.svg';


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
                <img className="absolute left-6 sm:left-[calc(100%-8.5rem)] bottom-16 sm:bottom-25 w-20 h-auto -rotate-25 sm:rotate-35 scale-50 sm:-scale-x-100 sm:scale-y-100 animate-[breath_0.75s_ease-in_infinite] cursor-pointer"
                    onClick={onClick}
                    src={arrow}
                />
            </div>
        );
    });


    return (<section className="relative flex flex-col min-h-full justify-start gap-6">
        <div className='relative rounded-md overflow-hidden w-full sm:w-4/5 mx-auto h-45 sm:h-30'>
            <div className='absolute w-full min-h-full h-30 sm:h-full bg-cover not-sm:translate-y-[-20%] bg-[53%_40%] sm:bg-[60%_40%] bg-no-repeat bg-[url(@/Assets/Images/me-wide.jpg)]'
                style={{
                    maskImage: 'linear-gradient(to left, transparent,transparent 12%, black, transparent 88%, transparent)'
                }}
            />
        </div>
        <h1 className="text-2xl sm:text-3xl text-center">Me in a nutshell</h1>
        <p>I'm a full-stack developer passionate about building projects from the ground up. I specialize in software design, with a keen interest in microservices and scalable architectures. I enjoy turning ideas into efficient, maintainable solutions that can grow with users' needs.</p>
        <figure className='max-w-full w-60 sm:w-100 mx-auto space-y-2'>
            <div className="w-full h-20 animate-[scrollingBackground_20s_linear_infinite]"
                style={{
                    maskImage: `url(${evolution})`,
                    maskRepeat: "no-repeat",
                    maskPosition: "center",
                    backgroundImage: `linear-gradient(to right, var(--color-yellow-chalk), var(--color-orange-chalk), var(--color-green-chalk), var(--color-blue-chalk), var(--color-purple-chalk), var(--color-yellow-chalk), var(--color-orange-chalk), var(--color-green-chalk), var(--color-blue-chalk), var(--color-purple-chalk))`,
                    backgroundRepeat: "repeat-x",
                    backgroundSize: "200% 100%"
                }}
            />
            <figcaption className='text-center text-base xs:texl-lg sm:text-xl'>Fig. 0: My journey to be a developer.</figcaption>
        </figure>
        <p className='text-sm text-right sm:text-center italic text-gray-chalk'>*If you'd like a quick look at my journey so far, you can explore the timeline <span className='hidden sm:inline'>below</span><span className='sm:hidden'>on the left</span>.*</p>
    </section>);
}