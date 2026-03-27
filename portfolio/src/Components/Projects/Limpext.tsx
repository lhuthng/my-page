import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";
import "@/Styles/Limpext.css";

export default function Limpext({ active, onClick }: ProjectProps) {
  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div className="flex justify-center items-center bg-white w-full h-full">
          <svg
            className="w-full max-w-80"
            xmlns="http://www.w3.org/2000/svg"
            version="1.1"
            viewBox="0 0 359.2 96"
            height="100%"
            width="100%"
            renderingIntent="geometricPrecision"
          >
            <defs>
              <linearGradient
                id="linear-gradient"
                x1="28.1"
                y1="1.5"
                x2="138.6"
                y2="73.2"
                gradientTransform="translate(0 96) scale(1 -1)"
                gradientUnits="userSpaceOnUse"
              >
                <stop offset="0" stopColor="#295c9c" />
                <stop offset=".2" stopColor="#3078b3" />
                <stop offset=".5" stopColor="#3394c8" />
                <stop offset=".8" stopColor="#329dd6" />
                <stop offset="1" stopColor="#32a2dd" />
              </linearGradient>
              <linearGradient
                id="linear-gradient1"
                x1="11.6"
                y1="87.2"
                x2="100.6"
                y2="87.2"
                gradientTransform="translate(0 96) scale(1 -1)"
                gradientUnits="userSpaceOnUse"
              >
                <stop offset="0" stopColor="#286199" />
                <stop offset=".4" stopColor="#3387c0" />
                <stop offset="1" stopColor="#30a9dc" />
              </linearGradient>
              <linearGradient
                id="linear-gradient2"
                x1="0"
                y1="67.3"
                x2="84.3"
                y2="67.3"
                gradientTransform="translate(0 96) scale(1 -1)"
                gradientUnits="userSpaceOnUse"
              >
                <stop offset="0" stopColor="#3060a3" />
                <stop offset=".5" stopColor="#3585bb" />
                <stop offset=".7" stopColor="#3792cc" />
                <stop offset=".9" stopColor="#3aa1df" />
                <stop offset="1" stopColor="#3ba7e6" />
              </linearGradient>
              <linearGradient
                id="linear-gradient3"
                x1="11.5"
                y1="62.5"
                x2="7.2"
                y2="22.4"
                gradientTransform="translate(0 96) scale(1 -1)"
                gradientUnits="userSpaceOnUse"
              >
                <stop offset="0" stopColor="#3060a3" />
                <stop offset=".6" stopColor="#3585bb" />
                <stop offset=".7" stopColor="#3792cc" />
                <stop offset=".9" stopColor="#3aa1df" />
                <stop offset="1" stopColor="#3ba7e6" />
              </linearGradient>
              <mask id="icon-mask">
                <g>
                  <path
                    className="mask-stroke"
                    d="M81.4,26.4c-8.8,4.7-24.7,10.7-36.9,9.6S3.7,26,3.9,21.3,53.9,3.6,58.2,3.6s40.6,7,44.9,14.1-3.1,56.9-10.6,62-38.1,12.9-45.3,11.6-33.9-14.1-33.9-14.1l-7.6-39.4"
                  />
                </g>
              </mask>
            </defs>
            <g id="Icon" mask="url(#icon-mask)">
              <path
                id="right"
                className="st3"
                d="M110,23l-11.3,55.2c-.6,1.7-.8,3.4-2.6,4.4l-47.2,13.4-34.9-13c-1.5-.9-2.8-2.1-3.6-6.9l34.4,12.6,44.8-12.5,13.4-62.5c6.4,2.1,7.9,5.1,7,9.4h0Z"
              />
              <path
                id="top"
                className="st1"
                d="M98.6,12.3l-1.3,5.3L61,7.3,24.3,15.8l-14.7-4.6L58.8,0l39.9,12.3Z"
              />
              <path
                id="bot"
                className="st0"
                d="M81.6,31.6l-36.4,8L3.2,24.1c-2.3-1-3.2-4.1-3.2-6.4l41.9,15.3,34.2-7.3.8-3c10.1-1.8,7.7,8.2,4.7,8.9h0Z"
              />
              <path
                id="left"
                className="st2"
                d="M17.1,74.7l-7.7-2.3L1.6,32.7c4.3-.5,7,1.2,7.8,4.2l7.6,37.7Z"
              />
            </g>
            <g id="text-container">
              <g id="Text">
                <path
                  id="bot1"
                  data-name="bot"
                  d="M132,65.3h1.6v12.8c0,.9,0,1.8-.3,2.7-.2.9-.5,1.6-1,2.3s-1.2,1.2-2,1.6-2,.6-3.3.6-1.9-.1-2.7-.4-1.4-.6-1.8-1c-.4-.4-.8-.8-1.1-1.3-.3-.5-.4-.9-.4-1.4h1.7c.1.5.3,1,.6,1.3.3.3.6.6,1,.8.4.2.8.3,1.2.4.5,0,.9.1,1.4.1,1.1,0,2-.2,2.7-.6s1.2-.9,1.5-1.5.6-1.3.7-2.1c0-.8.1-1.6.1-2.4-.5.8-1.3,1.5-2.2,2-.9.4-1.9.7-2.9.7s-2-.2-2.8-.6c-.8-.4-1.4-1-2-1.6s-.9-1.5-1.2-2.4-.4-1.9-.4-2.8.1-2,.3-2.9c.2-.9.6-1.7,1.1-2.4.5-.7,1.2-1.2,2-1.6s1.8-.6,2.9-.6.9,0,1.4.2c.5.1.9.3,1.4.5.9.5,1.7,1.2,2.1,2.1h0v-2.5h0ZM132,72.4c0-.9-.1-1.7-.3-2.4s-.5-1.4-.9-1.9-.9-1-1.5-1.3c-.6-.3-1.3-.5-2-.5s-1.3.1-1.9.4-1.1.6-1.5,1.1-.8,1.1-1,1.7-.4,1.5-.4,2.4c0,.9,0,1.7.2,2.5s.5,1.5.9,2,.9,1,1.5,1.4,1.4.5,2.2.5,1.4-.2,2-.5,1.1-.7,1.5-1.3.7-1.2.9-1.9.3-1.5.3-2.3h0ZM142.8,71.5c.3,0,.6,0,1-.2.4,0,.7-.2,1-.3.4-.2.4-.4.5-.7s0-.6,0-.9c0-.9-.3-1.6-.8-2.1-.5-.6-1.4-.8-2.6-.8s-1,0-1.5.2-.8.3-1.2.5-.6.6-.8,1-.4,1-.4,1.6h-1.6c0-.9.2-1.6.5-2.2.3-.6.7-1.1,1.2-1.5s1-.7,1.7-.9c.6-.2,1.3-.3,2-.3s1.3,0,1.9.2c.6.2,1.1.4,1.5.8.4.4.8.8,1,1.4.2.6.4,1.3.4,2.2v8c0,.7.1,1.1.3,1.3s.6.1,1.3-.1v1.3c-.1,0-.3,0-.5.2-.2,0-.5.1-.7.1s-.5,0-.7,0c-.3,0-.5-.1-.6-.3s-.3-.3-.4-.5-.2-.4-.2-.6c0-.2,0-.5,0-.8-.6.8-1.4,1.4-2.2,1.8s-1.7.6-2.7.6-1.1,0-1.7-.3-1-.4-1.4-.8-.7-.8-1-1.3c-.2-.5-.4-1.1-.4-1.9,0-2.5,1.5-4,4.6-4.4l2.1-.3h0ZM145.4,72.3c-.6.3-1.3.5-2,.6-1,.1-1.3.2-2,.2-1.2,0-2.1.4-2.8.9s-1,1.2-1,2.3,0,.8.2,1.2.4.6.6.8c.3.2.6.4.9.5s.7.2,1.1.2c.6,0,1.2,0,1.8-.3.6-.2,1.1-.4,1.5-.8s.8-.8,1.1-1.3c.3-.5.4-1.2.4-1.9v-2.3h0ZM151.9,75.5c.2,1.3.7,2.2,1.3,2.7s1.5.8,2.7.8,1,0,1.5-.2c.4-.1.7-.4,1-.6s.5-.5.6-.9c.1-.3.2-.7.2-1,0-.7-.2-1.3-.6-1.6-.4-.3-.9-.6-1.6-.8s-1.3-.4-2.1-.5-1.4-.4-2.1-.6-1.2-.7-1.6-1.2-.6-1.3-.6-2.3.4-2.2,1.2-3c.8-.8,2-1.2,3.4-1.2s2.6.4,3.5,1.1c.9.7,1.4,1.9,1.6,3.6h-1.6c-.1-1.1-.5-1.9-1.2-2.4-.6-.5-1.4-.7-2.4-.7s-1.7.2-2.2.7-.8,1.1-.8,1.8.2,1.2.6,1.5c.4.3.9.6,1.6.9s1.3.4,2.1.6c.7.2,1.4.4,2.1.7s1.2.7,1.6,1.3.6,1.3.6,2.2-.1,1.3-.4,1.9-.6,1-1.1,1.4-1,.6-1.7.8-1.4.3-2.2.3-1.5-.1-2.1-.4-1.1-.6-1.5-1.1c-.4-.5-.8-1-1-1.6s-.4-1.3-.4-2h1.6,0ZM168.5,80.1c-.2,0-.4.1-.7.2s-.6.1-1,.1c-.8,0-1.4-.2-1.8-.6s-.6-1.2-.6-2.3v-11h-2.3v-1h2.3v-4.3h1.5v4.3h2.5v1h-2.5v10.4c0,.4,0,.7,0,.9s0,.5.2.7c0,.2.2.3.4.4s.4.1.8.1.4,0,.6,0c.2,0,.4,0,.6-.1v1.4h0ZM172.5,80.1h-1.5v-14.6h1.5v2.3s.6-.8,1.3-1.3c1.3-.8,2.5-1,3.7-1v1.3c-.9,0-1.8.2-2.4.5s-1.4.9-1.7,1.5c-.3.6-.6,1.1-.8,1.8s-.2,1.5-.2,2.3v7.2ZM184.1,80.5c-.9,0-1.7-.2-2.5-.5s-1.5-.8-2-1.5c-.6-.6-1-1.5-1.4-2.4-.3-1-.5-2.1-.5-3.4s.1-2.1.4-3,.7-1.8,1.2-2.5,1.2-1.3,2-1.7,1.7-.6,2.8-.6,2,.2,2.9.6c.8.4,1.5,1,2,1.7s.9,1.5,1.2,2.5c.3,1,.4,2,.4,3.1s-.2,2.2-.5,3.2-.7,1.8-1.3,2.5-1.2,1.2-2,1.6-1.7.6-2.6.6h0ZM188.8,72.7c0-.9,0-1.7-.3-2.5-.2-.8-.5-1.4-.8-2s-.9-1-1.5-1.3c-.6-.3-1.3-.5-2.2-.5s-1.6.2-2.2.5c-.6.4-1.1.8-1.5,1.4s-.7,1.2-.8,2c-.2.8-.3,1.5-.3,2.3s.1,1.8.3,2.5c.2.8.5,1.4.9,2s.9,1,1.5,1.3c.6.3,1.3.5,2,.5s1.6-.2,2.2-.5c.6-.4,1.1-.8,1.5-1.4.4-.6.7-1.3.8-2,.2-.7.3-1.5.3-2.3h0ZM199.3,64.6h4.4l3.1,10.9,3.2-10.9h4.2l-5.3,14.7h-4.1l-5.3-14.7ZM229,75c-.1.9-.6,1.8-1.4,2.7-1.3,1.4-3,2.1-5.3,2.1s-3.5-.6-5-1.8c-2.2-1.8-2.2-3.2-2.2-5.9s.7-4.5,2-5.9,3-2,5.1-2,2.3.2,3.3.7,1.8,1.2,2.4,2.2c.6.9,1,1.9,1.1,3,.1.7.1,1.7.1,2.9h-10.1c0,1.5.5,2.5,1.4,3.1.5.4,1.2.6,1.9.6s1.4-.2,1.9-.7c.3-.2.5-.6.7-1h4ZM225.1,70.5c0-1-.4-1.8-.9-2.3-.8-.8-1.3-.8-2.1-.8s-1.6.3-2.1.8-.8,1.3-.9,2.3h6ZM239.2,68.2c-1.6,0-2.6.5-3.1,1.5-.3.6-.5,1.4-.5,2.6v7h-3.9v-14.7h3.7v2.6c.6-1,1.1-1.7,1.6-2,.7-.6,1.7-.9,2.8-.9h.2c0,0,.2,0,.3,0v3.9c-.2,0-.5,0-.7,0s-.3,0-.5,0h0ZM255,66.2c1.2,1.3,1.8,3.1,1.8,5.6s-.6,4.6-1.8,5.9-2.7,2-4.5,2-2.2-.3-2.9-.9c-.4-.3-.8-.8-1.3-1.4v7.1h-3.8v-19.9h3.7v2.2c.4-.6.9-1.1,1.3-1.5.9-.7,1.9-1,3.1-1,1.7,0,3.2.6,4.4,1.9h0ZM252.8,72c0-1.1-.3-2.1-.8-3s-1.4-1.3-2.5-1.3-2.4.7-2.9,2c-.3.7-.4,1.6-.4,2.7,0,1.7.5,2.9,1.4,3.6.5.4,1.2.6,1.9.6,1.1,0,1.9-.4,2.5-1.2.6-.8.9-1.9.9-3.3M260.2,66.2c1-1.3,2.8-1.9,5.3-1.9s3.1.3,4.3,1c1.3.6,1.9,1.8,1.9,3.6v6.8c0,.5,0,1,0,1.7,0,.5.1.9.2,1s.3.3.6.4v.6h-4.2c-.1-.3-.2-.6-.2-.8s0-.6-.1-.9c-.5.6-1.2,1.1-1.9,1.5-.8.5-1.8.7-2.8.7s-2.4-.4-3.3-1.1c-.9-.8-1.3-1.8-1.3-3.2s.7-3.1,2.1-3.9c.8-.4,1.9-.8,3.4-.9l1.3-.2c.7,0,1.2-.2,1.5-.3.6-.2.8-.6.8-1.1s-.2-1-.6-1.2c-.6-.3-1-.3-1.9-.3s-1.6.2-1.9.7c-.3.3-.5.8-.5,1.4h-3.7c0-1.3.4-2.3,1.1-3.2h0ZM263.1,76.6c.4.3.8.4,1.3.4.8,0,1.6-.2,2.3-.7s1.1-1.4,1.1-2.7v-1.4c-.2.2-.5.3-.7.4s-.6.2-1,.3l-.9.2c-.8.1-1.4.3-1.8.5-.6.4-.9.9-.9,1.6s.2,1.1.6,1.4M287.9,69.9h-4c0-.6-.3-1-.6-1.5-.4-.6-1.1-.9-2-.9-1.3,0-2.2.6-2.7,1.9-.3.7-.4,1.6-.4,2.7s.1,1.9.4,2.6c.5,1.2,1.3,1.8,2.6,1.8s1.6-.2,1.9-.7.6-1.1.7-1.9h4c0,1.2-.5,2.3-1.3,3.3-1.2,1.7-3,2.5-5.4,2.5s-4.1-.7-5.3-2.1-1.7-3.2-1.7-5.5.6-4.5,1.9-5.9c1.2-1.4,3-2.1,5.1-2.1,1.9,0,3.4.4,4.6,1.2s1.9,2.3,2.1,4.4h0ZM304.2,79.4h-4.7l-3.5-6.3-1.6,1.7v4.6h-3.8v-19.9h3.8v10.7l4.8-5.5h4.8l-5.2,5.6,5.4,9ZM315.6,77.3s-.1.2-.3.4-.3.4-.5.6c-.6.5-1.2.9-1.8,1.1s-1.2.3-2,.3c-2.2,0-3.7-.8-4.4-2.4-.4-.9-.6-2.1-.6-3.8v-8.9h4v8.9c0,.8.1,1.5.3,1.9.4.8,1.1,1.1,2.1,1.1s2.2-.5,2.7-1.6c.3-.6.4-1.3.4-2.3v-8h3.9v14.7h-3.8v-2.1h0ZM330.1,67.5c-1.3,0-2.2.6-2.7,1.7-.3.6-.4,1.3-.4,2.2v8h-3.9v-14.7h3.7v2.1c.5-.8,1-1.3,1.4-1.6.8-.6,1.8-.9,3-.9s2.8.4,3.8,1.2,1.5,2.1,1.5,4v9.9h-4v-9c0-.8-.1-1.4-.3-1.8-.4-.8-1.1-1.1-2.2-1.1h0ZM344.3,81.9c.4.4,1.1.5,2.1.5,1.4,0,2.3-.5,2.8-1.4.3-.6.5-1.6.5-3v-.9c-.4.6-.8,1.1-1.2,1.4-.8.6-1.8.9-3,.9-1.9,0-3.4-.7-4.6-2s-1.7-3.1-1.7-5.4.6-4,1.6-5.5,2.7-2.2,4.7-2.2,1.4.1,1.9.3c.9.4,1.7,1.1,2.3,2.1v-2.1h3.8v14c0,1.9-.3,3.3-1,4.3-1.1,1.7-3.2,2.5-6.4,2.5s-3.4-.4-4.6-1.1c-1.2-.7-1.9-1.8-2-3.3h4.2c.1.5.3.8.5,1h0ZM343.7,74.3c.5,1.2,1.5,1.9,2.8,1.9s1.7-.3,2.3-1,.9-1.8.9-3.3-.3-2.5-.9-3.2-1.4-1.1-2.4-1.1-2.3.6-2.8,1.9c-.3.7-.4,1.5-.4,2.5s.1,1.6.4,2.3h0Z"
                />
                <g id="top1" data-name="top">
                  <path d="M121.7,16.7h8.8v29.8h19.1v8.2h-27.9V16.7ZM161.6,16.7v6.8h-9.3v-6.8h9.3ZM161.6,25.5v29.4h-9.3v-29.4h9.3ZM186.3,27.8c-.8-1.8-2.4-2.7-4.8-2.7s-4.6.9-5.6,2.7c-.5,1-.8,2.5-.8,4.6v22.4h-10.6s0-38,0-38h10.2s0,5.1,0,5.1c1.2-1.9,2-2.9,3.1-3.7,1.9-1.5,4.6-1.6,7.6-1.6s5.1.8,6.9,2c1.4,1.2,2.5,1.4,3.2,3.3,1.3-2.2,2.4-3.1,4.3-4.1,2-1,4.6-1.2,7.1-1.2s3.3.5,4.9,1.1c1.6.6,3,1.8,4.3,3.3,1,1.3,1.7,2.9,2.1,4.8.2,1.2.3,3.1.3,5.5v23.4h-10v-23.7c0-1.4-.2-2.6-.7-3.5-.9-1.7-2.4-2.6-4.8-2.6s-4.5,1.1-5.5,3.3c-.5,1.2-.8,2.6-.8,4.3v22.2h-9.7v-22.2c0-2.2-.2-3.8-.7-4.8M230.2,42.3v12.4s-8.6,0-8.6,0V16.7h18c4.2,0,7.5,1,9.9,2.9s3.7,5,3.7,9.1-1.2,7.7-3.7,9.5-6,4-10.6,4h-8.8ZM241.5,33.2c1.1-.9,1.7-2.4,1.7-4.3s-.6-3.4-1.7-4.2c-1.1-.8-2.7-1.3-4.8-1.3h-6.5v11.2h6.5c2,0,3.6-.4,4.8-1.3h0ZM289.4,43.6c-.2,2.2-1.4,4.4-3.4,6.6-3.1,3.6-7.3,4.5-12.9,4.5s-9-.7-12.6-3.7-5.3-7.9-5.3-14.6,1.6-11.2,4.8-14.6,7.4-5.1,12.5-5.1,5.8.6,8.2,1.7c2.4,1.1,4.4,2.9,6,5.4,1.4,2.2,2.4,4.7,2.8,7.5.2,1.7.3,4.1.3,7.2h-25c.1,3.7,1.3,6.2,3.5,7.7,1.3.9,2.9,1.4,4.8,1.4s3.6-.6,4.8-1.7c.7-.6,1.3-1.4,1.8-2.5h9.8,0ZM279.9,32.4c-.2-2.5-.9-4.4-2.3-5.8-1.4-1.3-3.1-2-5.1-2s-3.9.7-5.2,2.1-2,3.3-2.3,5.6h14.9,0Z" />
                  <polygon points="323.9 54.8 312.2 54.8 305.4 43.5 297 54.8 285.9 54.8 300 35.1 287.5 16.8 299.2 16.8 305.4 27 311.9 16.8 323.1 16.8 310.3 35.1 323.9 54.8" />
                  <polygon points="357.6 16.8 357.6 24.7 345.5 24.7 345.5 54.8 336.9 54.8 336.9 24.7 324.7 24.7 324.7 16.8 357.6 16.8 357.6 16.8" />
                </g>
              </g>
            </g>
          </svg>
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full bg-blue-400 space-y-4 p-4 text-white">
          <h1>Full-Stack Web Development &amp; SEO</h1>
          <p>
            Developed custom, high-performance landing pages using Svelte 5 and
            Tailwind CSS, featuring Leaflet map integrations, full localization
            support, and optimized Apache server configurations.
          </p>
          <p>
            <strong>Technologies:</strong> Svelte 5, GSAP, Tailwind CSS,
            Leaflet, PHP, Apache, SEO
          </p>
        </div>
      }
      details={
        <div className="w-full h-full bg-white text-black border-blue-400 md:rounded-tr-xl not-md:rounded-bl-xl p-4 border-2 space-y-4">
          <h1>
            <strong>
              Freelance Web Development &amp; SEO: High-Performance Landing
              Pages
            </strong>
          </h1>
          <p>
            As a developer, I delivered an end-to-end web solution for{" "}
            <a
              className="text-blue-500 hover:brightness-110 hover:underline"
              href="https://limpext.de"
            >
              Limpext
            </a>
            , focusing on performance engineering, custom interactive
            components, and strategic SEO to drive client engagement.
          </p>
          <div className="flex flex-col gap-4">
            <div>
              <strong>Frontend &amp; Performance Engineering:</strong>
              <ul className="ml-4 list-disc list-inside space-y-1">
                <li>
                  <strong>Custom UI Development: </strong>Built two
                  zero-template, high-performance landing sites using{" "}
                  <strong>Svelte 5</strong> and
                  <strong> Tailwind CSS</strong> to perfectly match the client's
                  branding and service requirements.
                </li>
                <li>
                  <strong>Localization &amp; Speed: </strong>Engineered a custom
                  localization framework to support dynamic language switching
                  without page reloads, maintaining high Lighthouse performance
                  scores.
                </li>
                <li>
                  <strong>Interactive Components:</strong> Integrated custom
                  <strong> Leaflet</strong> map integrations and created
                  engaging scroll animations using <strong>GSAP</strong> to
                  modernize the user journey.
                </li>
              </ul>
            </div>
            <div>
              <strong>System Architecture &amp; Visibility:</strong>
              <ul className="ml-4 list-disc list-inside space-y-1">
                <li>
                  <strong>Deployment &amp; Routing:</strong> Managed the
                  production deployment on <strong>Alfahosting</strong>,
                  configuring
                  <strong> Apache</strong> environments including web routing
                  and server-side environment settings.
                </li>
                <li>
                  <strong>Data Handling: </strong>Architected backend workflows
                  with
                  <strong> PHP</strong> to manage secure data handling and
                  automated lead generation from customer contact forms.
                </li>
                <li>
                  <strong>SEO Optimization:</strong> Implemented structured
                  metadata and SEO-friendly routing to ensure organic search
                  visibility and proper indexing.
                </li>
              </ul>
            </div>
          </div>
        </div>
      }
    />
  );
}
