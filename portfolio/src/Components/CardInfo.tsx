import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react"

export interface CardInfoProps {
    x: number,
    y: number,
    dx?: number,
    dy?: number,
    compact?: boolean,
    onClick?: (toggle: boolean) => void,
    shown?: boolean,
    hoverSize?: [number, number],
    hoverDir?: [number, number, number],
    paths?: [number, number][],
    strokeColor?: string,
    className?: string,
    style?: React.CSSProperties,
    detail: ReactNode
}

const strokeWidth = 4;
const circleRadius = 6;
const pingRadius = circleRadius + 10;

function calculatePoints(paths: [number, number][]): [
    pathData: string, 
    viewbox: { x: number, y: number, width: number, height: number },
    lastPosition: [number, number]
] {
    let minX = -pingRadius, minY = -pingRadius, maxX = pingRadius, maxY = pingRadius;

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
        {
            x: minX, 
            y: minY, 
            width: maxX - minX, 
            height: maxY - minY
        },
        [x, y]
    ];
}

export default function CardInfo({ 
    x, y, 
    dx = 0, dy = 0, 
    paths = [], 
    compact,
    className = "", 
    style = {},
    shown,
    onClick,
    hoverDir,
    hoverSize,
    strokeColor = "black",
    detail
 } : CardInfoProps) {

    const [ pathData, viewbox, _lastPosition ] = calculatePoints(paths);
    const [ pivot, setPivot ] = useState<[number, number]>([x - dx, y - dy]);
    const defaultZIndex = 10;
    const [ zIndex, setZIndex ] = useState(defaultZIndex);
    const detailRef = useRef<HTMLDivElement>(null);

    const [ lastPosition, setLastPosition ] = useState(_lastPosition);

    useLayoutEffect(() => {
        if (!hoverDir) {
            setPivot([x - dx, y - dy]);
        }
        else if (hoverSize) {
            setPivot([x - dx + hoverDir[0] / 40, (y - dy + (hoverDir[1] - hoverDir[2]) / 40)]);
        }
    }, [ hoverDir, x, y, dx, dy ]);

    useGSAP(() => {
        if (compact) {
            setLastPosition([0, 0]);
            if (detailRef.current) {
                gsap.to(detailRef.current, {
                    opacity: 0,
                    duration: 0
                });
            }
        }
        else {
            setLastPosition(_lastPosition);
            if (detailRef.current) {
                gsap.to(detailRef.current, {
                    opacity: 1,
                    duration: 0
                });
            }
        }
    }, [ compact ]);

    useGSAP(() => {
        if (!compact || !detailRef.current) {
            setZIndex(defaultZIndex)
            return;
        }

        gsap.to(detailRef.current, {
            opacity: shown ? 1 : 0,
            duration: 0.1
        });
        setZIndex(defaultZIndex + (shown ? 1 : 0));
    }, [ shown, compact ]);
    

    return <div
        style={{
            position: 'absolute',
            left: `${pivot[0]}px`,
            top: `${pivot[1]}px`,
            zIndex,
        }}
    >
        <svg className="absolute"
            xmlns="http://www.w3.org/2000/svg"
            width={viewbox.width + strokeWidth} 
            height={viewbox.height + strokeWidth} 
            viewBox={`${viewbox.x - strokeWidth / 2} ${viewbox.y - strokeWidth / 2} ${viewbox.width + strokeWidth} ${viewbox.height + strokeWidth}`}
            style={{
                transform: `translate(${viewbox.x - strokeWidth / 2}px, ${viewbox.y - strokeWidth / 2}px)`,
                pointerEvents: "none",
            }}
        >
            {compact && <circle className="animate-ping"
                cx={0}
                cy={0}
                fill={strokeColor}
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                r={circleRadius}
            />}
            <circle className="hover:scale-120"
                style={{
                    pointerEvents: compact ? "auto" : "none",
                    cursor: compact ? "pointer" : "none",
                }}
                onClick={() => compact && onClick?.(true)}
                cx={0}
                cy={0}
                fillOpacity={0}
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                r={circleRadius}
            />
            {!compact && <path
                d={pathData}
                fill="none"
                stroke={strokeColor}
                strokeWidth={strokeWidth}
                strokeLinecap="round"
                strokeLinejoin="round"
            />}
        </svg>
        <div className={className + " not-lg:[&>div]:text-center"}
            ref={detailRef}
            style={{
                ...style,
                position: "absolute",
                left: lastPosition[0],
                top: lastPosition[1],
                transform: `translate(-50%, -50%)`,
                pointerEvents: shown ? "all" : "none",
                cursor: shown ? "pointer" : "none",
            }}
            onClick={() => compact && onClick?.(false)}
        >            
            {detail}
        </div>
    </div>
}