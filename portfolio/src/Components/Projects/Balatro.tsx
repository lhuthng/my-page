import ProjectTemplate from "./ProjectTemplate";
import balatroDemo from "@/Assets/Images/balatro.webp";

export default function Balatro() {
    return <ProjectTemplate 
        illustration={
            <div
                className="w-full h-full"
                style={{
                    backgroundImage: `url(${balatroDemo})`,
                    backgroundPosition: "center center",
                    backgroundSize: "cover"
                }}
            >
            </div>
        }
        description={
            <div></div>
        }
        details={
            <div></div>
        }
    />
}