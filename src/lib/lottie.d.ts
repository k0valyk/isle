declare module "lottie-web/build/player/esm/lottie_light.min.js" {
  export interface LottieAnimation {
    destroy: () => void;
    goToAndStop: (value: number, isFrame?: boolean) => void;
    play: () => void;
    pause: () => void;
  }

  export interface LottiePlayer {
    loadAnimation: (params: {
      container: Element;
      renderer?: "svg" | "canvas" | "html";
      loop?: boolean;
      autoplay?: boolean;
      animationData: unknown;
    }) => LottieAnimation;
  }

  const lottie: LottiePlayer;
  export default lottie;
}
