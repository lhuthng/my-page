import ProjectTemplate from "./ProjectTemplate";

export default function ThisPortfolio() {
  return (
    <ProjectTemplate
      active={false}
      onClick={() => {}}
      illustration={<div className="w-full h-full bg-red-500"></div>}
      description={<div></div>}
      details={<div></div>}
    />
  );
}
