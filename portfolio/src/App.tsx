import About from "./Components/About";
import Contact from "./Components/Contact";
import Experience from "./Components/Experience";
import Header from "./Components/Header";
import Landing from "./Components/Landing";
import Skills from "./Components/Skills";
import "./index.css";
import { gsap } from 'gsap';
import { ScrollToPlugin } from 'gsap/ScrollToPlugin';

gsap.registerPlugin(ScrollToPlugin);

export function App() {
  return (
    <>
      <Header sections={["Home", "About", "Skills", "Projects", "Contact"]}/>
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
