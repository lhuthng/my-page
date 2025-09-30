import { useGSAP } from "@gsap/react";
import About from "./Components/About";
import Contact from "./Components/Contact";
import Experience from "./Components/Experience";
import Header from "./Components/Header";
import Landing from "./Components/Landing";
import Skills from "./Components/Skills";
import "./index.css";
import { gsap } from 'gsap';
import { Flip } from "gsap/all";
import { ScrollToPlugin } from 'gsap/ScrollToPlugin';
import { ScrollTrigger } from "gsap/ScrollTrigger";

gsap.registerPlugin(Flip);
gsap.registerPlugin(ScrollToPlugin);
gsap.registerPlugin(ScrollTrigger);
gsap.registerPlugin(useGSAP);

export function App() {
  return (
    <>
      {/* <Header sections={["Home", "About", "Skiplls", "Projects", "Contact"]}/> */}
      <div className="bg-blackboard text-white w-full divide-y space-y-2">
        <Landing/>
        <About/>
        <Experience />
        <Skills/>
        <Contact/>
      </div>
    </>
  );
}

export default App;
