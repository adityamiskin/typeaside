import { useEffect, useState } from "react";
import { Dimensions, Keyboard, type KeyboardEvent, LayoutAnimation, Platform } from "react-native";

// How far the keyboard reaches up from the bottom of the screen. Edge-to-edge Android no longer
// resizes the window for it, so both platforms inset by hand.
export function useKeyboardHeight() {
  const [height, setHeight] = useState(0);
  useEffect(() => {
    const ios = Platform.OS === "ios";
    function change(next: number, event: KeyboardEvent) {
      if (ios && event.duration)
        LayoutAnimation.configureNext({
          duration: event.duration,
          update: { duration: event.duration, type: LayoutAnimation.Types.keyboard },
        });
      setHeight(next);
    }
    const subscriptions = [
      Keyboard.addListener(ios ? "keyboardWillChangeFrame" : "keyboardDidShow", (event) =>
        change(Math.max(0, Dimensions.get("screen").height - event.endCoordinates.screenY), event),
      ),
      Keyboard.addListener(ios ? "keyboardWillHide" : "keyboardDidHide", (event) =>
        change(0, event),
      ),
    ];
    return () => subscriptions.forEach((subscription) => subscription.remove());
  }, []);
  return height;
}
