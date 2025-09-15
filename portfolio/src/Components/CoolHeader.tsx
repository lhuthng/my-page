interface CoolHeaderProps {
    title: string
}

export default function CoolHeader({
    title
}: CoolHeaderProps) {
    return (<div className="flex h-20 justify-center items-center">
        <p className="relative text-4xl w-full text-center font-bold text-white-chalk">
            {title}
            <span className="absolute text-7xl text-transparent tracking-widest opacity-10 text-nowrap left-1/2 top-1/3 -translate-1/2 select-none"
                style={{
                    WebkitTextStroke: "2px var(--color-white-chalk)",
                    filter: "drop-shadow(-1rem 0 0.4rem var(--color-white-chalk)) drop-shadow(1rem 0 0.4rem var(--color-white-chalk))"
                }}
            >
                {title}
            </span>
        </p>
    </div>)
}