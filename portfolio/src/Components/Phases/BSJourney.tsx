import monkee from "@/Assets/GIFs/reading.webp";
import nerd from "@/Assets/Lotties/nerd.json";
import Lottie from "../Lottie";

export default function BSJourney() {
  return (
    <section className="flex flex-col lg:flex-row gap-6 sm:justify-center min-h-full">
      <h1 className="block sm:hidden text-2xl text-center">B.Sc Journey</h1>
      <div className="flex flex-col sm:flex-1 h-full justify-evenly sm:justify-center gap-6 lg:my-auto lg:gap-10">
        <span>
          Competitive programming gave me the spark, and university gave me the
          depth. My B.Sc in Computer Science took me from contest solutions to
          real software — algorithms, systems, theory, and everything in
          between. A DAAD scholarship also brought me to Germany for a semester,
          which turned out to be one of the more eye-opening parts of my
          education.
        </span>
        <span>
          Those years{" "}
          <Lottie
            className="inline-block w-7 h-7 translate-y-1"
            animationData={nerd}
            loop={true}
          />{" "}
          weren't just about learning concepts — they were about learning how to
          build. Team projects, coursework, and late-night debugging sessions
          turned theory into something I could actually ship.
        </span>
      </div>
      <figure className="mx-auto my-2 sm:my-auto space-y-2">
        <img
          className="max-h-40 sx:max-h-50 sm:max-h-60 rounded-2xl bg-red"
          src={monkee}
        ></img>
        <figcaption className="text-center text-base xs:texl-lg sm:text-xl">
          Fig.2 - Me learning hard.
        </figcaption>
      </figure>
    </section>
  );
}
