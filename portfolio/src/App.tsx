import About from "./Components/About";
import Contact from "./Components/Contact";
import Header from "./Components/Header";
import Landing from "./Components/Landing";
import Skills from "./Components/Skills";
import "./index.css";

export function App() {
  return (
    <>
      <Header sections={["Home", "About", "Skills", "Projects", "Contact"]}/>
      <div className="bg-blackboard text-white w-full divide-y space-y-2">
        <Landing/>
        <About/>
        <Skills/>
        <Contact/>
      </div>
    </>
  );
}

export default App;
