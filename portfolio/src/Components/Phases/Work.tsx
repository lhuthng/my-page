import monkee from "@/Assets/GIFs/working.webp";

export default function Work() {
    return (<div className="flex flex-col h-full justify-normal md:justify-center gap-6">
        <span>After completing a compulsory internship module, I received an offer to stay on as a full-time Programmer Analyst at DXC Technology, an outsourcing firm. What began as a short assignment turned into an opportunity to dive deeper into real-world software development.</span>
        <span>Across a range of client projects, I honed my technical skills, learned to adapt to new domains quickly, and discovered how collaboration shapes better solutions. Each project felt like a new puzzle - sometimes tricky, often complex, but always rewarding to solve.</span>
        <figure className='opacity-0 animate-[fadeIn_0.5s_ease-in_forwards] relative h-50 my-2 sm:my-6 [&>img]:absolute [&>img]:w-auto [&>img]:rounded-2xl [&>img]:transition-all [&>img]:duration-1000'>
            <img className='hidden xs:block h-1/3 left-1/2 sm:left-1/5 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-40' src={monkee}></img>
            <img className='hidden xs:block h-1/3 left-1/2 sm:left-4/5 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-40' src={monkee}></img>
            <img className='hidden xs:block h-2/3 left-1/2 sm:left-2/6 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-80' src={monkee}></img>
            <img className='hidden xs:block h-2/3 left-1/2 sm:left-4/6 top-1/2 -translate-x-1/2 -translate-y-1/2 opacity-80' src={monkee}></img>
            <img className='w-full xs:w-auto h-auto xs:h-full left-1/2 -translate-x-1/2 top-0' src={monkee}></img>
            <figcaption className='mx-auto pt-52 text-center text-base xs:texl-lg sm:text-xl'>Fig.3 - Me fixing one bug and creating five more.</figcaption>
        </figure>
    </div>)
}