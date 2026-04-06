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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandsPraying.js
var require_faHandsPraying = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandsPraying.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hands-praying";
    var width = 640;
    var height = 512;
    var aliases = ["praying-hands"];
    var unicode = "f684";
    var svgPathData = "M224 296c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-112.6 88.2-119.7c13.1-17.8 9.3-42.8-8.5-55.9s-42.8-9.3-55.9 8.5L106.5 142.9C89.3 166.2 80 194.5 80 223.6l0 110.7-58.1 19.4C8.8 358 0 370.2 0 384l0 96c0 10 4.7 19.5 12.7 25.5s18.4 8 28.1 5.2l154.4-44.1c55-15.7 92.8-65.9 92.8-123.1L288 224c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 72zm192 0l0-72c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 119.6c0 57.2 37.9 107.4 92.8 123.1l154.4 44.1c9.7 2.8 20 .8 28.1-5.2S640 490 640 480l0-96c0-13.8-8.8-26-21.9-30.4l-58.1-19.4 0-110.7c0-29-9.3-57.3-26.5-80.7L440.2 16.3C427.1-1.5 402.1-5.3 384.3 7.8s-21.6 38.1-8.5 55.9L464 183.4 464 296c0 13.3-10.7 24-24 24s-24-10.7-24-24z";
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
    exports.faHandsPraying = exports.definition;
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

// src/_entries/faPrayingHands.js
var import_faHandsPraying = __toESM(require_faHandsPraying());
var export_faPrayingHands = import_faHandsPraying.definition;
export {
  export_faPrayingHands as faPrayingHands
};
