import { useLottie } from "lottie-react";

interface AnimatedEmojiProps {
    className: string;
    animationData?: object;
    loop: boolean;
}

export default function AnimatedEmoji({
    className,
    animationData,
    loop
}: AnimatedEmojiProps) {
    const options = {
        animationData,
        loop
    };

    const { View } = useLottie(options);

    return <span className={className}>{View}</span>
}