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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceHandOverMouth.js
var require_faFaceHandOverMouth = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceHandOverMouth.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-hand-over-mouth";
    var width = 512;
    var height = 512;
    var aliases = [129762];
    var unicode = "e378";
    var svgPathData = "M512 256c0 113.1-73.3 209.1-175.1 242.9l49-49c18.1-18.1 18.7-47.1 1.8-65.9c16.9-18.8 16.3-47.8-1.8-65.9c-1-1-2.1-2-3.2-3c3.7-15.7-.5-32.8-12.8-45s-29.4-16.5-45-12.8c-.9-1.1-1.9-2.2-3-3.2c-18.7-18.7-49.1-18.7-67.9 0L207 301.1c-9.4-13.7-25.1-22.7-43-22.7c-28.7 0-52 23.3-52 52l0 53.6 0 32.8c0 21.9 5.5 42.5 15.3 60.5C51.2 433 0 350.5 0 256C0 114.6 114.6 0 256 0S512 114.6 512 256zM208.4 192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm-37.1 52.7c6.2 6.2 6.2 16.4 0 22.6L248 350.6c-2.6 2.6-2.6 6.8 0 9.4s6.8 2.6 9.4 0l67.3-67.3c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6L280 382.6c-2.6 2.6-2.6 6.8 0 9.4s6.8 2.6 9.4 0l51.3-51.3c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6L312 414.6c-2.6 2.6-2.6 6.8 0 9.4s6.8 2.6 9.4 0l19.3-19.3c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6l-56.8 56.8C288.7 502 264.4 512 239.2 512c-52.6 0-95.2-42.6-95.2-95.2l0-32.8 0-53.6c0-11 9-20 20-20s20 9 20 20l0 19.7c0 7.1 8.6 10.7 13.7 5.7l79-79c6.2-6.2 16.4-6.2 22.6 0z";
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
    exports.faFaceHandOverMouth = exports.definition;
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

// src/_entries/faFaceHandOverMouth.js
var import_faFaceHandOverMouth = __toESM(require_faFaceHandOverMouth());
var export_faFaceHandOverMouth = import_faFaceHandOverMouth.definition;
export {
  export_faFaceHandOverMouth as faFaceHandOverMouth
};
