export default function MSAdventure() {
    return (<section className="flex flex-col h-full justify-center gap-6">
        <span>After a year of professional experience, I decided to continue my journey by pursuing a Master's degree in Computer Science. During this program, I have deepened my knowledge in algorithms, mathematical modeling, IT security, and software engineering.</span>
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
        <span>This journey allows me to combine practical experience with advanced theoretical understanding, driving my growth as a researcher and problem solver in the field of computer science.</span>
    </section>);
}