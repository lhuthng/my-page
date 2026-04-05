export default function MSAdventure() {
  return (
    <section className="flex flex-col min-h-full justify-evenly sm:justify-center gap-6">
      <h1 className="block sm:hidden text-2xl text-center">M.Sc Adventure</h1>
      <span>
        After a year in industry, I felt the pull back toward the deeper end of
        computer science. The Master's program gave me space to slow down — and
        go further into the math, the theory, and the kinds of problems that
        don't always have a Stack Overflow answer.
      </span>
      <span className="flex">
        <span className="hidden sm:block">
          Some areas that genuinely keep me up at night:
          <ul className="pl-12" style={{ listStyleType: "circle" }}>
            <li>Computer Algebra</li>
            <li>Game Theory</li>
            <li>Coding Theory</li>
            <li>Software Analysis</li>
            <li>Hardware-oriented Security</li>
          </ul>
        </span>
        <span className="block sm:hidden">
          Things that keep me up at night: Computer Algebra, Game Theory, Coding
          Theory, Software Analysis, and Hardware-oriented Security.
        </span>
      </span>
      <span>
        Industry experience keeps things grounded; academia keeps things
        interesting. The two make for a better combination than either alone.
      </span>
    </section>
  );
}
