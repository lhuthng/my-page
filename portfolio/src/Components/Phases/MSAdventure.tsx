export default function MSAdventure() {
    return (<section className="flex flex-col min-h-full justify-evenly sm:justify-center gap-6">
        <h1 className="block sm:hidden text-2xl text-center">M.Sc Adventure</h1>
        <span>After a year in the professional world, I pursued a Master's in Computer Science to deepen my knowledge of algorithms, mathematical modeling, IT security, and software engineering.</span>
        <span className="flex">
            <span className="hidden sm:block">
                My current research interests include:
                <ul className="pl-12" style={{listStyleType: "circle"}}>
                    <li>Computer Algebra</li>
                    <li>Game Theory</li>
                    <li>Coding Theory</li>
                    <li>Software Analysis</li>
                    <li>Hardware-oriented Security</li>
                </ul>
            </span>
            <span className="block sm:hidden">My current research interest include Computer Algebra, Game Theory, Coding Theory, Software Analysis, and Hardware-oriented Security.</span>
        </span>
        <span>This journey combines practical experience with advanced theoretical understanding, driving my growth as a researcher and problem solver.</span>
    </section>);
}