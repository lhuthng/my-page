import { useGSAP } from "@gsap/react";
import About from "./Components/About";
import Contact from "./Components/Contact";
import Experience from "./Components/Experience";
import Header from "./Components/Header";
import Landing from "./Components/Landing";
import Skills from "./Components/Skills";
import "./index.css";
import { gsap } from "gsap";
import { Flip } from "gsap/all";
import { ScrollToPlugin } from "gsap/ScrollToPlugin";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Projects from "./Components/Projects";
import { Draggable } from "gsap/Draggable";
import TurbelenceFilter from "./Components/TurbelenceFilter";

gsap.registerPlugin(Flip);
gsap.registerPlugin(ScrollToPlugin);
gsap.registerPlugin(ScrollTrigger);
gsap.registerPlugin(Draggable);
gsap.registerPlugin(useGSAP);

export function App() {
  return (
    <>
      <svg className="absolute" width="0" height="0">
        <defs>
          {Array.from({ length: 5 }, (_, index) => (
            <TurbelenceFilter key={index} id={index} />
          ))}
        </defs>
        <filter id="grainy">
          <feTurbulence
            type="fractalNoise"
            baseFrequency="3"
            numOctaves="3"
            stitchTiles="stitch"
          />
        </filter>
      </svg>
      <div id="modal-root" />
      {/* <Header sections={["Home", "About", "Skiplls", "Projects", "Contact"]}/> */}
      <div className="bg-blackboard text-white w-full divide-y space-y-2">
        <Landing />
        <About />
        <Experience />
        <Skills />
        <Projects />
        <Contact />
      </div>
    </>
  );
}

export default App;
