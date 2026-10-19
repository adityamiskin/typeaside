import { useEffect, useState } from "react";
import { Dimensions, Keyboard, type KeyboardEvent, LayoutAnimation, Platform } from "react-native";

// Android resizes the window for the keyboard; only iOS needs an extra inset.
export function useKeyboardHeight() {
  const [height, setHeight] = useState(0);
  useEffect(() => {
    if (Platform.OS !== "ios") return;
    function change(next: number, event: KeyboardEvent) {
      if (event.duration)
        LayoutAnimation.configureNext({
          duration: event.duration,
          update: { duration: event.duration, type: LayoutAnimation.Types.keyboard },
        });
      setHeight(next);
    }
    const subscriptions = [
      Keyboard.addListener("keyboardWillChangeFrame", (event) =>
        change(Math.max(0, Dimensions.get("screen").height - event.endCoordinates.screenY), event),
      ),
      Keyboard.addListener("keyboardWillHide", (event) => change(0, event)),
    ];
    return () => subscriptions.forEach((subscription) => subscription.remove());
  }, []);
  return height;
}
