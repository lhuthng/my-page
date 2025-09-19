import CoolHeader from "./CoolHeader";
import '@/Styles/Experience.css';
import ExperienceInfo from "./WorkExperience";

export default function Experience() {
    return (<section className="max-w-340 mx-auto pb-64">
        <div className="w-[calc(100%-4rem)] lg:w-auto max-w-300 mx-auto">
            <CoolHeader title="Experience & Education" />
            <div className="grid-flex space-x-2">
                <ExperienceInfo
                    title="Full-stack developer"
                    duration="May 2025 - Present"
                    organization="Limpext"
                    technologies={['HTML/CSS/JS', 'Svelte 5', 'PHP', 'Apache']}
                    project="Designed, developed, and launched a custom, professional website for Limpext."
                >
                    <span><b>Enhanced Visibility: </b> Drove a significant increase in web traffic and improved organic search rankings by implementing comprehensive SEO strategies.</span>
                    <span><b>Full-stack Development: </b> Oversaw all aspects of development, from designing the front-end user interface to crafting the back-end logic.</span>
                </ExperienceInfo>
                <ExperienceInfo
                    title="Programmer Analyst"
                    duration="January 2020 – February 2021"
                    organization="DXC Technology Vietnam"
                    location="Ho Chi Minh City"
                    technologies={['HTML/CSS/JS', 'AngularJS', 'C#', 'ASP.NET Framework', 'MongoDB', 'OracleDB', 'IIS', 'Task Scheduler']}
                    project="Resolved technical debt and built new features to maintain and enhance enterprise applications for a variety of clients."
                >
                    <span><b>Front-end Implementation: </b> Utilized AngularJS to implement user-facing features, ensuring a seamless and reliable experience across multiple client projects.</span>
                    <span><b>API Development: </b> Developed new API endpoints using ASP.NET to enhance an internal portal, streamlining data retrieval for an internal sales team.</span>
                    <span><b>Bug Resolution: </b> Fixed critical bugs related to the customer shopping cart and its database, directly improving the stability and reliability of a core e-commerce platform.</span>
                </ExperienceInfo>
                <ExperienceInfo
                    title="Scientific Assistant"
                    duration="July 2022 – October 2024"
                    organization="Max-Planck Institute für Biophysics"
                    location="Frankfurt am Main"
                    technologies={['Python', 'MATLAB', 'NumPy', 'SciPy', 'Pandas', 'Matplotlib', 'pytest']}
                    project="Transformed a legacy data processing workflow by porting and optimizing MATLAB scripts to Python, enhancing collaboration and research efficiency."
                >
                    <span><b>Performance Optimization: </b>Accelerated data processing by rewriting legacy MATLAB scripts in Python and leveraging optimized libraries like NumPy and Pandas.</span>
                    <span><b>Workflow Automation: </b>Developed a custom Python module to parse and process a specialized data format, reducing manual data handling time by 50%.</span>
                    <span><b>Quality Assurance: </b> Authored a comprehensive suite of unit tests using pytest, establishing a robust quality assurance framework that ensured the accuracy and integrity of scientific data analysis.</span>
                </ExperienceInfo>
                <ExperienceInfo
                    title="Teaching Assistant"
                    duration="March 2021 – October 2021"
                    organization="Frankfurt University of Applied Science"
                    location="Frankfurt am Main"
                    technologies={['Java']}
                    project="Guided master's students through a course on competitive programming, focusing on fundamental algorithms and problem-solving techniques."
                >
                    <span><b>Mentorship & Instruction: </b>Mentored students on key data structures and algorithms, including dynamic programming, graph theory, and computational geometry, improving their problem-solving skills.</span>
                    <span><b>Technical Expertise: </b>Provided direct, real-time feedback on code, identified algorithmic flaws, and suggested optimizations, enhancing the students' coding efficiency and correctness.</span>
                </ExperienceInfo>
            </div>
        </div>
    </section>);
}