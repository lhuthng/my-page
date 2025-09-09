import monkee from '@/Assets/GIFs/practicing.webp';
import trophy from "@/Assets/Lotties/trophy.json";
import Lottie from '../Lottie';

export default function FirstEncounter() {
    return (
        <section className="flex flex-col h-full justify-evenly text-bg-white-chalk">
            <h1 className="block sm:hidden text-2xl text-center">First Encounter</h1>
            <span>I discovered my love for programming in middle school, drawn to the mix of logic and creativity. Pascal was my first language, and writing code felt like solving puzzles - each program a small win.</span>
            <figure className='mx-auto my-2 sm:my-6 space-y-2'>
                <img className='max-h-80 rounded-2xl bg-red' src={monkee}></img>
                <figcaption className='text-center text-base xs:texl-lg sm:text-xl'>Fig.1 - Me solving "puzzles".</figcaption>
            </figure>
            <span>This love for coding led me to competitive programming. I spent years sharpening my skills in Pascal and tackling tricky problems. Competing in contests was exciting, and every prize I earned made me even more passionate about coding <Lottie className="inline-block w-7 h-7 translate-y-1" animationData={trophy} loop={true}/>.</span>
        </section>
    );
}