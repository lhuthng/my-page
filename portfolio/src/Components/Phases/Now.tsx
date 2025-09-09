import hiking from '@/Assets/Images/hiking.webp'
import confetti from '@/Assets/Lotties/confetti.json'
import '@/Styles/Now.css'
import Lottie from '../Lottie'

export default function Now() {
    return (<section className='flex flex-col sm:flex-row-reverse w-full h-full gap-6'>
        <h1 className="block sm:hidden text-2xl text-center">Now</h1>
        <div className='relative min-w-auto sm:min-w-[clamp(18rem,100dvw-30rem,25rem)] h-80 sm:h-full sm:my-auto'>
            <div className='absolute sm:-left-[4rem] w-full sm:w-[calc(100%+4rem)] h-full bg-cover xs:bg-[length:150%] sm:bg-[length:200%] bg-[45%_27%] sm:bg-[40%_50%] bg-no-repeat bg-[url(@/Assets/Images/hiking.webp)] rounded-md [mask-image:linear-gradient(to_bottom,black,black_60%,transparent)] sm:[mask-image:linear-gradient(to_left,black,black_10%,transparent)]'></div>
        </div>
        <div className='relative my-auto'>
            <p>Now, as a recent graduate, I'm eager to apply my skills to real-world challenges. My academic projects have prepared me to contribute confidently to development teams, and I'm actively seeking opportunities to grow, learn, and make an impact.</p>
            <span className='absolute w-full sm:w-160 h-auto sm:h-100 left-1/2 -translate-x-1/2 bottom-0 overflow-hidden'>
                <Lottie className='w-full h-auto opacity-75 pointer-none' animationData={confetti} loop={true} speed={0.75}/>
            </span>
            
        </div>
    </section>)
}