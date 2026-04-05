import "@/Styles/SynRecordia.css";
import type { ProjectProps } from "./ProjectTemplate";
import ProjectTemplate from "./ProjectTemplate";
import github from "@/Assets/SVGs/github.svg";
import React from "react";

// Recorder hole patterns: [thumb, hole1, hole2, hole3, hole4, hole5, hole6]
// 1 = covered (filled bar), 0 = open (empty)
const HOLES: Record<string, number[]> = {
  C4: [1, 1, 1, 1, 1, 1, 1],
  D4: [1, 1, 1, 1, 1, 1, 0],
  E4: [1, 1, 1, 1, 1, 0, 0],
  G4: [1, 1, 1, 0, 0, 0, 0],
};

// "Mary had a little lamb": E D C D E E G E
// [noteName, visibleStart (0–100%), visibleWidth (0–100%)]
const MELODY: Array<[keyof typeof HOLES, number, number]> = [
  ["E4", 0, 10],
  ["D4", 13, 8],
  ["C4", 24, 12],
  ["D4", 39, 8],
  ["E4", 50, 10],
  ["E4", 63, 10],
  ["G4", 76, 14],
  ["E4", 93, 10],
];

// Beat divider lines at these visible positions (%)
const BEAT_POSITIONS = [20, 45, 70, 95];

const CYAN = "rgba(0, 230, 180, 0.85)";
const CYAN_GLOW = "0 0 6px rgba(0, 230, 180, 0.35)";

// Layout constants (all in % of container height)
const ROW_TOP = 10; // y of first row
const ROW_H = 7; // height of each row
const ROW_GAP = 2.5; // gap between rows
const NUM_ROWS = 7;

export default function SynRecordia({ active, onClick }: ProjectProps) {
  // The scrolling track is 200% wide so we can loop seamlessly:
  // first half  (0–50%)  = visible positions 0–100%
  // second half (50–100%) = identical copy  → translateX(-50%) looks same as 0
  // Every visible position X% maps to container position X/2 %.

  const noteRects: React.ReactNode[] = [];
  MELODY.forEach(([noteName, visStart, visWidth], noteIdx) => {
    const holes = HOLES[noteName];
    if (!holes) return;
    [0, 50].forEach((cycle) => {
      holes.forEach((filled, rowIdx) => {
        if (!filled) return;
        noteRects.push(
          <div
            key={`note-${noteIdx}-c${cycle}-r${rowIdx}`}
            className="absolute"
            style={{
              left: `calc(${visStart / 2 + cycle}% + 2px)`,
              width: `calc(${visWidth / 2}% - 4px)`,
              top: `${ROW_TOP + rowIdx * (ROW_H + ROW_GAP)}%`,
              height: `${ROW_H}%`,
              backgroundColor: CYAN,
              borderRadius: "2px",
              boxShadow: CYAN_GLOW,
            }}
          />,
        );
      });
    });
  });

  const beatLines: React.ReactNode[] = [];
  BEAT_POSITIONS.forEach((visPos, i) => {
    [0, 50].forEach((cycle) => {
      beatLines.push(
        <div
          key={`beat-${i}-c${cycle}`}
          className="absolute top-0 bottom-0 w-px pointer-events-none"
          style={{
            left: `${visPos / 2 + cycle}%`,
            background: "rgba(255,255,255,0.07)",
          }}
        />,
      );
    });
  });

  return (
    <ProjectTemplate
      active={active}
      onClick={onClick}
      illustration={
        <div
          className="relative w-full h-full overflow-hidden select-none"
          style={{ background: "#0d0d1a" }}
        >
          {/* Static row guide backgrounds */}
          {Array.from({ length: NUM_ROWS }, (_, i) => (
            <div
              key={`guide-${i}`}
              className="absolute w-full pointer-events-none"
              style={{
                top: `${ROW_TOP + i * (ROW_H + ROW_GAP)}%`,
                height: `${ROW_H}%`,
                background: "rgba(255,255,255,0.025)",
              }}
            />
          ))}

          {/* Scrolling track — 200% wide, moves -50% for a seamless loop */}
          <div
            className="absolute top-0 h-full"
            style={{
              width: "200%",
              animation: "synrecordiaScroll 12s linear infinite",
            }}
          >
            {beatLines}
            {noteRects}
          </div>

          {/* Fixed playhead on the right */}
          <div
            className="absolute top-0 bottom-0 z-20 pointer-events-none"
            style={{
              right: "6%",
              width: "2px",
              background:
                "linear-gradient(to bottom, transparent, rgba(255,255,255,0.55) 15%, rgba(255,255,255,0.55) 85%, transparent)",
            }}
          />

          {/* App label */}
          <div
            className="absolute top-2 left-3 font-mono uppercase tracking-widest text-xs pointer-events-none"
            style={{ color: "rgba(255,255,255,0.22)" }}
          >
            SynRecordia
          </div>

          {/* Legend */}
          <div className="absolute bottom-2 left-3 flex items-center gap-1.5 pointer-events-none">
            <div className="w-3 h-2 rounded-sm" style={{ background: CYAN }} />
            <span
              className="font-mono text-xs"
              style={{ color: "rgba(255,255,255,0.35)" }}
            >
              Full
            </span>
          </div>
        </div>
      }
      description={
        <div className="flex flex-col w-full h-full space-y-4 p-4 bg-navi-chalk text-white-chalk">
          <h1>Interactive Piano &amp; Recorder Visualizer</h1>
          <p>
            A browser-based, Synthesia-inspired music visualizer and sampler
            that syncs sampled audio with a live-scrolling piano-roll and
            animated fingering graphics — fully client-side.
          </p>
          <p>
            <strong>Stack:</strong> React, Tone.js, PIXI.js, Web Audio API
          </p>
        </div>
      }
      details={
        <div className="bg-white-chalk w-full h-full not-md:border-t-2 md:border-l-2 border-navi-chalk text-black space-y-4 p-4">
          <p className="italic font-medium">
            "What if you could see and hear the music at the same time — every
            note, every fingering, perfectly in sync?"
          </p>
          <p>
            Born from a desire to learn recorder visually, SynRecordia
            reimagines how you interact with sheet music. Sampled instruments,
            smooth piano-roll scrolling, and real-time playback controls — all
            running in the browser without a backend.
          </p>
          <div className="space-y-4">
            <h2 className="text-xl font-bold border-b border-black pb-1">
              Core Features:
            </h2>
            <ul className="list-disc list-inside ml-2 space-y-2">
              <li>
                <strong>Horizontal Piano-Roll:</strong> Smooth right-to-left
                scrolling timeline that renders per-note fingering hole patterns
                and note labels with glow highlights for active notes, powered
                by <strong>PIXI.js</strong>.
              </li>
              <li>
                <strong>Sampled Instruments:</strong> A custom Packed Sampler
                abstraction supports multiple instruments (Salamander Grand
                Piano, Philharmonia flute/recorder) with per-track instrument
                and variant switching.
              </li>
              <li>
                <strong>Real-time Controls:</strong> Play / Pause / Restart,
                tempo (BPM) control, loop mode, and mouse / touch / wheel
                scrubbing — all scheduled via <strong>Tone.js</strong> for
                sample-accurate timing.
              </li>
              <li>
                <strong>Instrument Config UI:</strong> Per-track volume and
                variant controls let you mix and switch instruments on the fly
                during playback.
              </li>
            </ul>
          </div>
          <div className="flex items-center gap-4 pt-2 flex-wrap">
            <a
              href="https://github.com/lhuthng/synrecordia"
              target="_blank"
              rel="noopener noreferrer"
              title="View on GitHub"
            >
              <i
                className="inline-block w-8 h-8 hover:scale-110 transition-transform"
                style={{
                  backgroundColor: "black",
                  maskImage: `url(${github})`,
                  WebkitMaskImage: `url(${github})`,
                  maskRepeat: "no-repeat",
                  maskPosition: "center",
                  maskSize: "contain",
                }}
              />
            </a>
            <a
              className="font-bold text-navi-chalk hover:underline"
              href="https://synrecordia.netlify.app/"
              target="_blank"
              rel="noopener noreferrer"
            >
              Live Demo →
            </a>
          </div>
        </div>
      }
    />
  );
}
