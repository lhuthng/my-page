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
          <h1>Self-Hosted Infrastructure & DevOps</h1>
          <p>
            A custom-architected VPS environment hosting a multi-tenant
            ecosystem of Svelte/React apps, Rust backends, and automated CI/CD
            pipelines with Nginx/Apache orchestration.
          </p>
          <p>
            <strong>Stack:</strong> Linux (Ubuntu), Nginx, Apache, Docker,
            GitHub Actions, Rust, Node.js
          </p>
        </div>
      }
      details={
        <div className="bg-orange-chalk w-full h-full not-md:border-t-2 md:border-l-2 text-black space-y-4 p-4">
          <p className="italic font-medium">
            "A playground without constraints. Building the engine from scratch
            to own the foundation, the wiring, and the blueprint."
          </p>
          <p>
            This infrastructure project demonstrates my ability to manage the
            full deployment lifecycle, moving beyond "black-box" hosting to a
            transparent, self-managed server architecture.
          </p>
          <div className="space-y-4">
            <h2 className="text-xl font-bold border-b border-black pb-1">
              Technical Architecture:
            </h2>
            <ul className="list-disc list-inside ml-2 space-y-2">
              <li>
                <strong>Traffic Orchestration:</strong> Implemented a
                <strong> multi-layered proxy system</strong> using{" "}
                <strong>Nginx</strong> as a high-performance entry point for SSL
                termination, efficiently routing traffic to{" "}
                <strong>Apache2</strong> and <strong>Node.js</strong>{" "}
                microservices.
              </li>
              <li>
                <strong>Modular Backend:</strong> Leverages a high-performance
                <strong> Rust</strong> and <strong>Node.js</strong> runtime
                environment to serve a diverse range of{" "}
                <strong>Svelte 5</strong> and
                <strong> React</strong> applications with minimal overhead.
              </li>
              <li>
                <strong>Deployment Automation:</strong> Engineered
                <strong> GitHub Actions</strong> CI/CD workflows for automated
                build/deploy cycles, ensuring zero-downtime updates across the
                entire application suite.
              </li>
              <li>
                <strong>Network Sovereignty:</strong> Managed domain
                segmentation and DNS via <strong>Joker.com</strong>, including
                subdomain logic for dedicated mail servers and private
                development environments.
              </li>
              <li>
                <strong>Diverse Ecosystem:</strong> Concurrently hosts
                <strong> Digital Arts Gallery</strong>,{" "}
                <strong>Personal Blog</strong>,<strong> backend APIs</strong>,
                and custom <strong>game servers</strong>
                on a single, optimized Linux instance (Contabo).
              </li>
            </ul>
          </div>
          <div className="pt-2">
            <p className="font-bold">Explore the "How" and "Why":</p>
            <ul className="list-disc list-inside space-y-1">
              <li>
                <a
                  className="text-cyan-700 hover:underline"
                  href="https://blog.huuthangle.site/posts/the-hosting-cloud"
                >
                  [Technical Setup: The Hosting Cloud]
                </a>
              </li>
              <li>
                <a
                  className="text-cyan-700 hover:underline"
                  href="https://blog.huuthangle.site/posts/i-made-a-blog"
                >
                  [Philosophy: So..., I Made a Blog]
                </a>
              </li>
            </ul>
          </div>
        </div>
      }
    />
  );
}
