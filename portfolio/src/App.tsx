import About from "./Components/About";
import Contact from "./Components/Contact";
import Header from "./Components/Header";
import Landing from "./Components/Landing";
import "./index.css";

import logo from "./logo.svg";
import reactLogo from "./react.svg";

export function App() {
  return (
    <>
      <Header sections={["Home", "About", "Skills", "Projects", "Contact"]}/>
      <div className="bg-blackboard text-white w-full divide-y divide-gray-300 z-10">
        <Landing/>
        <About/>
        <Contact/>
      </div>
    </>
  );
}

export default App;
