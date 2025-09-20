interface UserSVGProps {
    className?: string
    style?: React.CSSProperties | undefined
    stroke?: string
    strokeWidth?: number
    fill?: string
    onClick?: () => void
}


export default function UserSVG({
    className = "",
    style,
    stroke = "none",
    strokeWidth = 1,
    fill = "none",
    onClick
} : UserSVGProps) {
    return <svg className={className} 
        style={{
            ...style,
        }}
        xmlns="http://www.w3.org/2000/svg" viewBox={`${-strokeWidth} ${-strokeWidth} ${100 + strokeWidth * 2} ${100 + strokeWidth * 2}`}
        onClick={onClick}
    >
        <path d="M100,99.98l-98.97.02-1.03-5.93,8.64-.02-5.94-31.48s48.2-1.91,50.57,0c2.05,1.65,8.94,31.49,8.94,31.49l3.93-.04c.09-10.95,20.9.91,21.83.03-.64-12.64-23.09-8.55-23.09-8.55,0,0-5.82-23.51-8.54-25.57s-17.93-1.3-17.93-1.3l7.21-9.61c8.83,4.97,12.77,6.13,18.64,6.03,6.29-.11,18.63-6.03,18.63-6.03,0,0,5.5,1.48,10.69,7.77,6.76,8.19,6.42,43.19,6.42,43.19Z"
            style={{
                stroke,
                strokeWidth,
                fill
            }}
        />
        <path d="M79.36,6.18c5.71,5.71,5.82,18.25,4.53,25.78-3.7,21.7-44.14,23.36-39.59-13.72,2.09-17.05,22.64-24.47,35.05-12.06Z"
            style={{
                stroke,
                strokeWidth,
                fill
            }}
        />
    </svg>
}