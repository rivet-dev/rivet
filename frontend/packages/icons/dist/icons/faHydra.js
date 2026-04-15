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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHydra.js
var require_faHydra = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHydra.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hydra";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e686";
    var svgPathData = "M256 73.1C255.8 32.7 223 0 182.5 0c-16.7 0-32.9 5.7-45.9 16.1L108 39C97.4 47.5 93.3 61.8 97.8 74.6S114.4 96 128 96l48 0s0 0 0 0c8.8 0 16 7.2 16 16l0 16s0 0 0 0l0 .8-.1 .8-11.5 115c-2.4 23.7 6 47.3 22.9 64.1c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0c-23.6-23.6-35.4-56.7-32.1-89.9l4.5-45.2c-14.2-29.5-48.3-46.8-82.5-37c-8.9 2.6-17.3 6.9-24.6 12.7L12 199C1.4 207.5-2.7 221.8 1.8 234.6S18.4 256 32 256l48 0c8.8 0 16 7.2 16 16l0 16 0 24.8c0 61 40.4 112.6 96 129.4l0 37.8c0 17.7 14.3 32 32 32l36.9 0c15.6 0 29-11.3 31.6-26.7l6.2-37.3 74.7 0 6.2 37.3c2.6 15.4 15.9 26.7 31.6 26.7l36.9 0c17.7 0 32-14.3 32-32l0-32 32 0c70.7 0 128-57.3 128-128c0-40.3-19-78.2-51.2-102.4l-25-18.8c-1.8-1.4-3.7-2.6-5.7-3.6c-4.3-2.1-8.9-3.2-13.5-3.2s-9.3 .8-13.6 2.8c-3.3 1.5-6.4 3.6-9.1 6.2c-2.7 2.6-4.9 5.7-6.5 9c-2.1 4.3-3.2 8.9-3.2 13.5s.8 9.3 2.8 13.6c.9 2 2.1 4 3.4 5.8l32.6 46.6c18.4 26.3-.4 62.4-32.5 62.4c-13.3 0-25.6-6.6-33-17.7l-10.7-16.1C448.7 279.4 405.1 256 358.4 256l-.1 0-9.2 0-5.3 0c-47.8 0-87-38.4-87.8-85.3c0-.5 0-.9 0-1.4l0-.6 0-4.7c0-.4 0-.8 0-1.2l0-34.9 0-16 0-27.8 0-11.1zm32 36.9l0 60.4c.5 25 18.8 46.7 43.3 52.3l4-24.1 6.6-26.4c1.8-7.1 8.2-12.1 15.5-12.1l58.5 0c13.6 0 25.7-8.6 30.2-21.4s.4-27.1-10.2-35.6L408.8 81.3C394.8 70.1 377.5 64 359.6 64c-31.4 0-59 18.5-71.6 45.9z";
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
    exports.faHydra = exports.definition;
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

// src/_entries/faHydra.js
var import_faHydra = __toESM(require_faHydra());
var export_faHydra = import_faHydra.definition;
export {
  export_faHydra as faHydra
};
