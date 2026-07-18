var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandsBubbles.js
var require_faHandsBubbles = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandsBubbles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hands-bubbles";
    var width = 512;
    var height = 512;
    var aliases = ["hands-wash"];
    var unicode = "e05e";
    var svgPathData = "M384 64a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm96 128a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM128 464a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM0 160l.1 72.6c0 44.8 17.6 87.1 47.9 118.4 .3-77 40.1-148.4 105.4-189.3l1.9-1.2c-2.5-3.7-2.8-8.6-.4-12.7l62-107.4c6.6-11.5 2.7-26.2-8.8-32.8S182 5 175.4 16.5l-68 117.8 0 0 0 0-43.3 75 0-49.3c0-17.7-14.4-32-32-32S0 142.4 0 160zm213.6-35.9c14.1-8.8 29.8-12.7 45.2-12.1l36.5-63.3c6.6-11.5 2.7-26.2-8.8-32.8s-26.2-2.7-32.8 8.8l-67.1 116.2 27-16.9zm86.5-35.7l-16.4 28.4c15.1 5.5 28.6 15.6 38.3 29.7l19.7-34.2c6.6-11.5 2.7-26.2-8.8-32.8s-26.2-2.7-32.8 8.8zm46.4 63.7l-12.5 21.7c2 8.6 2.5 17.5 1.7 26.1l4.3 0 0 0 34.4 0 13.7-23.8c6.6-11.5 2.7-26.2-8.8-32.8s-26.2-2.7-32.8 8.8zM283.1 175c-9.4-15-29.1-19.5-44.1-10.2l-60.2 37.6C127.3 234.7 96 291.2 96 352l0 8c0 8.9 .8 17.6 2.2 26.1 35.4 8.2 61.8 40 61.8 77.9 0 6.3-.7 12.5-2.1 18.4 25.2 18.6 56.4 29.6 90.1 29.6l176 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-92 0c-6.6 0-12-5.4-12-12s5.4-12 12-12l124 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-124 0c-6.6 0-12-5.4-12-12s5.4-12 12-12l156 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-156 0c-6.6 0-12-5.4-12-12s5.4-12 12-12l124 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-136 0 0 0 0 0-93.2 0 46.2-28.9c15-9.4 19.5-29.1 10.2-44.1z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faHandsBubbles = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faHandsWash.js
var import_faHandsBubbles = __toESM(require_faHandsBubbles());
var export_faHandsWash = import_faHandsBubbles.definition;
export {
  export_faHandsWash as faHandsWash
};
