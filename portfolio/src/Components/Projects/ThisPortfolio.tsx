import Lottie from "../Lottie";
import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";
import boxes from "@/Assets/Lotties/boxes.json";

export default function ThisPortfolio({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div className="relative bg-white-chalk w-ful h-full">
          <Lottie
            className="absolute left-1/2 top-1/2 -translate-1/2 max-w-full"
            animationData={boxes}
            loop={true}
            speed={0.5}
          />
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-orange-chalk text-black">
          <h1>Personal Web Server</h1>
          <p>
            A VPS-based, full-stack environment that hosts this portfolio, a
            personal blog, backend services, and live development demos for
            clients.
          </p>
          <p>
            <strong>Technology:</strong> GitHub Actions (CI/CD), VPS, Nginx,
            Apache2, Node.js, React, Svelte, UI/UX Design, HTML5, CSS3,
            JavaScript
          </p>
        </div>
      }
      details={
        <div className="bg-orange-chalk w-full h-full not-md:border-t-2 md:border-l-2 border-black text-black space-y-4 p-4">
          <p>
            A playground without constraints. Building the engine from scratch
            to own the foundation, the wiring, and the blueprint.
          </p>
          <p>
            This project showcases advanced DevOps and server management
            expertise across a multi-faceted hosting architecture.
          </p>
          <div className="space-y-4">
            <h1>Key Features &amp; Technical Depth: </h1>
            <ul className="list-disc list-inside ml-2 space-y-2">
              <li>
                <strong>Infrastructure:</strong> Full-stack environment hosted
                on a Linux VPS (Contabo) serving as the core hosting
                infrastructure for the portfolio, blog, and backend services.
              </li>
              <li>
                <strong>Custom Engine:</strong> Modular architecture using a{" "}
                <strong>Rust</strong> and <strong>Node.Js</strong> backend with
                a <strong>Svelte 5</strong>to remove middleman limitations.
              </li>
              <li>
                <strong>Traffic Orchestration: </strong> Multi-layered proxy
                system using <strong>Nginx</strong> for SSL termination and
                caching, with <strong>Apache2</strong>
                for specialized application hosting.
              </li>
              <li>
                <strong>Automation:</strong> Fully automated CI/CD pipeline
                utilizing GitHub Actions for zero-downtime deployment.
              </li>
              <li>
                <strong>Network Services:</strong> Domain configuration via
                joker.com, including subdomain segmentation and maintenance of a
                dedicated mail server.
              </li>
              <li>
                <strong>Diverse Content Hosting:</strong> The single platform
                concurrently hosts a variety of personal and client-facing
                assets, including the main <strong>Digital Portfolio</strong>,{" "}
                <strong>a Personal Blog</strong>, various{" "}
                <strong>backend APIs</strong>,{" "}
                <strong>live development demos</strong>, and custom{" "}
                <strong>game servers</strong>.
              </li>
            </ul>
          </div>
          <p>
            If you are interested in the "How" and "Why" behind this
            architecture, you can read this kick-off on my blog:{" "}
            <a
              className="text-cyan-600 hover:font-semibold"
              href="https://blog.huuthang.site/posts/i-made-a-blog"
            >
              [So..., I Made a Blog]
            </a>
            .
          </p>
        </div>
      }
    />
  );
}
