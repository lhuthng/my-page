import monkee from "@/Assets/GIFs/practicing.webp";
import trophy from "@/Assets/Lotties/trophy.json";
import Lottie from "../Lottie";

export default function FirstEncounter() {
  return (
    <section className="flex flex-col min-h-full justify-evenly text-bg-white-chalk">
      <h1 className="block sm:hidden text-2xl text-center">First Encounter</h1>
      <span>
        So... it started in middle school — a dusty computer lab and{" "}
        <code className="font-courier-prime text-yellow-chalk bg-navi-chalk">
          Pascal
        </code>
        . Not the most glamorous origin story, but the moment my first program
        ran correctly, something clicked. Logic, creativity, a dash of
        stubbornness — it turned out coding had all of them.
      </span>
      <figure className="mx-auto my-2 sm:my-6 space-y-2">
        <img className="max-h-80 rounded-2xl" src={monkee}></img>
        <figcaption className="text-center text-base xs:texl-lg sm:text-xl">
          Fig.1 - Me solving "puzzles".
        </figcaption>
      </figure>
      <span>
        That early spark pulled me into competitive programming — years of
        hunting edge cases, optimizing algorithms under contest pressure, and
        occasionally staring at a wrong-answer verdict for way too long. Every
        trophy{" "}
        <Lottie
          className="inline-block w-7 h-7 translate-y-1"
          animationData={trophy}
          loop={true}
        />{" "}
        just made the next problem feel worth solving.
      </span>
    </section>
  );
}
