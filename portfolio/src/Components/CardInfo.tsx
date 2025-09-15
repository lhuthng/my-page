import { useRef, useState, type ReactNode } from "react"

export interface CardInfoProps {
    x: number,
    y: number,
    dx?: number,
    dy?: number,
    paths?: [number, number][],
    strokeColor?: string,
    className?: string,
    style?: React.CSSProperties,
    detail: ReactNode
}

const strokeWidth = 4;
const circleRadius = 6;

function calculatePoints(paths: [number, number][]): [
    pathData: string, 
    viewbox: { x: number, y: number, width: number, height: number },
    lastPosition: [number, number]
] {

    let minX = -circleRadius, minY = -circleRadius, maxX = circleRadius, maxY = circleRadius;

    let x = 0, y = 0;

    if (paths[0]) {
        const angleInRadians = paths[0][0] * (Math.PI / 180);
        x += circleRadius * Math.cos(angleInRadians);
        y += circleRadius * Math.sin(angleInRadians);
    }

    let points: [number, number][] = [[x, y]];

    paths.forEach(([angle, length], index) => {
        const angleInRadians = angle * (Math.PI / 180);
        if (index === 0) length -= circleRadius;
        x += length * Math.cos(angleInRadians);
        y += length * Math.sin(angleInRadians);
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
        points.push([x, y]);
    });

    return [
        points.map((point, index) => `${index === 0 ? 'M' : 'L'}${Number(point[0]).toFixed(3)} ${Number(point[1]).toFixed(3)}`).join(), 
        {x: minX, y: minY, width: maxX - minX, height: maxY - minY},
        [x, y]
    ];
}

export default function CardInfo({ 
    x, y, 
    dx = 0, dy = 0, 
    paths = [], 
    className = "", 
    style = {},
    strokeColor = "black",
    detail
 } : CardInfoProps) {

    const [ pathData, viewbox, lastPosition ] = calculatePoints(paths);
    
    return <div
        style={{
            position: 'absolute',
            left: `${x - dx}px`,
            top: `${y - dy}px`,
            zIndex: 10
        }}
    >
        <svg className="absolute pointer-events-none"
            width={viewbox.width + 2 * strokeWidth} 
            height={viewbox.height + 2 * strokeWidth} 
            viewBox={`${viewbox.x - strokeWidth} ${viewbox.y - strokeWidth} ${viewbox.width + 2 * strokeWidth} ${viewbox.height + 2 * strokeWidth}`}
            style={{
                transform: `translate(${viewbox.x - strokeWidth}px, ${viewbox.y - strokeWidth}px)`
            }}
        >
            <circle
                cx={0}
                cy={0}
                fill="none"
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                r={circleRadius}
            />
            <path 
                d={pathData}
                fill="none"
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                strokeLinecap="round"
                strokeLinejoin="round"
            />
        </svg>
        <div className={className}
            style={{
                ...style,
                position: "absolute",
                left: lastPosition[0],
                top: lastPosition[1],
                transform: `translate(-50%, -50%)`
            }}
        >            
            {detail}
        </div>
    </div>
}