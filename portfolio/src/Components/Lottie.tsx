import { useLottie } from "lottie-react";

interface LottieProps {
    className: string;
    animationData?: object;
    loop: boolean;
}

export default function Lottie({
    className = "",
    animationData,
    loop
}: LottieProps) {
    const options = {
        animationData,
        loop
    };

    const { View } = useLottie(options);

    return <span className={className}>{View}</span>
}