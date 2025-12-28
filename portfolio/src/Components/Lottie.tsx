import { useLottie } from "lottie-react";
import { useEffect } from "react";

interface LottieProps {
  className: string;
  animationData?: object;
  loop?: boolean;
  speed?: number;
}

export default function Lottie({
  className = "",
  animationData,
  loop = true,
  speed = 1,
}: LottieProps) {
  const options = {
    animationData,
    loop,
    speed,
  };

  const { View, setSpeed } = useLottie(options);

  useEffect(() => {
    setSpeed(speed);
  }, [speed]);

  return <span className={className}>{View}</span>;
}
