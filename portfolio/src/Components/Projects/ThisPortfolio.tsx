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
            A robust, VPS-based, full-stack environment that serves as the core
            hosting infrastructure for this entire digital platform, including
            the portfolio, a personal blog, various backend services, and live
            development demonstrations for clients.
          </p>
          <p>
            This project showcases advanced DevOps and server management
            expertise across a multi-faceted hosting architecture.
          </p>
          <div className="space-y-4">
            <h1>Key Features &amp; Technical Depth: </h1>
            <ul className="list-disc list-inside ml-2 space-y-2">
              <li>
                <strong>VPS-Based Core Infrastructure:</strong> The server
                operates as a robust, full-stack environment hosted on a{" "}
                <strong>Virtual Private Server (VPS) via Contabo</strong>. This
                demonstrates practical expertise in managing dedicated cloud
                infrastructure and resource allocation.
              </li>
              <li>
                <strong>Advanced Server Management:</strong> Implemented a
                high-performance, multi-layered proxy system featuring{" "}
                <strong>Nginx as the primary reverse proxy</strong> for
                efficient traffic, caching, and SSL termination, working in
                tandem with <strong>Apache2</strong> for specialized application
                hosting.
              </li>
              <li>
                <strong>Continuous Deployment (CI/CD):</strong> Features a fully
                automated deployment pipeline utilizing{" "}
                <strong>GitHub Actions</strong>. This ensures rapid, reliable,
                and zero-downtime Continuous Integration and Continuous
                Deployment for all hosted applications.
              </li>
              <li>
                <strong>Full-Stack Technology Stack:</strong> Engineered to
                support modern development using{" "}
                <strong>Node.js, Go, and Rust</strong> for backend services,
                with frontends built using cutting-edge frameworks like{" "}
                <strong>React</strong> (for complex applications) and{" "}
                <strong>Svelte 5</strong> (for optimized interfaces).
              </li>
              <li>
                <strong>Comprehensive Domain &amp; Network Services:</strong>{" "}
                Manages domain configurations via <strong>joker.com</strong>,
                including the complex setup of subdomains for service
                segmentation and the deployment and maintenance of a dedicated{" "}
                <strong>mail server</strong>.
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
        </div>
      }
    />
  );
}
