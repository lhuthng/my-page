interface ExperienceInfoProps {
    title: string,
    duration: string
    organization: string,
    location: string,
    description: string
}

export default function ExperienceInfo({
    title, duration, organization, location, description
}: ExperienceInfoProps) {
    return (<div className="flex flex-col border-2 p-4 w-full">
        <h1>{title}</h1>
        <span>{organization}</span>
        <span className="flex justify-between">
            <span>{location}</span>
            <span>{duration}</span>
        </span>
        <p>{description}</p>
    </div>)
}