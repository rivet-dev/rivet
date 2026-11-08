import { actor } from "rivetkit";

const mathUtils = actor({
  actions: {
    // This is an action
    multiplyByTwo: (c, x: number) => {
      return x * 2;
    }
  }
});
