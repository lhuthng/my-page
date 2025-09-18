import CoolHeader from "./CoolHeader";
import '@/Styles/Experience.css';
import ExperienceInfo from "./WorkExperience";

export default function Experience() {
    return (<section className="max-w-340 mx-auto">
        <div className="max-w-200 mx-auto">
            <CoolHeader title="Experience & Education" />
            <div className="grid-flex space-x-2">
                <ExperienceInfo
                    title="Frontend Freelancer"
                    duration="duration"
                    organization="organization"
                    location="location"
                    description="description"
                />
                <ExperienceInfo
                    title="title"
                    duration="duration"
                    organization="organization"
                    location="location"
                    description="description"
                />
            </div>
        </div>
    </section>);
}