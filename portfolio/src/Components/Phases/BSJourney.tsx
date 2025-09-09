import monkee from '@/Assets/GIFs/reading.webp'
import nerd from '@/Assets/Lotties/nerd.json'
import Lottie from '../Lottie'


export default function BSJourney() {
    return (<section className="flex flex-col lg:flex-row gap-6 sm:justify-center h-full">
        <h1 className="block sm:hidden text-2xl text-center">B.Sc Journey</h1>
        <div className='flex flex-col sm:flex-1 h-full justify-evenly sm:justify-center gap-6 lg:my-auto lg:gap-10'>
            <span>To build on my early passion, I pursued a B.Sc in Computer Science. Those years gave me the chance to dive deeper into programming, learn core concepts of computing, and work on more complex problems. I also spent a semester in Germany through a DAAD program, which was a valuable part of my journey.</span>
            <span>During my studies <Lottie className="inline-block w-7 h-7 translate-y-1" animationData={nerd} loop={true}/>, I also developed practical software engineering skills and worked on projects that prepared me to be job-ready and confident in contributing to real development teams.</span>
        </div>
        <figure className='mx-auto my-2 sm:my-auto space-y-2'>
            <img className='max-h-40 sx:max-h-50 sm:max-h-60 rounded-2xl bg-red' src={monkee}></img>
            <figcaption className='text-center text-base xs:texl-lg sm:text-xl'>Fig.2 - Me learning hard.</figcaption>
        </figure>
    </section>)
}